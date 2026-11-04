//! Record one effect where it lands, or refuse it.
//!
//! This is the guarantee, and [`crate::Claim`] is the optimisation: a claim
//! refuses work whose authority had already ended, but it ran before the effect
//! did anything and cannot say anything about the instant it writes. Only the
//! place the record is presented to can, so the check is here.
//!
//! The checks are asked in the order that makes the answer the true one:
//!
//! 1. **Is this the latest generation?** A record under a superseded one is not
//!    this database's record.
//! 2. **Is there a lease row?** No row means nothing was admitted for this
//!    resource, so nothing can be attributed to it.
//! 3. **Was the lease granted under this generation?** A lease never adopts a
//!    later one.
//! 4. **Is this the current fence?** A superseded epoch is refused, and the
//!    claim that superseded it is the accounting point for the effects it had
//!    in flight - the row no longer describes that epoch, so this rule cannot
//!    attribute them.
//! 5. **Is the caller the holder?** `lease.holder == ctx.sender`. This is the
//!    check a caller cannot pass by reproducing numbers: the transport
//!    authenticates the sender, so the sender is never an argument.
//!
//! Only after all five does a refusal become *ambiguous*, because only then is
//! the effect one this database admitted. An effect that was admitted and
//! refused anyway may have changed the world, so the refusal is written as an
//! `effect-ambiguous` trace row and the lease is quarantined: the write is not
//! counted as a success, and [`crate::Claim`] will not hand the resource to
//! anybody else until [`crate::Resolve`] says what happened.

use crate::{
	Fn::{Fenced::IsLive, Json::Quoted, Latest::Latest},
	Struct::{
		Answer::Answer,
		Code::Code,
		Context::Context,
		Effect::Effect,
		Row::{Row, Value},
		Table::TableName,
	},
};

/// Record one effect, or refuse it and record that it is ambiguous.
pub fn Record<C:Context>(Ctx:&mut C, Args:Effect) -> Answer {
	if Args.Name.is_empty() {
		return Answer::Refused(Code::AdmissionRefused);
	}
	if Latest(Ctx, &Args.Generation).is_none() {
		return Answer::Refused(Code::AdmissionRefused);
	}
	let Some(Lease) = Ctx.Find(TableName::Lease, &Args.Resource) else {
		return Answer::Refused(Code::AdmissionRefused);
	};
	if Lease.Text("generation") != Some(Args.Generation.as_str()) {
		return Answer::Refused(Code::AdmissionRefused);
	}
	if Lease.Number("fence") != Some(Args.Fence) {
		return Answer::Refused(Code::AdmissionRefused);
	}
	if Lease.Text("holder") != Some(Ctx.Sender()) {
		return Answer::Refused(Code::AdmissionRefused);
	}

	// From here the record is one this database issued to this caller, so a
	// refusal means an authorised effect may have landed.
	if Lease.Text("state") == Some("quarantined") {
		return Quarantine(Ctx, &Lease, &Args, "already quarantined by an earlier effect");
	}
	if Lease.Text("state") == Some("released") {
		return Quarantine(Ctx, &Lease, &Args, "released while the effect was in flight");
	}
	if !IsLive(&Lease, Ctx.Now()) {
		return Quarantine(Ctx, &Lease, &Args, "outlived its authority");
	}

	let Now = Ctx.Now();
	let Sender = Ctx.Sender().to_string();
	Ctx.Insert(
		TableName::Trace,
		Row::Of([
			(
				"id",
				Value::Text(format!("{Now}:effect:{}#{}#{}", Args.Resource, Args.Fence, Args.Name)),
			),
			("at", Value::Number(Now)),
			("kind", Value::Text("effect".to_string())),
			("subject", Value::Text(Args.Resource)),
			(
				"payload",
				Value::Text(format!("{{\"name\":{},\"fence\":{}}}", Quoted(&Args.Name), Args.Fence)),
			),
			("generation", Value::Text(Args.Generation)),
			("sender", Value::Text(Sender)),
		]),
	);
	Answer::Accepted
}

/// Write a refusal as an ambiguity and quarantine the resource.
///
/// The row is a trace rather than a new table because the catalog's six tables
/// are what a module renders, and evidence of a refused write is what `trace`
/// is for: it carries no fence, so the write worth keeping - the one that was
/// refused - is not dropped. The quarantine is the lease row's `state`, so "may
/// this resource be reassigned" stays one key lookup, which is the only kind of
/// question this context can answer.
fn Quarantine<C:Context>(Ctx:&mut C, Lease:&Row, Args:&Effect, Reason:&str) -> Answer {
	let Now = Ctx.Now();
	let Sender = Ctx.Sender().to_string();
	Ctx.Insert(
		TableName::Trace,
		Row::Of([
			(
				"id",
				Value::Text(format!("{Now}:effect-ambiguous:{}#{}#{}", Args.Resource, Args.Fence, Args.Name)),
			),
			("at", Value::Number(Now)),
			("kind", Value::Text("effect-ambiguous".to_string())),
			("subject", Value::Text(Args.Resource.clone())),
			(
				"payload",
				Value::Text(format!(
					"{{\"name\":{},\"fence\":{},\"reason\":{}}}",
					Quoted(&Args.Name),
					Args.Fence,
					Quoted(Reason)
				)),
			),
			("generation", Value::Text(Args.Generation.clone())),
			("sender", Value::Text(Sender)),
		]),
	);
	if Lease.Text("state") != Some("quarantined") {
		Ctx.Update(
			TableName::Lease,
			Lease.clone().With("state", Value::Text("quarantined".to_string())),
		);
	}
	Answer::Refused(Code::AdmissionRefused)
}

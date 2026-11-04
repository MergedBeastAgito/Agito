//! State what happened to the effects that quarantined a resource, and release
//! it for reassignment.
//!
//! The lease row is moved to `released` rather than deleted, so the fence
//! survives and the next claim advances past it. The ambiguity rows stay and
//! the answer is appended as a third row: a resolution that erased the question
//! would be the retraction the handover rule forbids.
//!
//! The sender is recorded, not checked. Who may resolve is a role, and a role
//! is a deployment's decision; the rule's job is that the answer is
//! attributable.

use crate::{
	Fn::{Json::Quoted, Latest::Latest},
	Struct::{
		Answer::Answer,
		Code::Code,
		Context::Context,
		Resolve::Resolve as ResolveArgs,
		Row::{Row, Value},
		Table::TableName,
	},
};

/// Answer what happened to a quarantined resource's effects, and release it.
pub fn Resolve<C:Context>(Ctx:&mut C, Args:ResolveArgs) -> Answer {
	let ResolveArgs { Resource, Generation, Outcome } = Args;
	if Latest(Ctx, &Generation).is_none() {
		return Answer::Refused(Code::AdmissionRefused);
	}
	let Some(Lease) = Ctx.Find(TableName::Lease, &Resource) else {
		return Answer::Refused(Code::AdmissionRefused);
	};
	if Lease.Text("state") != Some("quarantined") {
		return Answer::Refused(Code::AdmissionRefused);
	}
	let Fence = Lease.Number("fence");
	Ctx.Update(TableName::Lease, Lease.With("state", Value::Text("released".to_string())));
	let Now = Ctx.Now();
	let Sender = Ctx.Sender().to_string();
	Ctx.Insert(
		TableName::Trace,
		Row::Of([
			("id", Value::Text(format!("{Now}:effect-resolved:{Resource}"))),
			("at", Value::Number(Now)),
			("kind", Value::Text("effect-resolved".to_string())),
			("subject", Value::Text(Resource)),
			(
				"payload",
				Value::Text(format!(
					"{{\"outcome\":{},\"fence\":{}}}",
					Quoted(Outcome.Name()),
					Fence.map_or_else(|| "null".to_string(), |Fence| Fence.to_string())
				)),
			),
			("generation", Value::Text(Generation)),
			("sender", Value::Text(Sender)),
		]),
	);
	Answer::Accepted
}

//! Reveal one property version to the caller, and write the observation that is
//! the reveal's own receipt.
//!
//! The row id is `<property>:<version>:<audience>`, so the receipt *is* the
//! allowance: a second probe by the same audience at the same version finds the
//! row and is refused as `budget-exhausted`. That is why no second row records
//! a charge, and why two racing probes cannot both spend a one-use allowance -
//! the database serialises them, and the loser finds the winner's row.
//!
//! The audience is the context's sender. It is not an argument: a
//! caller-supplied audience would be a caller-supplied identity, and the
//! recipient of a reveal is never something the caller names.

use crate::{
	Fn::Fenced::Fenced,
	Struct::{
		Answer::ProbeAnswer,
		Code::Code,
		Context::Context,
		Probe::Probe as ProbeArgs,
		Row::{Row, Value},
		Table::TableName,
	},
};

/// Reveal one property version, or refuse without writing.
pub fn Probe<C:Context>(Ctx:&mut C, Args:ProbeArgs) -> ProbeAnswer {
	let ProbeArgs { Generation, Resource, Fence, Property, Version, Observed } = Args;
	if Property.is_empty() {
		return ProbeAnswer::Refused(Code::AdmissionRefused);
	}
	if Version < 1 {
		return ProbeAnswer::Refused(Code::AdmissionRefused);
	}
	if Fenced(Ctx, &Generation, &Resource, Fence).is_none() {
		return ProbeAnswer::Refused(Code::AdmissionRefused);
	}
	let Sender = Ctx.Sender().to_string();
	let ID = format!("{Property}:{Version}:{Sender}");
	if Ctx.Find(TableName::Probe, &ID).is_some() {
		return ProbeAnswer::Refused(Code::BudgetExhausted);
	}
	let Now = Ctx.Now();
	let Observation = Row::Of([
		("id", Value::Text(ID)),
		("property", Value::Text(Property)),
		("version", Value::Number(Version)),
		("audience", Value::Text(Sender)),
		("observed", Value::Text(Observed)),
		("at", Value::Number(Now)),
		("generation", Value::Text(Generation)),
	]);
	Ctx.Insert(TableName::Probe, Observation.clone());
	ProbeAnswer::Observed(Observation)
}

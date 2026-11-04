//! Bind a generation: append it, and refuse a manifest that names a capability
//! the catalog does not declare.
//!
//! A bind names its predecessor, and the predecessor must still be the latest -
//! which is what makes two concurrent binds a compare-and-swap rather than a
//! fork. In a serialised database the second one sees the first one's successor
//! and is refused.
//!
//! One insert, no update: the previous generation is untouched, and the catalog
//! marks the table append-only so a context cannot quietly rewrite it.

use crate::{
	Fn::{Json::Array, Latest::Latest},
	Struct::{
		Answer::Answer,
		Bind::Bind as BindArgs,
		Code::Code,
		Context::Context,
		Row::{Row, Value},
		Table::TableName,
	},
};

/// Bind a generation.
pub fn Bind<C:Context>(Ctx:&mut C, Args:BindArgs) -> Answer {
	let BindArgs { Previous, Manifest } = Args;
	if Manifest.Digest.is_empty() {
		return Answer::Refused(Code::AdmissionRefused);
	}
	let mut Named = Manifest.Capabilities.clone();
	let Declared = Named.len();
	Named.sort();
	Named.dedup();
	if Named.len() != Declared {
		return Answer::Refused(Code::AdmissionRefused);
	}
	for Id in &Named {
		if Ctx.Find(TableName::Capability, Id).is_none() {
			return Answer::Refused(Code::AdmissionRefused);
		}
	}
	let Sequence = match &Previous {
		None => {
			// The first generation is always `1`, so "has anything been bound?"
			// is one lookup rather than a search for the maximum.
			if Ctx.Find(TableName::Generation, "1").is_some() {
				return Answer::Refused(Code::AdmissionRefused);
			}
			1
		},
		Some(Previous) => {
			let Some(Record) = Ctx.Find(TableName::Generation, Previous) else {
				return Answer::Refused(Code::AdmissionRefused);
			};
			if Latest(Ctx, Previous).is_none() {
				return Answer::Refused(Code::AdmissionRefused);
			}
			let Some(Parent) = Record.Number("sequence") else {
				return Answer::Refused(Code::AdmissionRefused);
			};
			let Some(Next) = Parent.checked_add(1) else {
				return Answer::Refused(Code::AdmissionRefused);
			};
			Next
		},
	};
	let Id = Sequence.to_string();
	if Ctx.Find(TableName::Generation, &Id).is_some() {
		return Answer::Refused(Code::AdmissionRefused);
	}
	let Now = Ctx.Now();
	let Sender = Ctx.Sender().to_string();
	Ctx.Insert(
		TableName::Generation,
		Row::Of([
			("id", Value::Text(Id)),
			("sequence", Value::Number(Sequence)),
			("parent", Previous.map_or(Value::Null, Value::Text)),
			("manifestDigest", Value::Text(Manifest.Digest)),
			("capabilities", Value::Text(Array(&Named))),
			("boundAt", Value::Number(Now)),
			("boundBy", Value::Text(Sender)),
		]),
	);
	Answer::Accepted
}

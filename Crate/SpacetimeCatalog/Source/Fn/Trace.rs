//! Append one event, keyed by its instant, kind and subject.
//!
//! A trace is evidence, not an authoritative effect, so it carries no fence:
//! the write worth keeping is often the one that was refused, and fencing the
//! log would drop exactly that. It does carry the generation, because an event
//! read under the wrong interpreter is a different event.
//!
//! A replay at the same instant appends nothing; a different payload under the
//! same key is refused rather than silently replacing the first, because
//! reusing a content key with different content is how a log starts lying.

use crate::{
	Fn::Latest::Latest,
	Struct::{
		Answer::Answer,
		Code::Code,
		Context::Context,
		Row::{Row, Value},
		Table::TableName,
		Trace::Trace as TraceArgs,
	},
};

/// Append one event.
pub fn RecordTrace<C:Context>(Ctx:&mut C, Args:TraceArgs) -> Answer {
	let TraceArgs { Generation, Kind, Subject, Payload } = Args;
	if Latest(Ctx, &Generation).is_none() {
		return Answer::Refused(Code::AdmissionRefused);
	}
	if Kind.is_empty() || Subject.is_empty() {
		return Answer::Refused(Code::AdmissionRefused);
	}
	let Now = Ctx.Now();
	let Id = format!("{Now}:{Kind}:{Subject}");
	if let Some(Existing) = Ctx.Find(TableName::Trace, &Id) {
		return if Existing.Text("payload") == Some(Payload.as_str()) {
			Answer::Accepted
		} else {
			Answer::Refused(Code::AdmissionRefused)
		};
	}
	let Sender = Ctx.Sender().to_string();
	Ctx.Insert(
		TableName::Trace,
		Row::Of([
			("id", Value::Text(Id)),
			("at", Value::Number(Now)),
			("kind", Value::Text(Kind)),
			("subject", Value::Text(Subject)),
			("payload", Value::Text(Payload)),
			("generation", Value::Text(Generation)),
			("sender", Value::Text(Sender)),
		]),
	);
	Answer::Accepted
}

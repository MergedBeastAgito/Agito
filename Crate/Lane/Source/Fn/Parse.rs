//! Reading a reply, and refusing anything that is not an answer.
//!
//! A decision model's reply is one `result` holding an `answers` object, one
//! entry per question asked, and the spend beside it. Three things about this
//! parser are deliberate:
//!
//! - **A failed call is not an answer with empty questions.** `success: false`
//!   is refused, and the endpoint's own message goes into the detail, because
//!   the commonest failure (an expired credential) arrives as a 200-shaped body
//!   with `success: false` in it and reads as "the model said nothing".
//! - **The distribution is kept.** A choice's probabilities are carried whole,
//!   so a caller can tell a committed answer from a model spreading its mass
//!   across five options. Re-normalising the winner would erase that.
//! - **A question type this crate does not interpret is carried, not dropped.**
//!   The endpoint may understand it; this crate simply cannot turn it into a
//!   probability, and pretending otherwise would be a typed answer that is not
//!   one.
//!
//! The parser takes a body and nothing else, so a captured reply is the thing
//! the suite tests - not a reply this crate built for itself.

use std::collections::BTreeMap;

use serde_json::Value;

use crate::Struct::{
	Answer::{Answer, Answered, Usage},
	Error::Error,
};

/// Read one reply.
pub fn Parse(Body:&str) -> Result<Answer, Error> {
	let Parsed:Value = serde_json::from_str(Body)
		.map_err(|Why| Error::Unparseable("the reply is not JSON", format!("{Why} · {}", Clip(Body))))?;
	if Parsed.get("success").and_then(Value::as_bool) == Some(false) {
		return Err(Error::Unparseable("the endpoint refused the request", Complaint(&Parsed)));
	}
	let Result = Parsed
		.get("result")
		.filter(|Result| !Result.is_null())
		.ok_or_else(|| Error::Unparseable("the reply carries no result", Clip(Body)))?;
	let Answers = Result
		.get("answers")
		.and_then(Value::as_object)
		.ok_or_else(|| Error::Unparseable("the reply carries no answers", Clip(Body)))?;
	if Answers.is_empty() {
		return Err(Error::Unparseable("the reply answered no question", Clip(Body)));
	}

	let Questions:BTreeMap<String, Answered> = Answers.iter().map(|(Name, One)| (Name.clone(), Read(One))).collect();

	// The spend sits beside `answers` inside `result`; a top-level `usage` is
	// read too, because a proxy in front of this endpoint may hoist it.
	let Usage = Result.get("usage").or_else(|| Parsed.get("usage"));
	let Input = Usage
		.and_then(|Usage| Usage.get("input_tokens"))
		.and_then(Value::as_u64)
		.unwrap_or(0);
	let Output = Usage
		.and_then(|Usage| Usage.get("output_tokens"))
		.and_then(Value::as_u64)
		.unwrap_or(0);
	let Model = Result.get("model").and_then(Value::as_str).unwrap_or_default().to_string();
	Ok(Answer { Model, Questions, Usage:Usage { Input, Output } })
}

/// One question's answer.
///
/// A choice is recognised by its `probabilities` rather than by its `type`,
/// because the distribution is what makes it a choice: an answer with a
/// `choice` word and no probabilities is a word this crate cannot threshold.
fn Read(One:&Value) -> Answered {
	if let Some(Probabilities) = One.get("probabilities").and_then(Value::as_object) {
		if let Some(Choice) = One.get("choice").and_then(Value::as_str) {
			return Answered::Choice {
				Choice:Choice.to_string(),
				Probabilities:Probabilities
					.iter()
					.filter_map(|(Option, Share)| Share.as_f64().map(|Share| (Option.clone(), Share)))
					.collect(),
			};
		}
	}
	if let Some(Score) = One.get("score").and_then(Value::as_f64) {
		return Answered::Score { Score, Legend:Legend(One.get("legend")) };
	}
	Answered::Other(One.clone())
}

/// A score's legend, which the endpoint may write as an array of words or as an
/// object keyed by the position.
///
/// Both spellings were measured on this account, so both are read rather than
/// one being declared correct: an object keyed `"0".."3"` is sorted numerically
/// rather than lexically, because `"10"` sorting before `"2"` would put a
/// long scale's words in the wrong order.
fn Legend(Value:Option<&Value>) -> Vec<String> {
	match Value {
		Some(Value::Array(Words)) => Words.iter().filter_map(Value::as_str).map(str::to_string).collect(),
		Some(Value::Object(Words)) => {
			let mut Keyed:Vec<(i64, String)> = Words
				.iter()
				.filter_map(|(Key, Word)| Some((Key.parse::<i64>().ok()?, Word.as_str()?.to_string())))
				.collect();
			Keyed.sort_by_key(|(Position, _)| *Position);
			Keyed.into_iter().map(|(_, Word)| Word).collect()
		},
		_ => Vec::new(),
	}
}

/// What the endpoint said about a refusal.
fn Complaint(Parsed:&Value) -> String {
	Parsed
		.get("errors")
		.and_then(Value::as_array)
		.and_then(|Errors| Errors.first())
		.map(|One| {
			let Code = One.get("code").map(Value::to_string).unwrap_or_default();
			let Message = One.get("message").and_then(Value::as_str).unwrap_or("no message");
			format!("{Message} (code {Code})")
		})
		.unwrap_or_else(|| "the endpoint reported a failure and said nothing else".to_string())
}

/// A body, shortened to something a log can hold.
///
/// A detail is read by a person and pasted into a report, so an unbounded body
/// would be a detail nobody reads. Nothing secret is in a reply body: the token
/// travels in a header, and this crate never sees it.
fn Clip(Body:&str) -> String {
	let Trimmed = Body.trim();
	if Trimmed.chars().count() <= 240 {
		return Trimmed.to_string();
	}
	let Shortened:String = Trimmed.chars().take(240).collect();
	format!("{Shortened}…")
}

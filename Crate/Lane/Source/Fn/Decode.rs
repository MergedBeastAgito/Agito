//! Reading a reply into typed answers, and saying exactly what is wrong when it
//! will not read.
//!
//! A decision model's reply is one `result` holding an `answers` object - one
//! entry per question asked - and the spend beside it. Three things about this
//! decoder are deliberate:
//!
//! - **A failed call is not an answer with empty questions.** `success: false`
//!   is `DecodeError::Refused`, and the endpoint's own message goes into the
//!   detail, because the commonest failure (an expired credential) arrives as a
//!   200-shaped body with `success: false` in it and reads as "the model said
//!   nothing".
//! - **The distribution is kept.** A choice's probabilities are carried whole,
//!   so a caller can tell a committed answer from a model spreading its mass
//!   across five options. Re-normalising the winner would erase that, and the
//!   chosen option is checked against its own distribution rather than trusted:
//!   a `choice` naming an option the reply did not carry is
//!   `DecodeError::UnknownChoice`, not a choice with a probability of zero.
//! - **A question type this crate does not interpret is carried, not dropped.**
//!   The endpoint may understand it; this crate simply cannot turn it into a
//!   probability, and pretending otherwise would be a typed answer that is not
//!   one.
//!
//! # Three entry points, one reading
//!
//! [`Decode`] reads a body and nothing else, which is the shape a captured
//! reply has. [`DecodeWithin`] reads the same body against the caller's token
//! ceiling, so the answer carries what is *left* as well as what was spent -
//! a remaining computed anywhere else would be a second authority over the same
//! arithmetic. [`DecodeBytes`] is the same read for a transport that hands back
//! bytes rather than text; the crate's own [`crate::Struct::Port::Poster`]
//! returns a `String`, so the byte form exists for an adapter, not for the
//! port.
//!
//! A ceiling the reply spent past reports `Remaining: Some(0)` rather than a
//! negative number: the overrun is a fact for the meter to refuse the *next*
//! call with, and a `u64` that wrapped would be a bug wearing a number.
//!
//! # No request, no network
//!
//! The decoder takes a body and nothing else. It never sees a token, a status
//! code or a socket, which is what lets the suite test it against a captured
//! reply - and what keeps the crate's one dependency list honest.

use std::collections::BTreeMap;

use serde_json::{Map, Value};

use crate::Struct::{
	Answer::{Answer, Answered, Usage},
	Decode::DecodeError,
};

/// Read one reply with no ceiling declared.
///
/// The answer's [`Usage::Remaining`] is `None`: a remaining without a ceiling
/// would be a number this crate invented.
pub fn Decode(Body:&str) -> Result<Answer, DecodeError> { Read(Body, None) }

/// Read one reply against the caller's token ceiling.
///
/// The answer's [`Usage::Remaining`] is the ceiling less the tokens this reply
/// reported, floored at zero. The ceiling is not sent anywhere and not checked
/// before the call - it is the caller's meter
/// ([`crate::Struct::Budget::Budget`]) that refuses the next request - so this
/// is a report, not a policy.
pub fn DecodeWithin(Body:&str, Ceiling:u64) -> Result<Answer, DecodeError> { Read(Body, Some(Ceiling)) }

/// Read one reply from the bytes a transport handed back.
///
/// Bytes that are not UTF-8 are `DecodeError::Malformed` rather than a lossy
/// string: a body decoded with replacement characters is a body whose
/// probabilities may have been mangled, and an answer built from it would be
/// wrong in a way nothing downstream could see.
pub fn DecodeBytes(Body:&[u8]) -> Result<Answer, DecodeError> {
	let Text = std::str::from_utf8(Body).map_err(|Why| {
		DecodeError::Malformed(format!(
			"the reply is not UTF-8 ({Why}) · {}",
			Clip(&String::from_utf8_lossy(Body))
		))
	})?;
	Decode(Text)
}

/// The one reading, with or without a ceiling.
fn Read(Body:&str, Ceiling:Option<u64>) -> Result<Answer, DecodeError> {
	let Parsed:Value = serde_json::from_str(Body)
		.map_err(|Why| DecodeError::Malformed(format!("the reply is not JSON: {Why} · {}", Clip(Body))))?;
	let Parsed = Object(&Parsed, "the reply")?;
	if Parsed.get("success").and_then(Value::as_bool) == Some(false) {
		return Err(DecodeError::Refused(Complaint(Parsed)));
	}
	let Result = Parsed
		.get("result")
		.filter(|Result| !Result.is_null())
		.ok_or_else(|| DecodeError::Missing("result", format!("the reply carries no result · {}", Clip(Body))))?;
	let Result = Object(Result, "result")?;
	let Answers = Result.get("answers").ok_or_else(|| {
		DecodeError::Missing("result.answers", format!("the result carries no answers · {}", Clip(Body)))
	})?;
	let Answers = Object(Answers, "result.answers")?;
	if Answers.is_empty() {
		return Err(DecodeError::Empty(format!(
			"`result.answers` is an empty object · {}",
			Clip(Body)
		)));
	}

	let mut Questions:BTreeMap<String, Answered> = BTreeMap::new();
	for (Name, One) in Answers {
		Questions.insert(Name.clone(), OneAnswer(Name, One)?);
	}

	// The spend sits beside `answers` inside `result`; a top-level `usage` is
	// read too, because a proxy in front of this endpoint may hoist it. A
	// `null` in either place is not a reported spend, so the other one is still
	// consulted before the reply is called spendless.
	let Usage = Result
		.get("usage")
		.filter(|Usage| !Usage.is_null())
		.or_else(|| Parsed.get("usage").filter(|Usage| !Usage.is_null()));
	let Usage = Spend(Usage, Ceiling)?;
	let Model = Result
		.get("model")
		.or_else(|| Parsed.get("model"))
		.and_then(Value::as_str)
		.unwrap_or_default()
		.to_string();
	Ok(Answer { Model, Questions, Usage })
}

/// One question's answer.
///
/// A declared type this crate does not interpret - `noul` is one the endpoint
/// knows and this crate does not - is carried whole rather than read as a
/// choice that happens to be missing its distribution. Only an undeclared type
/// is recognised by shape, because then the shape is all there is: an answer
/// carrying `probabilities` is a distribution, and one carrying `score` is a
/// position on a scale.
fn OneAnswer(Name:&str, One:&Value) -> Result<Answered, DecodeError> {
	let One = One
		.as_object()
		.ok_or_else(|| DecodeError::Shape(Name.to_string(), format!("the answer is {}, not an object", Kind(One))))?;
	let Declared = One.get("type").and_then(Value::as_str).unwrap_or_default();

	match Declared {
		"choice" => return Choice(Name, One),
		"score" => return Position(Name, One),
		"" => {},
		// A type this crate cannot turn into a probability or a position is
		// kept as it arrived: dropping it would be an answer that is not one,
		// and refusing it would be this crate claiming a vocabulary it lacks.
		_ => return Ok(Answered::Other(Value::Object(One.clone()))),
	}
	if One.contains_key("probabilities") {
		return Choice(Name, One);
	}
	if One.contains_key("score") {
		return Position(Name, One);
	}
	Ok(Answered::Other(Value::Object(One.clone())))
}

/// A score: the value, and the legend it sits on.
fn Position(Name:&str, One:&Map<String, Value>) -> Result<Answered, DecodeError> {
	let Score = One.get("score").and_then(Value::as_f64).ok_or_else(|| {
		DecodeError::Shape(
			format!("{Name}.score"),
			format!("the score is {}, not a number", One.get("score").map(Kind).unwrap_or("absent")),
		)
	})?;
	Ok(Answered::Score { Score, Legend:Legend(One.get("legend")) })
}

/// A choice: the option named, and the distribution it was named from.
fn Choice(Name:&str, One:&Map<String, Value>) -> Result<Answered, DecodeError> {
	let Shares = One.get("probabilities").ok_or_else(|| {
		DecodeError::Missing(
			format!("{Name}.probabilities"),
			"a choice without its distribution cannot be told apart from a name this crate should not trust"
				.to_string(),
		)
	})?;
	let Shares = Shares.as_object().ok_or_else(|| {
		DecodeError::ProbabilityShape(
			Name.to_string(),
			format!("`probabilities` is {}, not an object of options", Kind(Shares)),
		)
	})?;
	if Shares.is_empty() {
		return Err(DecodeError::ProbabilityShape(
			Name.to_string(),
			"`probabilities` is an empty object, so no option could have been chosen".to_string(),
		));
	}

	let mut Probabilities:BTreeMap<String, f64> = BTreeMap::new();
	for (Option, Share) in Shares {
		let Share = Share.as_f64().ok_or_else(|| {
			DecodeError::ProbabilityShape(
				Name.to_string(),
				format!("option `{Option}` has {}, not a number", Kind(Share)),
			)
		})?;
		// A share is a share. A negative one or one above certainty is not a
		// rounding artefact - it is a body this crate has no reading for, and
		// accepting it would put a number outside 0..1 in a caller's hands.
		if !(0.0..=1.0).contains(&Share) {
			return Err(DecodeError::ProbabilityShape(
				Name.to_string(),
				format!("option `{Option}` has share {Share}, outside 0..=1"),
			));
		}
		Probabilities.insert(Option.clone(), Share);
	}

	let Options:Vec<String> = Probabilities.keys().cloned().collect();
	let Choice = match One.get("choice") {
		Some(Value::String(Choice)) => Choice.clone(),
		Some(Value::Null) | None => {
			return Err(DecodeError::UnknownChoice(Name.to_string(), None, Options));
		},
		Some(Other) => {
			return Err(DecodeError::Shape(
				format!("{Name}.choice"),
				format!("the chosen option is {}, not a string", Kind(Other)),
			));
		},
	};
	if !Probabilities.contains_key(&Choice) {
		return Err(DecodeError::UnknownChoice(Name.to_string(), Some(Choice), Options));
	}
	Ok(Answered::Choice { Choice, Probabilities })
}

/// What the reply reported spending, as the meter charges it.
///
/// `input_tokens` is required: it is the count the invoice is computed from, and
/// a reply that does not report it cannot be metered. `output_tokens` is read
/// when present and zero when absent, because `clef-flash` prices output at zero
/// and a body that omitted the count was not hiding a cost.
fn Spend(Value:Option<&Value>, Ceiling:Option<u64>) -> Result<Usage, DecodeError> {
	let Reported = Value.filter(|Reported| !Reported.is_null()).ok_or_else(|| {
		DecodeError::Missing(
			"result.usage".to_string(),
			"the reply reported no spend, so what it cost is unknown".to_string(),
		)
	})?;
	let Reported = Reported.as_object().ok_or_else(|| {
		DecodeError::BudgetShape(format!("`usage` is {}, not an object of token counts", Kind(Reported)))
	})?;
	let Input = Count(Reported, "input_tokens")?;
	let Output = match Reported.get("output_tokens") {
		Some(_) => Count(Reported, "output_tokens")?,
		None => 0,
	};
	Ok(Usage::Of(Input, Output, Ceiling))
}

/// One unsigned count inside `usage`.
fn Count(Usage:&Map<String, Value>, Field:&str) -> Result<u64, DecodeError> {
	let Value = Usage.get(Field).ok_or_else(|| {
		DecodeError::Missing(
			format!("result.usage.{Field}"),
			"the reply reported a spend without this count".to_string(),
		)
	})?;
	Value
		.as_u64()
		.ok_or_else(|| DecodeError::BudgetShape(format!("`{Field}` is {}, not an unsigned token count", Kind(Value))))
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
fn Complaint(Parsed:&Map<String, Value>) -> String {
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

/// The JSON type a value actually is, in one word, for a detail a person reads.
fn Kind(Value:&Value) -> &'static str {
	match Value {
		Value::Null => "null",
		Value::Bool(_) => "a boolean",
		Value::Number(_) => "a number",
		Value::String(_) => "a string",
		Value::Array(_) => "an array",
		Value::Object(_) => "an object",
	}
}

/// One value that has to be an object, or the field that was not.
fn Object<'a>(Value:&'a Value, Field:&str) -> Result<&'a Map<String, Value>, DecodeError> {
	Value
		.as_object()
		.ok_or_else(|| DecodeError::Shape(Field.to_string(), format!("expected a JSON object, found {}", Kind(Value))))
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

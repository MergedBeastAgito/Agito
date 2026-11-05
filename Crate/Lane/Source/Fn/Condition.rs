//! The corpus's condition vocabulary: how a lane writes `when`, and how it is
//! read into a [`When`].
//!
//! A lane that states its own condition is a lane an adapter adds by editing
//! JSON. This module is that statement's whole syntax, and it is deliberately a
//! closed set: nine operators, one tag per condition, no expression language. A
//! string evaluated at run time would be a parse failure mid-run, a second syntax
//! nobody's editor checks, and a way for a data file to run logic.
//!
//! # The JSON schema
//!
//! One condition is one object with exactly one operator key. `always` is the
//! exception and may also be written as the bare string.
//!
//! ```json
//! "always"
//! { "is":      { "field": "verdict", "value": "blocked" } }
//! { "is-not":  { "field": "gear", "value": -1 } }
//! { "one-of":  { "field": "verdict", "values": ["blocked", "stalled"] } }
//! { "more":    { "field": "speed", "than": 60 } }
//! { "less":    { "field": "moved", "than": 1.5 } }
//! { "at-least":{ "field": "speed", "than": 40 } }
//! { "at-most": { "field": "speed", "than": 40 } }
//! { "has":     { "field": "route_cue" } }
//! { "lacks":   { "field": "route_cue" } }
//! { "all":     [ <condition>, <condition> ] }
//! { "any":     [ <condition>, <condition> ] }
//! { "none":    [ <condition>, <condition> ] }
//! ```
//!
//! A compound nests the same objects, so the compound case in a lane is written
//! exactly like this - two numbers and a word:
//!
//! ```json
//! {
//!   "fields": ["speed", "steer", "verdict"],
//!   "templates": ["corner.entry"],
//!   "when": {
//!     "all": [
//!       { "more": { "field": "speed", "than": 55 } },
//!       { "is": { "field": "verdict", "value": "moving" } }
//!     ]
//!   }
//! }
//! ```
//!
//! # A field is a path
//!
//! `field` is read against the caller's [`crate::Struct::Context::Context`], and
//! a dot walks into an object: `speed` reads the top level, `car.gear` reads the
//! object under `car`. A path that does not resolve is not an error and not a
//! zero - every comparison is false for it, and `lacks` is the one operator that
//! is true, because "we could not read it" is a fact a corpus may select on.
//!
//! # Adding a lane is a JSON edit
//!
//! ```json
//! "lanes": {
//!   "ice": {
//!     "fields": ["speed", "surface"],
//!     "templates": ["surface.loss"],
//!     "when": { "all": [
//!       { "is": { "field": "surface", "value": "ice" } },
//!       { "more": { "field": "speed", "than": 40 } }
//!     ] }
//!   }
//! }
//! ```
//!
//! Nothing in this crate changes: the loader reads the lane, this module reads
//! its condition, and [`crate::Fn::SelectLane::SelectLane`] evaluates it. The
//! cost of that freedom is bounded by the vocabulary above - a corpus can only
//! say things this crate already knows how to answer.
//!
//! # Why the refusal is a sentence
//!
//! [`Condition`] answers `Result<When, String>` rather than an error type of its
//! own. The path and the lane id that a load refusal must name belong to the
//! loader, which is this function's only caller and the only code that knows
//! where the bytes came from; a type here would carry a path it had to be given.
//! The sentence names the operator and the fragment that was wrong, so the loader
//! can wrap it without rewriting it.

use serde::{Deserialize, Deserializer, Serialize, Serializer};
use serde_json::{Map, Value};

use crate::Struct::When::When;

/// The operators this crate reads, in the order they are documented.
///
/// Exported because a refusal that says "unknown operator" is only useful beside
/// the list of the ones that exist, and because a test asserting the vocabulary
/// should not restate it.
pub const Operators:&[&str] =
	&["always", "is", "is-not", "one-of", "more", "less", "at-least", "at-most", "has", "lacks", "all", "any", "none"];

/// Read one lane's `when` into the condition it states.
pub fn Condition(Body:&Value) -> Result<When, String> { Read(Body) }

/// Every field path a condition reads, in the order it reads them.
///
/// The loader's use is to refuse a condition over a field the corpus never
/// declared; a trace's use is to say what a lane looked at without re-running
/// the selection. Duplicates are kept, because a condition that reads one field
/// twice was written that way on purpose.
pub fn Fields(Condition:&When) -> Vec<&str> {
	match Condition {
		When::Always => Vec::new(),
		When::AnyOf(Cases) | When::AllOf(Cases) | When::NoneOf(Cases) =>
			Cases.iter().flat_map(Fields).collect(),
		One => One.Field().into_iter().collect(),
	}
}

/// One condition, read recursively.
fn Read(Body:&Value) -> Result<When, String> {
	if let Some(Word) = Body.as_str() {
		return match Word {
			"always" => Ok(When::Always),
			Other => Err(format!("`{Other}` is not a condition; the only bare word is `always`")),
		};
	}
	let Some(Fields) = Body.as_object() else {
		return Err(format!(
			"a condition is an object with one operator key, found {}",
			Kind(Body)
		));
	};
	if Fields.len() != 1 {
		let Written:Vec<&str> = Fields.keys().map(String::as_str).collect();
		return Err(format!(
			"a condition names exactly one operator, found {}: {}",
			Fields.len(),
			Written.join(", ")
		));
	}
	let (Operator, Payload) = Fields.iter().next().expect("the map has one entry");
	match Operator.as_str() {
		"always" => Ok(When::Always),
		"is" => {
			let (Field, Value) = Pair(Payload, Operator)?;
			Ok(When::Is { Field, Value })
		},
		"is-not" => {
			let (Field, Value) = Pair(Payload, Operator)?;
			Ok(When::IsNot { Field, Value })
		},
		"one-of" => {
			let Object = Object(Payload, Operator)?;
			let Field = Text(Object.get("field"), Operator, "field")?;
			let Values = Object
				.get("values")
				.and_then(Value::as_array)
				.ok_or_else(|| format!("`{Operator}` needs a `values` array, found {}", Found(Object.get("values"))))?;
			Ok(When::OneOf { Field, Values:Values.clone() })
		},
		"more" => Bound(Payload, Operator).map(|(Field, Than)| When::More { Field, Than }),
		"less" => Bound(Payload, Operator).map(|(Field, Than)| When::Less { Field, Than }),
		"at-least" => Bound(Payload, Operator).map(|(Field, Than)| When::AtLeast { Field, Than }),
		"at-most" => Bound(Payload, Operator).map(|(Field, Than)| When::AtMost { Field, Than }),
		"has" => Field(Payload, Operator).map(|Field| When::Has { Field }),
		"lacks" => Field(Payload, Operator).map(|Field| When::Lacks { Field }),
		"all" => Cases(Payload, Operator).map(When::AllOf),
		"any" => Cases(Payload, Operator).map(When::AnyOf),
		"none" => Cases(Payload, Operator).map(When::NoneOf),
		Other => Err(format!(
			"`{Other}` is not an operator; the vocabulary is {}",
			Operators.join(", ")
		)),
	}
}

/// `{"field": "...", "value": ...}` - the two-operand leaves.
fn Pair(Payload:&Value, Operator:&str) -> Result<(String, Value), String> {
	let Object = Object(Payload, Operator)?;
	let Field = Text(Object.get("field"), Operator, "field")?;
	let Value = Object
		.get("value")
		.cloned()
		.ok_or_else(|| format!("`{Operator}` needs a `value`, found nothing"))?;
	Ok((Field, Value))
}

/// `{"field": "...", "than": <number>}` - the two numeric leaves.
fn Bound(Payload:&Value, Operator:&str) -> Result<(String, f64), String> {
	let Object = Object(Payload, Operator)?;
	let Field = Text(Object.get("field"), Operator, "field")?;
	let Than = Object
		.get("than")
		.and_then(Value::as_f64)
		.ok_or_else(|| format!("`{Operator}` needs a numeric `than`, found {}", Found(Object.get("than"))))?;
	Ok((Field, Than))
}

/// `{"field": "..."}` - the one-field leaves.
fn Field(Payload:&Value, Operator:&str) -> Result<String, String> {
	let Object = Object(Payload, Operator)?;
	Text(Object.get("field"), Operator, "field")
}

/// `[ <condition>, ... ]` - the compounds.
fn Cases(Payload:&Value, Operator:&str) -> Result<Vec<When>, String> {
	let Listed = Payload
		.as_array()
		.ok_or_else(|| format!("`{Operator}` takes an array of conditions, found {}", Kind(Payload)))?;
	Listed.iter().map(Read).collect()
}

/// One operator's payload, which has to be an object.
fn Object<'a>(Payload:&'a Value, Operator:&str) -> Result<&'a Map<String, Value>, String> {
	Payload
		.as_object()
		.ok_or_else(|| format!("`{Operator}` takes an object, found {}", Kind(Payload)))
}

/// One required string inside an operator's payload.
fn Text(Value:Option<&Value>, Operator:&str, Key:&str) -> Result<String, String> {
	Value
		.and_then(Value::as_str)
		.map(str::to_string)
		.ok_or_else(|| format!("`{Operator}` needs a `{Key}` string, found {}", Found(Value)))
}

/// What was found where something else was expected.
fn Found(Value:Option<&Value>) -> String { Value.map(Kind).unwrap_or("nothing").to_string() }

/// The JSON type a value actually is, in one word, for a sentence a person reads.
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

// The condition is also a value a program may hold, so it reads and writes the
// same syntax it is authored in. The implementations live here rather than in
// `Struct::When` because the schema and the type are one subject: a second
// spelling of a condition would be a corpus two authors can disagree in.
impl Serialize for When {
	fn serialize<S: Serializer>(&self, Serializer:S) -> Result<S::Ok, S::Error> {
		Value::serialize(&Written(self), Serializer)
	}
}

impl<'de> Deserialize<'de> for When {
	fn deserialize<D: Deserializer<'de>>(Deserializer:D) -> Result<Self, D::Error> {
		let Body = Value::deserialize(Deserializer)?;
		Read(&Body).map_err(D::Error::custom)
	}
}

/// The condition as the JSON it was written as.
fn Written(Condition:&When) -> Value {
	match Condition {
		When::Always => Value::String("always".to_string()),
		When::Is { Field, Value: Held } => One("is", [("field", Value::String(Field.clone())), ("value", Held.clone())]),
		When::IsNot { Field, Value: Held } => {
			One("is-not", [("field", Value::String(Field.clone())), ("value", Held.clone())])
		},
		When::OneOf { Field, Values } => One(
			"one-of",
			[("field", Value::String(Field.clone())), ("values", Value::Array(Values.clone()))],
		),
		When::More { Field, Than } => One("more", [("field", Value::String(Field.clone())), ("than", Number(*Than))]),
		When::Less { Field, Than } => One("less", [("field", Value::String(Field.clone())), ("than", Number(*Than))]),
		When::AtLeast { Field, Than } => {
			One("at-least", [("field", Value::String(Field.clone())), ("than", Number(*Than))])
		},
		When::AtMost { Field, Than } => {
			One("at-most", [("field", Value::String(Field.clone())), ("than", Number(*Than))])
		},
		When::Has { Field } => One("has", [("field", Value::String(Field.clone()))]),
		When::Lacks { Field } => One("lacks", [("field", Value::String(Field.clone()))]),
		When::AllOf(Cases) => One("all", Value::Array(Cases.iter().map(Written).collect())),
		When::AnyOf(Cases) => One("any", Value::Array(Cases.iter().map(Written).collect())),
		When::NoneOf(Cases) => One("none", Value::Array(Cases.iter().map(Written).collect())),
	}
}

/// One operator and its payload, as one object.
fn One<const N:usize>(Operator:&str, Payload:[(&str, Value); N]) -> Value {
	let Fields:Map<String, Value> = Payload.into_iter().map(|(Key, Value)| (Key.to_string(), Value)).collect();
	let mut Object = Map::new();
	Object.insert(Operator.to_string(), Value::Object(Fields));
	Value::Object(Object)
}

/// A number as JSON, refusing the one value JSON cannot hold.
///
/// A NaN or an infinity cannot have been written by a corpus - JSON has no
/// spelling for either - but a condition may be built in Rust, so the write
/// answers `null` rather than a number that would fail to parse back. The read
/// side refuses `null`, so the pair is not silently equal; the value is simply
/// not expressible, which is the honest answer.
fn Number(Value:f64) -> Value {
	serde_json::Number::from_f64(Value).map(Value::Number).unwrap_or(Value::Null)
}

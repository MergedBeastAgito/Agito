//! What a condition reads: a field path and the value behind it.
//!
//! A context is not a [`crate::Struct::State::State`], although the two hold the
//! same kind of thing. A state is what a request *sends*: every field in it is a
//! field the model will be shown, so it is kept flat and a field the caller did
//! not observe is written as `null` rather than omitted. A context is what a
//! *condition* reads, and it is larger in two directions - it may carry a
//! counter the caller keeps, a knob the corpus never sends, a sentence the
//! ingestion derived - and it may group its fields, because a context built by
//! an adapter is that adapter's shape rather than the endpoint's.
//!
//! Keeping the two apart is what lets a lane select on a field it would never
//! spend tokens on, and [`Context::Leaf`] is the seam between them: it answers a
//! one-field state, so a condition written against a state answers against a
//! context without a second implementation of every comparison.
//!
//! # A path is a name, and a dot walks into an object
//!
//! `speed` reads the top level. `car.speed` reads the object under `car`. The
//! exact key is tried first, so a field whose own name contains a dot is still
//! reachable by writing that name; only when no such key exists is the name read
//! as a path. A path that walks into a scalar, an array or a `null` answers
//! nothing rather than failing - a context is assembled under load and a
//! condition is not a place to fail a run.
//!
//! # Missing is not a value
//!
//! [`Context::Get`] answers `None` for a field that is absent *and* for one that
//! is `null`, and [`Context::Leaf`] answers a state with no field in it for
//! both. That is the whole of "an unobserved field is not a field of zero": a
//! reading of zero and a reading the ingestion could not take must not look
//! alike, and every comparison in [`crate::Struct::When::When`] is false for the
//! second.
//!
//! # The JSON shape
//!
//! A context is one JSON object, and nothing else is accepted:
//!
//! ```json
//! {
//!   "speed_kmh": 38,
//!   "verdict": "moving",
//!   "car": { "gear": 3, "rpm": 4100 },
//!   "knob": { "target": 60 }
//! }
//! ```
//!
//! That object is exactly what [`Context::Of`] reads and what
//! [`Context::Body`] writes, so a context captured to a file replays into the
//! same value - which is what makes a selection arguable after the fact.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

use crate::Struct::{Error::Error, State::State};

/// One context, as the caller observed it.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Context(BTreeMap<String, Value>);

impl Context {
	/// An empty context.
	pub fn New() -> Self { Self(BTreeMap::new()) }

	/// The context a JSON object names.
	///
	/// Anything that is not an object is refused: an array of readings has no
	/// field names, and a condition over it could not say what it read. The
	/// refusal carries the value, because the caller holding an array is one
	/// mistake away from the object it meant.
	pub fn Of(Value:Value) -> Result<Self, Error> {
		match Value {
			Value::Object(Fields) => Ok(Self(Fields.into_iter().collect())),
			Other => Err(Error::CorpusInvalid("the context is not a JSON object", Other.to_string())),
		}
	}

	/// Record one field.
	pub fn Set(&mut self, Field:&str, Value:impl Into<Value>) { self.0.insert(Field.to_string(), Value.into()); }

	/// The same context with one field recorded, for building one in an
	/// expression.
	pub fn With(mut self, Field:&str, Value:impl Into<Value>) -> Self {
		self.Set(Field, Value);
		self
	}

	/// One field's value, or `None` when it is absent or null.
	pub fn Get(&self, Field:&str) -> Option<&Value> {
		if let Some(Direct) = self.0.get(Field).filter(|Value| !Value.is_null()) {
			return Some(Direct);
		}
		if !Field.contains('.') {
			return None;
		}
		let mut Steps = Field.split('.');
		let mut Cursor = self.0.get(Steps.next()?)?;
		for Step in Steps {
			Cursor = Cursor.get(Step)?;
		}
		(!Cursor.is_null()).then_some(Cursor)
	}

	/// One field as a number, when it is one.
	pub fn Number(&self, Field:&str) -> Option<f64> { self.Get(Field)?.as_f64() }

	/// Whether the context carries this field with a value.
	pub fn Has(&self, Field:&str) -> bool { self.Get(Field).is_some() }

	/// One field as the state a condition reads.
	///
	/// The key is the path that was asked for, not the leaf it resolved to: a
	/// condition holds the path it was written with, and asking
	/// [`crate::Struct::When::When::Holds`] about `car.gear` has to find the
	/// field it named. An unresolved path answers a state with no field at all,
	/// so `has` is false and every comparison is false - the same statement as
	/// "we could not read it".
	pub fn Leaf(&self, Field:&str) -> State {
		let mut Leaf = State::New();
		if let Some(Value) = self.Get(Field) {
			Leaf.Set(Field, Value.clone());
		}
		Leaf
	}

	/// Every field name at the top level, sorted.
	pub fn Fields(&self) -> impl Iterator<Item = &String> { self.0.keys() }

	/// The context as a JSON object.
	pub fn Body(&self) -> Value { Value::Object(self.0.clone().into_iter().collect::<Map<String, Value>>()) }

	/// Whether anything was recorded at all.
	pub fn IsEmpty(&self) -> bool { self.0.is_empty() }

	/// The context as one line, for a refusal that has to say what was tried.
	///
	/// A missing match is read by a person comparing two runs, so the context
	/// travels with the refusal: "no lane holds" is a sentence nobody can act on
	/// without the values that were held against the corpus.
	pub fn Summary(&self) -> String {
		if self.IsEmpty() {
			return "an empty context".to_string();
		}
		self.0
			.iter()
			.map(|(Field, Value)| {
				match Value {
					Value::Null => format!("{Field}=null"),
					Value::Object(_) => format!("{Field}={}", Compact(Value)),
					Other => format!("{Field}={Other}"),
				}
			})
			.collect::<Vec<String>>()
			.join(", ")
	}
}

impl From<State> for Context {
	fn from(State:State) -> Self { Self(State.Body().as_object().cloned().unwrap_or_default().into_iter().collect()) }
}

impl From<&State> for Context {
	fn from(State:&State) -> Self { Self::from(State.clone()) }
}

impl From<BTreeMap<String, Value>> for Context {
	fn from(Fields:BTreeMap<String, Value>) -> Self { Self(Fields) }
}

/// A value as one line, for a summary that has to stay one line.
fn Compact(Value:&Value) -> String {
	let Written = Value.to_string();
	if Written.chars().count() <= 120 {
		return Written;
	}
	let Shortened:String = Written.chars().take(120).collect();
	format!("{Shortened}…")
}

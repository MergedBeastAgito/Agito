//! What the caller observed, named the way the corpus names it.
//!
//! A state is a map and not a struct, because the fields belong to the corpus
//! rather than to this crate: a game names `altitude`, a page names
//! `scroll_y`, and a crate that grew a field for each would be a crate per
//! caller. The map is the only shape that lets one loader serve both.
//!
//! Two rules make the map honest:
//!
//! - **A field that is not there is not zero.** [`State::Get`] answers `None`
//!   for a missing field *and* for a field that is `null`, so a predicate
//!   written over it is false rather than true-by-accident. A reading of zero
//!   and a reading the ingestion could not take must not look alike.
//! - **A lane sends the fields it names, and no others.** [`State::Keep`]
//!   answers every field the lane lists - with `null` when the state does not
//!   carry it - so the request says "this was not observed" instead of silently
//!   omitting the field and leaving the model to guess whether it was asked
//!   about.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

use crate::Struct::Error::Error;

/// One observation, as the corpus names its fields.
///
/// `transparent`, so the state *is* the object it carries: a request's `state`
/// key holds these fields directly rather than a `{"fields": {...}}` wrapper
/// nobody would write.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct State(BTreeMap<String, Value>);

impl State {
	/// An empty observation.
	pub fn New() -> Self { Self(BTreeMap::new()) }

	/// The state a JSON object names.
	///
	/// Anything that is not an object is refused: an array of readings has no
	/// field names, and a request built from it would be a request the model
	/// cannot read.
	pub fn Of(Value:Value) -> Result<Self, Error> {
		match Value {
			Value::Object(Fields) => Ok(Self(Fields.into_iter().collect())),
			Other => Err(Error::CorpusInvalid("the state is not a JSON object", Other.to_string())),
		}
	}

	/// Record one field.
	pub fn Set(&mut self, Field:&str, Value:impl Into<Value>) { self.0.insert(Field.to_string(), Value.into()); }

	/// One field's value, or `None` when it is absent or null.
	pub fn Get(&self, Field:&str) -> Option<&Value> { self.0.get(Field).filter(|Value| !Value.is_null()) }

	/// One field as a number, when it is one.
	pub fn Number(&self, Field:&str) -> Option<f64> { self.Get(Field)?.as_f64() }

	/// Whether the state carries this field with a value.
	pub fn Has(&self, Field:&str) -> bool { self.Get(Field).is_some() }

	/// The fields one lane names, in the lane's own order, with `null` for the
	/// ones this state does not carry.
	pub fn Keep(&self, Fields:&[String]) -> State {
		State(
			Fields
				.iter()
				.map(|Field| (Field.clone(), self.0.get(Field).cloned().unwrap_or(Value::Null)))
				.collect(),
		)
	}

	/// The state as the JSON a request carries.
	pub fn Body(&self) -> Value { Value::Object(self.0.clone().into_iter().collect::<Map<String, Value>>()) }

	/// Every field name, sorted.
	pub fn Fields(&self) -> impl Iterator<Item = &String> { self.0.keys() }

	/// Whether anything was observed at all.
	pub fn IsEmpty(&self) -> bool { self.0.is_empty() }
}

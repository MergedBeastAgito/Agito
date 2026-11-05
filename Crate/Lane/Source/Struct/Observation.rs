//! The observation: what the ingestion can hand over, one typed entry per field.
//!
//! `Observation.json` is the corpus's other half, and this is what it becomes.
//! It is *descriptive* rather than structural: an address, a unit, a range and a
//! source are the ingestion's vocabulary, and this crate does not read a single
//! one of them. They are carried because a corpus author needs somewhere to say
//! where a number comes from, and because the loader refuses a lane that sends a
//! field the observation never declared - which is a question the model cannot
//! answer and a failure worth finding before a request is paid for.
//!
//! # The JSON shape
//!
//! ```json
//! {
//!   "$comment": "what the file is for, if the author wants to say",
//!   "version": 1,
//!   "state": {
//!     "speed_kmh": {
//!       "type": "number", "source": "memory", "address": "car+0x430",
//!       "unit": "km/h", "range": [0, 400], "note": "signed forward speed"
//!     },
//!     "grounded": { "type": "array", "items": "boolean", "length": 4 },
//!     "note": { "type": "string" }
//!   },
//!   "derived": {
//!     "moved_m": "metres the car advanced over the last six ticks"
//!   }
//! }
//! ```
//!
//! Three things about this mapping are deliberate:
//!
//! - **A field's name is its key, and is not repeated inside the entry.** A
//!   corpus that wrote the name twice could disagree with itself, and the entry
//!   that disagreed would be the one nothing read.
//! - **Every descriptor is optional, `type` included.** The real corpus writes
//!   `type` on every field; the crate's own fixture writes a field described only
//!   by a unit. Requiring a key the fixture does not carry would refuse a corpus
//!   that works, and this crate does not interpret the type word anyway - the
//!   lane decides what travels and the model decides what it means.
//! - **An unknown key is kept, not dropped.** `extra` holds whatever the entry
//!   wrote that this crate has not been taught, so a corpus may describe itself
//!   further without waiting for a new release - and a round trip through these
//!   types does not silently delete the author's notes.
//!
//! `derived` is prose by design. A derived value is computed by whoever holds the
//! window - "not moving for three ticks" is a counter, not a field - so the
//! corpus states what it means and the caller states its value in the context.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::Struct::Error::Error;

/// One corpus's observation, as it was written.
///
/// Every field states its own key rather than relying on a container
/// `rename_all`: the serde build this workspace resolves does not apply one to
/// struct fields, and an observation that read nothing would describe a corpus
/// with no fields at all.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Observation {
	/// The version the observation file declares.
	#[serde(rename = "version", default, skip_serializing_if = "Zero")]
	pub Version:u64,
	/// Every field the ingestion can hand over, by the name the corpus uses.
	#[serde(rename = "state", default, skip_serializing_if = "BTreeMap::is_empty")]
	pub Fields:BTreeMap<String, ObservationField>,
	/// The derived values the observation names, as the prose that defines them.
	#[serde(rename = "derived", default, skip_serializing_if = "BTreeMap::is_empty")]
	pub Derived:BTreeMap<String, Value>,
	/// The file's own comment, when it wrote one.
	#[serde(rename = "$comment", default, skip_serializing_if = "Option::is_none")]
	pub Note:Option<String>,
	/// Every root key this crate has not been taught.
	#[serde(flatten, default, skip_serializing_if = "BTreeMap::is_empty")]
	pub Extra:BTreeMap<String, Value>,
}

/// One field the ingestion can hand over.
///
/// Every descriptor is optional because a corpus may describe a field as fully
/// or as loosely as it likes; `type` is the one the real corpus always writes,
/// and even that is carried rather than interpreted.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct ObservationField {
	/// The corpus's own type word: `number`, `integer`, `array`, `string`...
	#[serde(rename = "type", default, skip_serializing_if = "Option::is_none")]
	pub Kind:Option<String>,
	/// Where the value comes from: the ingestion's own word.
	#[serde(rename = "source", default, skip_serializing_if = "Option::is_none")]
	pub Source:Option<String>,
	/// The ingestion's own address for it, as a string it understands.
	#[serde(rename = "address", default, skip_serializing_if = "Option::is_none")]
	pub Address:Option<String>,
	/// The unit the value is in.
	#[serde(rename = "unit", default, skip_serializing_if = "Option::is_none")]
	pub Unit:Option<String>,
	/// The range the value stays within, when the corpus states one.
	///
	/// Read as numbers, so a bound written `0` and one written `0.0` are the same
	/// bound, and a round trip writes the decimal form. A range is a numeric
	/// fact: the ingestion clamps against it, so keeping the file's spelling of
	/// an integer would be a second, textual fact about the same number.
	#[serde(rename = "range", default, skip_serializing_if = "Option::is_none")]
	pub Range:Option<Vec<f64>>,
	/// For an array, what its items are.
	#[serde(rename = "items", default, skip_serializing_if = "Option::is_none")]
	pub Items:Option<String>,
	/// For an array, how many items it holds.
	#[serde(rename = "length", default, skip_serializing_if = "Option::is_none")]
	pub Length:Option<u64>,
	/// What the field means, in the author's words.
	#[serde(rename = "note", default, skip_serializing_if = "Option::is_none")]
	pub Note:Option<String>,
	/// Every key this crate has not been taught.
	#[serde(flatten, default, skip_serializing_if = "BTreeMap::is_empty")]
	pub Extra:BTreeMap<String, Value>,
}

impl ObservationField {
	/// One descriptor, typed.
	///
	/// A descriptor that is not an object - a bare string where an entry was
	/// expected - is carried whole under `$value` in [`ObservationField::Extra`]
	/// rather than refused: the loader is the only thing that decides whether a
	/// corpus is acceptable, and it asks about names rather than descriptors.
	pub fn Of(Descriptor:&Value) -> Self {
		serde_json::from_value(Descriptor.clone()).unwrap_or_else(|_| {
			let mut Extra = BTreeMap::new();
			Extra.insert("$value".to_string(), Descriptor.clone());
			Self { Extra, ..Self::default() }
		})
	}

	/// The corpus's own type word, when it wrote one.
	pub fn Kind(&self) -> Option<&str> { self.Kind.as_deref() }
}

impl Observation {
	/// The observation a parsed JSON value names.
	///
	/// A value that is not the shape above is refused with the reader's own
	/// complaint in the detail, because the person who has to fix it is looking
	/// at the file rather than at this crate.
	pub fn Of(Value:Value) -> Result<Self, Error> {
		serde_json::from_value(Value)
			.map_err(|Why| Error::CorpusInvalid("the observation is not the shape this crate reads", Why.to_string()))
	}

	/// The observation a corpus's own two tables describe.
	///
	/// The descriptors a loader read are the source of truth, and this is their
	/// typed view: one entry per field, built from the descriptor rather than
	/// stored beside it. A descriptor that is not the shape above is carried in
	/// [`ObservationField::Extra`] under `$value` rather than refused, because
	/// this crate does not interpret descriptors - refusing one here would
	/// refuse a corpus that loads, over a field nothing reads.
	pub fn OfEntries(Fields:&BTreeMap<String, Value>, Derived:&BTreeMap<String, Value>) -> Self {
		Self {
			Version:0,
			Fields:Fields
				.iter()
				.map(|(Name, Descriptor)| (Name.clone(), ObservationField::Of(Descriptor)))
				.collect(),
			Derived:Derived.clone(),
			Note:None,
			Extra:BTreeMap::new(),
		}
	}

	/// Every field the ingestion can hand over, sorted by name.
	pub fn Fields(&self) -> impl Iterator<Item = (&String, &ObservationField)> { self.Fields.iter() }

	/// One field's descriptor, when the observation declares it.
	pub fn Field(&self, Name:&str) -> Option<&ObservationField> { self.Fields.get(Name) }

	/// Whether the observation declares this field.
	pub fn Declares(&self, Name:&str) -> bool { self.Fields.contains_key(Name) }

	/// Every derived value the observation names, sorted.
	pub fn Derived(&self) -> impl Iterator<Item = (&String, &Value)> { self.Derived.iter() }

	/// Whether the observation declares nothing at all.
	pub fn IsEmpty(&self) -> bool { self.Fields.is_empty() && self.Derived.is_empty() }
}

/// Whether a number is zero, for `skip_serializing_if`.
///
/// A named function rather than a closure because serde's attribute takes a
/// path, and a `version` of zero is "the file did not say" rather than a
/// version - writing it back would invent a fact.
fn Zero(Number:&u64) -> bool { *Number == 0 }

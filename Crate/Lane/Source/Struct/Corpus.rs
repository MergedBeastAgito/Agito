//! The corpus: an observation, its lanes, its templates, and where they came
//! from.
//!
//! A corpus is a directory, and this is the value its files become: a
//! `Templates/Observation.json` naming every field a caller may send, a
//! `Templates/Questions.json` naming the templates, and the lane definitions
//! beside them - inline in the question file or in a file of their own. The
//! fields are public because a corpus is data a caller may also build by hand in
//! a test, but [`crate::Fn::Load::Load`] and [`crate::Fn::Load::Read`] are the
//! constructors that mean anything: they are the only ones that check the corpus
//! against itself.
//!
//! # Where it came from, and in what order
//!
//! [`Corpus::Root`] is the directory the corpus was read out of, and `None` for
//! a corpus read from two strings. It is carried rather than guessed because a
//! report that says "the corpus" is a report nobody can act on: a run has to be
//! able to name the files it obeyed.
//!
//! [`Corpus::Order`] is the lane ids in the order the corpus declared them. A
//! `BTreeMap` sorts for determinism, and sorting is the right answer for a
//! printed policy and the wrong one for a selection: a corpus may state a
//! priority by writing its lanes in an order, and a map cannot remember one. A
//! lane the order does not mention is still selectable - [`Corpus::Ordered`]
//! appends it rather than hiding it, so an order that went stale when someone
//! added a lane cannot make the new lane invisible.
//!
//! # The observation, twice
//!
//! [`Corpus::Fields`] keeps the descriptors exactly as the file wrote them,
//! because an address, a unit and a range are the *ingestion's* vocabulary -
//! this crate only needs the names, and it uses them for one thing: to refuse a
//! lane that sends a field the observation never declared. A question about a
//! field the ingestion does not carry is a question the model cannot answer, and
//! finding that out at load is cheaper than finding it out from an answer that
//! ignored the field.
//!
//! [`Corpus::Observation`] is the same data typed, one entry per field, for a
//! caller that wants to read a descriptor rather than carry it. The two cannot
//! disagree: the typed view is built from the descriptors, never stored beside
//! them.
//!
//! The *derived* conditions are kept for the same reason, and they are as
//! sendable as the read ones: `verdict` and `situation` are computed by the
//! ingestion rather than read from it, and a lane that sends the sentence
//! beside the numbers is the lane that works. [`Corpus::Observed`] is the union
//! of the two, which is what a lane is checked against.

use std::{collections::BTreeMap, path::PathBuf};

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::Struct::{Lane::Lane, Observation::Observation, Template::Template};

/// One corpus, as it was read.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct Corpus {
	/// The directory it was read out of, or `None` when it came from strings.
	#[serde(skip)]
	pub Root:Option<PathBuf>,
	/// The version the question file declares.
	#[serde(default, skip_serializing_if = "Zero")]
	pub Version:u64,
	/// Every field a caller may send, by the name the corpus uses, with the
	/// observation's own descriptor.
	#[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
	pub Fields:BTreeMap<String, Value>,
	/// The derived conditions the observation states, as prose.
	#[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
	pub Derived:BTreeMap<String, Value>,
	/// The lanes, by name.
	#[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
	pub Lanes:BTreeMap<String, Lane>,
	/// The templates, by name.
	#[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
	pub Templates:BTreeMap<String, Template>,
	/// The lane ids in the order the corpus declared them.
	#[serde(default, skip_serializing_if = "Vec::is_empty")]
	pub Order:Vec<String>,
}

impl Corpus {
	/// The directory it was read out of, when it was read from one.
	pub fn Root(&self) -> Option<&PathBuf> { self.Root.as_ref() }

	/// The version the question file declares.
	pub fn Version(&self) -> u64 { self.Version }

	/// Every observed field, sorted by name.
	pub fn Fields(&self) -> impl Iterator<Item = (&String, &Value)> { self.Fields.iter() }

	/// One field's descriptor, when the observation declares it.
	pub fn Field(&self, Name:&str) -> Option<&Value> { self.Fields.get(Name) }

	/// Whether the observation names this field at all: a reading the ingestion
	/// takes, or a condition it derives from those readings.
	///
	/// The union rather than the readings alone, because the observation file
	/// declares both halves and a lane may send either - the derived sentence
	/// (`verdict`, `situation`) is the part of a state that says what the
	/// numbers mean, and a lane that sends it is the lane that works. The
	/// readings on their own are [`Corpus::Fields`] and [`Corpus::Field`].
	pub fn Observed(&self, Name:&str) -> bool { self.Fields.contains_key(Name) || self.Derived.contains_key(Name) }

	/// The derived conditions the observation states.
	pub fn Derived(&self) -> impl Iterator<Item = (&String, &Value)> { self.Derived.iter() }

	/// The observation as typed entries, one per declared field.
	///
	/// Built on demand rather than stored, so a corpus cannot hold two answers
	/// to "which fields exist" - the descriptors above are the only copy.
	pub fn Observation(&self) -> Observation { Observation::OfEntries(&self.Fields, &self.Derived) }

	/// Every lane, sorted by name. This is a stable order for printing, not a
	/// priority: [`Corpus::Ordered`] is the order the corpus declared.
	pub fn Lanes(&self) -> impl Iterator<Item = &Lane> { self.Lanes.values() }

	/// The lanes in the order the corpus declared them.
	///
	/// A lane the order list does not mention is appended rather than hidden: an
	/// order that went stale when someone added a lane must not make the new
	/// lane invisible to a caller that prints its policy.
	pub fn Ordered(&self) -> Vec<&Lane> {
		let Named:Vec<&Lane> = self.Order.iter().filter_map(|Name| self.Lanes.get(Name)).collect();
		let Rest:Vec<&Lane> = self
			.Lanes
			.iter()
			.filter(|(Name, _)| !self.Order.iter().any(|Listed| Listed == *Name))
			.map(|(_, Lane)| Lane)
			.collect();
		Named.into_iter().chain(Rest).collect()
	}

	/// One lane, when the corpus declares it.
	pub fn Lane(&self, Name:&str) -> Option<&Lane> { self.Lanes.get(Name) }

	/// Every template, sorted by name.
	pub fn Templates(&self) -> impl Iterator<Item = &Template> { self.Templates.values() }

	/// One template, when the corpus declares it.
	pub fn Template(&self, Name:&str) -> Option<&Template> { self.Templates.get(Name) }

	/// Whether the corpus declares nothing at all.
	pub fn IsEmpty(&self) -> bool {
		self.Lanes.is_empty() && self.Templates.is_empty() && self.Fields.is_empty() && self.Derived.is_empty()
	}
}

/// Whether a number is zero, for `skip_serializing_if`.
///
/// A version of zero is "the file did not say" rather than a version, and
/// writing it back would invent a fact.
fn Zero(Number:&u64) -> bool { *Number == 0 }

//! The corpus: an observation, its lanes, its templates.
//!
//! Two files in one directory, and this is the value they become. The fields are
//! public because a corpus is data a caller may also build by hand in a test,
//! but [`crate::Fn::Load::Load`] and [`crate::Fn::Load::Read`] are the
//! constructors that mean anything: they are the only ones that check the corpus
//! against itself.
//!
//! The observation is kept as the descriptors the file wrote rather than as a
//! parsed field type, because an address, a unit and a range are the *ingestion's*
//! vocabulary - this crate only needs the names, and it uses them for one
//! thing: to refuse a lane that sends a field the observation never declared.
//! A question about a field the ingestion does not carry is a question the model
//! cannot answer, and finding that out at load is cheaper than finding it out
//! from an answer that ignored the field.

use std::collections::BTreeMap;

use serde_json::Value;

use crate::Struct::{Lane::Lane, Template::Template};

/// One corpus, as it was read.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Corpus {
	/// The version the question file declares.
	pub Version:u64,
	/// Every field a caller may send, by the name the corpus uses, with the
	/// observation's own descriptor.
	pub Fields:BTreeMap<String, Value>,
	/// The derived conditions the observation states, as prose.
	pub Derived:BTreeMap<String, Value>,
	/// The lanes, by name.
	pub Lanes:BTreeMap<String, Lane>,
	/// The templates, by name.
	pub Templates:BTreeMap<String, Template>,
}

impl Corpus {
	/// The version the question file declares.
	pub fn Version(&self) -> u64 { self.Version }

	/// Every observed field, sorted by name.
	pub fn Fields(&self) -> impl Iterator<Item = (&String, &Value)> { self.Fields.iter() }

	/// One field's descriptor, when the observation declares it.
	pub fn Field(&self, Name:&str) -> Option<&Value> { self.Fields.get(Name) }

	/// Whether the observation declares this field.
	pub fn Observed(&self, Name:&str) -> bool { self.Fields.contains_key(Name) }

	/// The derived conditions the observation states.
	pub fn Derived(&self) -> impl Iterator<Item = (&String, &Value)> { self.Derived.iter() }

	/// Every lane, sorted by name. Selection order is the caller's rules, not
	/// this order: the map is sorted for determinism, not for priority.
	pub fn Lanes(&self) -> impl Iterator<Item = &Lane> { self.Lanes.values() }

	/// One lane, when the corpus declares it.
	pub fn Lane(&self, Name:&str) -> Option<&Lane> { self.Lanes.get(Name) }

	/// Every template, sorted by name.
	pub fn Templates(&self) -> impl Iterator<Item = &Template> { self.Templates.values() }

	/// One template, when the corpus declares it.
	pub fn Template(&self, Name:&str) -> Option<&Template> { self.Templates.get(Name) }
}

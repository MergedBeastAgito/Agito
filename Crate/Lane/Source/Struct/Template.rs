//! One template: the questions a lane asks, and the condition the corpus says
//! they belong to.
//!
//! The `when` here is the corpus author's sentence about when the template
//! applies, and it is carried as prose rather than parsed: the condition that
//! actually selects a template is a [`crate::Struct::When::When`] the caller
//! supplies, because only the caller can read the state it has. Keeping the
//! sentence is still worth it - a template whose only description is its
//! question text is one nobody can tell apart from its neighbour.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::Struct::Question::Question;

/// One question set.
///
/// # The JSON shape
///
/// A template is one entry in a corpus's `templates` object, keyed by its id:
///
/// ```json
/// {
///   "when": "approaching a turn, speed above the turn pace",
///   "questions": {
///     "action": { "type": "choice", "instructions": "What next?",
///                 "criteria": { "brake": "Brake before the turn" } }
///   }
/// }
/// ```
///
/// `when` is prose, not a condition: the condition that selects a template is
/// the caller's ([`crate::Struct::When::When`]) or the lane's own
/// ([`crate::Struct::Predicate::Predicate`]), and the sentence is kept because a
/// template whose only description is its question text is one nobody can tell
/// apart from its neighbour.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Template {
	/// The template's name, which is its key in the corpus.
	#[serde(rename = "name", default, skip_serializing_if = "String::is_empty")]
	pub Name:String,
	/// When it applies, as the corpus states it.
	#[serde(rename = "when", default, skip_serializing_if = "String::is_empty")]
	pub When:String,
	/// The questions, by name. A request carries them in this order.
	#[serde(rename = "questions", default, skip_serializing_if = "BTreeMap::is_empty")]
	pub Questions:BTreeMap<String, Question>,
}

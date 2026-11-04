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

use crate::Struct::Question::Question;

/// One question set.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Template {
	/// The template's name, which is its key in the corpus.
	pub Name:String,
	/// When it applies, as the corpus states it.
	pub When:String,
	/// The questions, by name. A request carries them in this order.
	pub Questions:BTreeMap<String, Question>,
}

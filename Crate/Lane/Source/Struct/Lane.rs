//! A lane: a request *shape*, and the templates it serves.
//!
//! A lane is not a request. It is the answer to "which fields does this question
//! actually read", and it exists because sending all sixteen every tick costs
//! tokens, latency and money for fields the question never reads. Measured on
//! `clef-flash` with one template: the three-field lane sent 375 input tokens
//! where the full state sent 499, and the round trip did not get faster - so
//! the lane's saving is the context budget, and the speed comes from asking
//! less often.
//!
//! The `cadence` and `why` fields are the corpus's own prose and are carried
//! rather than interpreted: a caller printing its policy is the reader, and a
//! lane whose reason is only in the corpus author's head is a lane nobody can
//! argue with.
//!
//! The `select` field is the corpus's own *condition*, and it is interpreted -
//! it is a [`crate::Struct::Predicate::Predicate`], read from the lane's JSON.
//! That is what lets an adapter add a lane by editing JSON: the lane carries
//! both what it sends and when it applies, so nothing in this crate has to be
//! rebuilt to know about it. A lane that states no `select` is
//! [`crate::Struct::Predicate::Predicate::Always`] - it applies whenever no more
//! specific lane does, and a corpus in which every lane is that is a corpus with
//! no conditions at all, which the selector reports rather than resolving by
//! taking the first.

use serde::{Deserialize, Serialize};

use crate::Struct::{Predicate::Predicate, Template::Template};

/// One request shape.
///
/// # The JSON shape
///
/// A lane is written as one entry in a corpus's `lanes` object, keyed by its
/// name:
///
/// ```json
/// {
///   "fields": ["depth", "rate"],
///   "templates": ["queue.drain"],
///   "model": "clef-flash",
///   "select": { "op": "gt", "field": "depth", "than": 55 },
///   "cadence": "while the queue is over its target",
///   "why": "the drain rate says how fast the backlog is actually clearing"
/// }
/// ```
///
/// `fields` names observation fields - the entries `Observation.json` declares,
/// read or derived - and is what the request sends. `templates` names question
/// templates, in preference order. `select` is the condition that chooses the
/// lane, in the vocabulary [`crate::Struct::Predicate::Predicate`] documents;
/// `model`, `cadence` and `why` are carried for the corpus author's sake. Every
/// key may be absent and the lane still loads: a lane with no `fields` asks about
/// instructions alone, and a lane with no `select` is a catch-all.
///
/// Every field states its own key rather than relying on a container
/// `rename_all`: the serde build this workspace resolves does not apply one to
/// struct fields, and a corpus whose keys silently did not match would be a
/// corpus that loaded empty. An explicit key is also the thing a reader can
/// check against the JSON above without knowing serde's rules.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Lane {
	/// The lane's name, which is its key in the corpus.
	#[serde(rename = "name", default, skip_serializing_if = "String::is_empty")]
	pub Name:String,
	/// The model this lane asks, as the corpus named it.
	///
	/// A lane names its own model because a cheap lane and an expensive one are
	/// a corpus author's decision, not a rebuild: three numbers do not need the
	/// same model as the full state. Empty means the corpus named none, and then
	/// the caller's default stands in - [`crate::Struct::Decider::Create`] stamps
	/// it onto every unnamed lane, and [`crate::Fn::Model::Bare`] answers the
	/// crate's own fallback for a lane composed without either.
	#[serde(rename = "model", default, skip_serializing_if = "String::is_empty")]
	pub Model:String,
	/// The fields it sends, in the order it names them.
	#[serde(rename = "fields", default, skip_serializing_if = "Vec::is_empty")]
	pub Fields:Vec<String>,
	/// The templates it serves, in preference order.
	#[serde(rename = "templates", default, skip_serializing_if = "Vec::is_empty")]
	pub Templates:Vec<String>,
	/// The condition that selects it, as the corpus states it.
	///
	/// A lane that states no `select` is [`Predicate::Always`]: it applies
	/// whenever no more specific lane does, and two of them make a corpus that
	/// cannot say which one wins - which [`crate::Fn::SelectLane::SelectLane`]
	/// reports rather than resolving.
	#[serde(rename = "select", default)]
	pub Select:Predicate,
	/// How often it is worth sending, as the corpus says it.
	#[serde(rename = "cadence", default, skip_serializing_if = "String::is_empty")]
	pub Cadence:String,
	/// Why it exists, as the corpus says it.
	#[serde(rename = "why", default, skip_serializing_if = "String::is_empty")]
	pub Why:String,
}

/// What one selection chose: the lane, and the template inside it.
///
/// Both halves, because a request composed from a lane and the wrong template is
/// a shape with nothing in it - the lane decides which fields travel, the
/// template decides which questions are asked, and neither is a request alone.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Selection {
	/// The lane the state selected.
	#[serde(rename = "lane")]
	pub Lane:Lane,
	/// The template the state selected inside that lane.
	#[serde(rename = "template")]
	pub Template:Template,
}

impl Selection {
	/// The selection as one name, `lane / template`, which is what a trace
	/// prints.
	pub fn Name(&self) -> String { format!("{} / {}", self.Lane.Name, self.Template.Name) }
}

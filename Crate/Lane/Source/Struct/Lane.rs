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

use crate::Struct::Template::Template;

/// One request shape.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Lane {
	/// The lane's name, which is its key in the corpus.
	pub Name:String,
	/// The fields it sends, in the order it names them.
	pub Fields:Vec<String>,
	/// The templates it serves, in preference order.
	pub Templates:Vec<String>,
	/// How often it is worth sending, as the corpus says it.
	pub Cadence:String,
	/// Why it exists, as the corpus says it.
	pub Why:String,
}

/// What one selection chose: the lane, and the template inside it.
///
/// Both halves, because a request composed from a lane and the wrong template is
/// a shape with nothing in it - the lane decides which fields travel, the
/// template decides which questions are asked, and neither is a request alone.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Selection {
	/// The lane the state selected.
	pub Lane:Lane,
	/// The template the state selected inside that lane.
	pub Template:Template,
}

impl Selection {
	/// The selection as one name, `lane / template`, which is what a trace
	/// prints.
	pub fn Name(&self) -> String { format!("{} / {}", self.Lane.Name, self.Template.Name) }
}

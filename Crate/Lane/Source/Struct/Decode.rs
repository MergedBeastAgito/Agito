//! Why a reply could not be read: one variant per way it can be wrong.
//!
//! `module.json` declares seven codes for the port, and a reply this crate
//! cannot read is exactly one of them (`unparseable`). That is the right
//! granularity for a caller deciding what to do next, and the wrong one for a
//! person holding a body that will not decode: "unparseable" says nothing about
//! *which* field was wrong. So the decoder keeps its own vocabulary, finer than
//! the manifest's, and [`crate::Struct::Error::Error::Decode`] is the single
//! place the two are joined.
//!
//! # Four kinds of wrong, and they are not the same wrong
//!
//! - [`DecodeError::Malformed`] - the bytes are not JSON, or not UTF-8. Nothing
//!   can be salvaged, so the detail carries the body's opening and the parser's
//!   own sentence.
//! - [`DecodeError::Refused`] - the endpoint answered and the answer is
//!   `success: false`. The body is well formed; the *call* failed. The two are
//!   separated because their remedies are: one is a bug in this crate's reading
//!   or the endpoint's contract, the other is a credential or a quota.
//! - [`DecodeError::Missing`] / [`DecodeError::Shape`] - a field the answer
//!   cannot do without is absent, or is present as the wrong type. The field is
//!   named in both, because "missing" without a name is the opaque string this
//!   type exists to replace.
//! - [`DecodeError::UnknownChoice`] / [`DecodeError::ProbabilityShape`] /
//!   [`DecodeError::BudgetShape`] - the answer's *content* is wrong: a chosen
//!   option outside its own distribution, a share that is not a share, a spend
//!   that is not two counts.
//!
//! [`DecodeError::Empty`] is the fifth and smallest: the reply is well formed,
//! carries `answers`, and answers nothing - which is a reply no caller can act
//! on and a different fact from a missing field.
//!
//! # Why no variant carries an `f64` or a `Value`
//!
//! A caller narrowing on the variant has to be able to compare two errors, so
//! the type is `Eq` and holds no floating point. The observed value - a share
//! of `1.5`, a spend of `"many"` - belongs in the detail string, which is read
//! by a person and pasted into a report, never matched on by a program.

use std::fmt;

/// One reply that could not become an answer.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum DecodeError {
	/// The body is not JSON, or is not UTF-8.
	Malformed {
		/// The parser's sentence and the body's opening.
		Detail:String,
	},
	/// The endpoint answered `success: false`.
	Refused {
		/// What the endpoint said about the refusal.
		Detail:String,
	},
	/// A field the answer cannot do without is absent.
	Missing {
		/// The field's path in the reply, e.g. `result.answers`.
		Field:String,
		/// What was looked for and what was found instead.
		Detail:String,
	},
	/// The reply answered no question.
	Empty {
		/// What the empty object was, and the body's opening.
		Detail:String,
	},
	/// A field is present as the wrong type.
	Shape {
		/// The field's path in the reply.
		Field:String,
		/// The type expected and the type found.
		Detail:String,
	},
	/// A choice names no option, or names one the reply did not carry.
	UnknownChoice {
		/// The question whose answer is wrong.
		Question:String,
		/// The option the reply named, or `None` when it named none.
		Choice:Option<String>,
		/// Every option the reply did carry, in order.
		Options:Vec<String>,
	},
	/// A choice's probabilities are not a map of shares.
	ProbabilityShape {
		/// The question whose distribution is wrong.
		Question:String,
		/// Which option, and what its share actually was.
		Detail:String,
	},
	/// The reported spend is not two token counts.
	BudgetShape {
		/// Which count, and what it actually was.
		Detail:String,
	},
}

impl DecodeError {
	/// The word a trace records, stable across releases.
	pub const fn Name(&self) -> &'static str {
		match self {
			DecodeError::Malformed { .. } => "malformed",
			DecodeError::Refused { .. } => "refused",
			DecodeError::Missing { .. } => "missing",
			DecodeError::Empty { .. } => "empty",
			DecodeError::Shape { .. } => "shape",
			DecodeError::UnknownChoice { .. } => "unknown-choice",
			DecodeError::ProbabilityShape { .. } => "probability-shape",
			DecodeError::BudgetShape { .. } => "budget-shape",
		}
	}

	/// The sentence a human reads, which names the field or the question.
	pub fn Sentence(&self) -> String {
		match self {
			DecodeError::Malformed { .. } => "the reply is not JSON".to_string(),
			DecodeError::Refused { .. } => "the endpoint refused the request".to_string(),
			DecodeError::Missing { Field, .. } => format!("the reply carries no `{Field}`"),
			DecodeError::Empty { .. } => "the reply answered no question".to_string(),
			DecodeError::Shape { Field, .. } => format!("`{Field}` is not the shape an answer needs"),
			DecodeError::UnknownChoice { Question, Choice:Some(Choice), .. } => {
				format!("question `{Question}` chose `{Choice}`, which the reply did not carry")
			},
			DecodeError::UnknownChoice { Question, Choice:None, .. } => {
				format!("question `{Question}` named no chosen option")
			},
			DecodeError::ProbabilityShape { Question, .. } => {
				format!("question `{Question}` has probabilities this crate cannot read")
			},
			DecodeError::BudgetShape { .. } => "the reply's spend is not two token counts".to_string(),
		}
	}

	/// What was observed, which may be empty.
	///
	/// Built rather than stored for [`DecodeError::UnknownChoice`], because the
	/// options a reply carried are only worth spelling out when someone asks:
	/// a `Vec` kept solely for a log line would be a `Vec` cloned on every
	/// refusal.
	pub fn Detail(&self) -> String {
		match self {
			DecodeError::Malformed { Detail }
			| DecodeError::Refused { Detail }
			| DecodeError::Empty { Detail }
			| DecodeError::ProbabilityShape { Detail, .. }
			| DecodeError::BudgetShape { Detail } => Detail.clone(),
			DecodeError::Missing { Detail, .. } | DecodeError::Shape { Detail, .. } => Detail.clone(),
			DecodeError::UnknownChoice { Options, .. } => match Options.len() {
				0 => "the reply carried no options at all".to_string(),
				_ => format!("the options were {}", Options.join(", ")),
			},
		}
	}

	/// The body is not JSON.
	pub fn Malformed(Detail:impl Into<String>) -> Self { DecodeError::Malformed { Detail:Detail.into() } }

	/// The endpoint refused the request.
	pub fn Refused(Detail:impl Into<String>) -> Self { DecodeError::Refused { Detail:Detail.into() } }

	/// A required field is absent.
	pub fn Missing(Field:impl Into<String>, Detail:impl Into<String>) -> Self {
		DecodeError::Missing { Field:Field.into(), Detail:Detail.into() }
	}

	/// The reply answered no question.
	pub fn Empty(Detail:impl Into<String>) -> Self { DecodeError::Empty { Detail:Detail.into() } }

	/// A field is present as the wrong type.
	pub fn Shape(Field:impl Into<String>, Detail:impl Into<String>) -> Self {
		DecodeError::Shape { Field:Field.into(), Detail:Detail.into() }
	}

	/// A choice names no option, or one the reply did not carry.
	pub fn UnknownChoice(Question:impl Into<String>, Choice:Option<String>, Options:Vec<String>) -> Self {
		DecodeError::UnknownChoice { Question:Question.into(), Choice, Options }
	}

	/// A choice's probabilities are not a map of shares.
	pub fn ProbabilityShape(Question:impl Into<String>, Detail:impl Into<String>) -> Self {
		DecodeError::ProbabilityShape { Question:Question.into(), Detail:Detail.into() }
	}

	/// The reported spend is not two token counts.
	pub fn BudgetShape(Detail:impl Into<String>) -> Self { DecodeError::BudgetShape { Detail:Detail.into() } }

	/// The variant and the sentence as one word, which is what a trace records:
	/// `unknown-choice: question `course` chose `nearer`…`.
	pub fn Why(&self) -> String { format!("{}: {}", self.Name(), self.Sentence()) }
}

impl fmt::Display for DecodeError {
	fn fmt(&self, Formatter:&mut fmt::Formatter<'_>) -> fmt::Result {
		write!(Formatter, "{}: {}", self.Name(), self.Sentence())
	}
}

impl std::error::Error for DecodeError {}

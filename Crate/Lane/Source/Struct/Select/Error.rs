//! Why a selection chose nothing, or chose twice.
//!
//! Selection is the one verb in this crate that can fail without anything being
//! wrong with a file: a corpus is well formed, every lane is valid, and still no
//! lane holds for this context - or two do. Neither is a defect the loader could
//! have found, because both depend on the values the caller brought, so they are
//! reported here rather than refused at load.
//!
//! # Why this is not [`crate::Struct::Error::Error`]
//!
//! The crate's other error type is the *port's* vocabulary: seven codes,
//! enumerated in `module.json`, describing a corpus that could not be read, a
//! rule that named nothing, a budget spent, a poster that failed. A selection
//! failure happens before a request exists - there is no poster, no meter and no
//! reply to talk about - and it carries two facts the port has no shape for: the
//! context that matched nothing, and the *set* of lanes that matched.
//!
//! [`SelectError::NoMatchingLane`] is the same fact `module.json` calls
//! `no-match`, and [`From`] lowers this type into that one for a caller that
//! wants a single error type at its own boundary. Ambiguity lowers to
//! `corpus-invalid`, because two lanes that both hold for one context is a corpus
//! that disagrees with itself - the loader simply could not see it.

use std::fmt;

/// Why a selection chose nothing, or chose twice.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SelectError {
	/// No lane's condition held for this context.
	NoMatchingLane {
		/// The sentence a human reads.
		Message:String,
		/// The context that matched nothing, as one line.
		Context:String,
	},
	/// More than one lane's condition held, and the corpus says nothing about
	/// which of them wins.
	AmbiguousMatch {
		/// The sentence a human reads.
		Message:String,
		/// The lanes that held, in the corpus's own order, as it names them.
		Lanes:Vec<String>,
	},
}

impl SelectError {
	/// The sentence a human reads.
	pub fn Message(&self) -> &str {
		match self {
			SelectError::NoMatchingLane { Message, .. } | SelectError::AmbiguousMatch { Message, .. } => Message,
		}
	}

	/// What was observed, as one line: the context, or the lanes.
	pub fn Detail(&self) -> String {
		match self {
			SelectError::NoMatchingLane { Context, .. } => Context.clone(),
			SelectError::AmbiguousMatch { Lanes, .. } => Lanes.join(", "),
		}
	}

	/// The lanes that held, when the failure was ambiguity, and an empty list
	/// otherwise.
	pub fn Lanes(&self) -> &[String] {
		match self {
			SelectError::AmbiguousMatch { Lanes, .. } => Lanes,
			_ => &[],
		}
	}
}

impl From<SelectError> for crate::Struct::Error::Error {
	/// The port's own code for the same fact.
	fn from(Why:SelectError) -> Self {
		match Why {
			SelectError::NoMatchingLane { Message, Context } => crate::Struct::Error::Error::NoMatch(Message, Context),
			SelectError::AmbiguousMatch { Message, Lanes } => {
				crate::Struct::Error::Error::CorpusInvalid(Message, Lanes.join(", "))
			},
		}
	}
}

impl fmt::Display for SelectError {
	fn fmt(&self, Formatter:&mut fmt::Formatter<'_>) -> fmt::Result {
		match self {
			SelectError::NoMatchingLane { Message, Context } => {
				write!(Formatter, "no-matching-lane: {Message} (the context was {Context})")
			},
			SelectError::AmbiguousMatch { Message, Lanes } => {
				write!(Formatter, "ambiguous-match: {Message} ({})", Lanes.join(", "))
			},
		}
	}
}

impl std::error::Error for SelectError {}

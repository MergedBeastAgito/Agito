//! Why a selection chose nothing, or chose twice.
//!
//! Selection is the one verb in this crate that can fail without anything being
//! wrong with a file: a corpus is well formed, every lane is valid, and still no
//! lane holds for this context - or two do. Neither is a defect in the corpus
//! that the loader could have found, because both depend on the values the
//! caller brought, so they are reported here rather than refused at load.
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
//! `corpus-invalid`, because two lanes that both hold for one context is a
//! corpus that disagrees with itself - the loader simply could not see it.

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
		/// The lanes that held, sorted, as the corpus names them.
		Lanes:Vec<String>,
	},
	/// A context was built from JSON that is not an object.
	NotAnObject {
		/// The sentence a human reads.
		Message:String,
		/// What the value actually was, truncated to something loggable.
		Detail:String,
	},
}

impl SelectError {
	/// No lane's condition held.
	pub fn NoMatchingLane(Message:impl Into<String>, Context:impl Into<String>) -> Self {
		SelectError::NoMatchingLane { Message:Message.into(), Context:Context.into() }
	}

	/// More than one lane's condition held.
	pub fn AmbiguousMatch(Message:impl Into<String>, Lanes:Vec<String>) -> Self {
		SelectError::AmbiguousMatch { Message:Message.into(), Lanes }
	}

	/// A context built from something that is not an object.
	pub fn NotAnObject(Message:impl Into<String>, Detail:impl Into<String>) -> Self {
		SelectError::NotAnObject { Message:Message.into(), Detail:Detail.into() }
	}

	/// The sentence a human reads.
	pub fn Message(&self) -> &str {
		match self {
			SelectError::NoMatchingLane { Message, .. }
			| SelectError::AmbiguousMatch { Message, .. }
			| SelectError::NotAnObject { Message, .. } => Message,
		}
	}

	/// What was observed, as one line: the context, the lanes, or the value.
	pub fn Detail(&self) -> String {
		match self {
			SelectError::NoMatchingLane { Context, .. } => Context.clone(),
			SelectError::AmbiguousMatch { Lanes, .. } => Lanes.join(", "),
			SelectError::NotAnObject { Detail, .. } => Detail.clone(),
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
			SelectError::NotAnObject { Message, Detail } => crate::Struct::Error::Error::CorpusInvalid(Message, Detail),
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
			SelectError::NotAnObject { Message, Detail } => {
				write!(Formatter, "not-an-object: {Message} ({Detail})")
			},
		}
	}
}

impl std::error::Error for SelectError {}

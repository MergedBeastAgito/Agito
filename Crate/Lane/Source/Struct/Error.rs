//! The seven failure modes `module.json` declares, and no others.
//!
//! One enum, one variant per code, and `detail` kept apart from the message in
//! every one of them. The split is not decoration: a caller narrowing on a code
//! should be able to log the sentence and read the observation without parsing
//! it out of the same string.
//!
//! The list is deliberately shorter than the number of things that can go
//! wrong. A missing file and a file that is not JSON are one code
//! (`corpus-unreadable`) because the caller's answer to both is the same - the
//! corpus is not there - while a corpus whose lanes name fields the observation
//! does not carry is a different fact (`corpus-invalid`): the file was read and
//! it disagrees with itself. Likewise `post-failed` covers every way the poster
//! can refuse, because a credential, a socket and a status code are the
//! adapter's vocabulary, not this capability's.

use std::fmt;

use crate::Struct::Decode::DecodeError;

/// The codes, in the order the manifest names them.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Code {
	/// A file the corpus needs is missing or is not JSON.
	CorpusUnreadable,
	/// The corpus was read and disagrees with itself.
	CorpusInvalid,
	/// A rule named a lane the corpus does not have.
	UnknownLane,
	/// Nothing matched the state and the rules name no fallback.
	NoMatch,
	/// The run has spent its ceiling.
	BudgetExhausted,
	/// The poster did not answer.
	PostFailed,
	/// A reply that is not the shape a decision model answers.
	Unparseable,
}

impl Code {
	/// The string the manifest uses.
	pub const fn Name(self) -> &'static str {
		match self {
			Code::CorpusUnreadable => "corpus-unreadable",
			Code::CorpusInvalid => "corpus-invalid",
			Code::UnknownLane => "unknown-lane",
			Code::NoMatch => "no-match",
			Code::BudgetExhausted => "budget-exhausted",
			Code::PostFailed => "post-failed",
			Code::Unparseable => "unparseable",
		}
	}

	/// Every code, so a harness can compare the manifest against the type.
	pub const fn All() -> [Code; 7] {
		[
			Code::CorpusUnreadable,
			Code::CorpusInvalid,
			Code::UnknownLane,
			Code::NoMatch,
			Code::BudgetExhausted,
			Code::PostFailed,
			Code::Unparseable,
		]
	}

	/// The code a manifest string names, if it names one of the seven.
	pub fn Parse(Value:&str) -> Option<Code> { Code::All().into_iter().find(|Code| Code.Name() == Value) }
}

/// The one error type a call fails with.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Error {
	/// A file the corpus needs is missing or is not JSON.
	CorpusUnreadable {
		/// The sentence a human reads.
		Message:String,
		/// What was observed, kept apart from the sentence.
		Detail:String,
	},
	/// The corpus was read and disagrees with itself.
	CorpusInvalid {
		/// The sentence a human reads.
		Message:String,
		/// What the file actually said.
		Detail:String,
	},
	/// A rule named a lane the corpus does not have.
	UnknownLane {
		/// The sentence a human reads.
		Message:String,
		/// The lanes the corpus does have.
		Detail:String,
	},
	/// Nothing matched the state and the rules name no fallback.
	NoMatch {
		/// The sentence a human reads.
		Message:String,
		/// What was in the state when nothing matched.
		Detail:String,
	},
	/// The run has spent its ceiling.
	BudgetExhausted {
		/// The sentence a human reads.
		Message:String,
		/// The meter's own line, which is what a report would print anyway.
		Detail:String,
	},
	/// The poster did not answer.
	PostFailed {
		/// The sentence a human reads.
		Message:String,
		/// What the poster observed.
		Detail:String,
	},
	/// A reply that is not the shape a decision model answers.
	Unparseable {
		/// The sentence a human reads.
		Message:String,
		/// What the body actually held, truncated to something loggable.
		Detail:String,
		/// Why the decoder refused it, when the refusal came from the decoder.
		/// `None` for a refusal this crate states itself, so a caller can tell
		/// "the decoder has a specific reason" from "the port has none".
		Cause:Option<DecodeError>,
	},
}

impl Error {
	/// A corpus that could not be read.
	pub fn CorpusUnreadable(Message:impl Into<String>, Detail:impl Into<String>) -> Self {
		Error::CorpusUnreadable { Message:Message.into(), Detail:Detail.into() }
	}

	/// A corpus that disagrees with itself.
	pub fn CorpusInvalid(Message:impl Into<String>, Detail:impl Into<String>) -> Self {
		Error::CorpusInvalid { Message:Message.into(), Detail:Detail.into() }
	}

	/// A rule naming a lane the corpus does not have.
	pub fn UnknownLane(Message:impl Into<String>, Detail:impl Into<String>) -> Self {
		Error::UnknownLane { Message:Message.into(), Detail:Detail.into() }
	}

	/// Nothing matched and there is no fallback.
	pub fn NoMatch(Message:impl Into<String>, Detail:impl Into<String>) -> Self {
		Error::NoMatch { Message:Message.into(), Detail:Detail.into() }
	}

	/// The ceiling has been reached.
	pub fn BudgetExhausted(Message:impl Into<String>, Detail:impl Into<String>) -> Self {
		Error::BudgetExhausted { Message:Message.into(), Detail:Detail.into() }
	}

	/// The poster did not answer.
	pub fn PostFailed(Message:impl Into<String>, Detail:impl Into<String>) -> Self {
		Error::PostFailed { Message:Message.into(), Detail:Detail.into() }
	}

	/// A reply that is not an answer.
	pub fn Unparseable(Message:impl Into<String>, Detail:impl Into<String>) -> Self {
		Error::Unparseable { Message:Message.into(), Detail:Detail.into(), Cause:None }
	}

	/// A reply the decoder refused, with the decoder's own reason kept.
	///
	/// The sentence and the detail are the decoder's, so a log line reads the
	/// same whether the failure was read through this type or through
	/// [`crate::Struct::Decode::DecodeError`] directly - and the reason stays a
	/// value, not a string a caller would have to parse back out.
	pub fn Decode(Why:DecodeError) -> Self {
		Error::Unparseable { Message:Why.Sentence(), Detail:Why.Detail(), Cause:Some(Why) }
	}

	/// The decoder's own reason, when the refusal came from the decoder.
	pub fn Cause(&self) -> Option<&DecodeError> {
		match self {
			Error::Unparseable { Cause, .. } => Cause.as_ref(),
			_ => None,
		}
	}

	/// The code this failure carries.
	pub const fn Code(&self) -> Code {
		match self {
			Error::CorpusUnreadable { .. } => Code::CorpusUnreadable,
			Error::CorpusInvalid { .. } => Code::CorpusInvalid,
			Error::UnknownLane { .. } => Code::UnknownLane,
			Error::NoMatch { .. } => Code::NoMatch,
			Error::BudgetExhausted { .. } => Code::BudgetExhausted,
			Error::PostFailed { .. } => Code::PostFailed,
			Error::Unparseable { .. } => Code::Unparseable,
		}
	}

	/// The sentence a human reads.
	pub fn Message(&self) -> &str {
		match self {
			Error::CorpusUnreadable { Message, .. }
			| Error::CorpusInvalid { Message, .. }
			| Error::UnknownLane { Message, .. }
			| Error::NoMatch { Message, .. }
			| Error::BudgetExhausted { Message, .. }
			| Error::PostFailed { Message, .. }
			| Error::Unparseable { Message, .. } => Message,
		}
	}

	/// What was observed, which may be empty.
	pub fn Detail(&self) -> &str {
		match self {
			Error::CorpusUnreadable { Detail, .. }
			| Error::CorpusInvalid { Detail, .. }
			| Error::UnknownLane { Detail, .. }
			| Error::NoMatch { Detail, .. }
			| Error::BudgetExhausted { Detail, .. }
			| Error::PostFailed { Detail, .. }
			| Error::Unparseable { Detail, .. } => Detail,
		}
	}

	/// The code and the sentence as one word, which is what a trace or a meter
	/// records: `post-failed: no token`.
	pub fn Why(&self) -> String { format!("{}: {}", self.Code().Name(), self.Message()) }
}

impl fmt::Display for Error {
	fn fmt(&self, Formatter:&mut fmt::Formatter<'_>) -> fmt::Result {
		write!(Formatter, "{}: {}", self.Code().Name(), self.Message())
	}
}

impl std::error::Error for Error {}

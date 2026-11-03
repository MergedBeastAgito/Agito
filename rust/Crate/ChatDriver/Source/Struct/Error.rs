//! The four failure modes `module.json` declares, and no others.
//!
//! One enum, one variant per code, `detail` kept apart from the message in every
//! one of them.

use std::fmt;

/// The codes, in the order the manifest names them.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Code {
	/// The page could not answer.
	NotReady,
	/// The composer would not enter search mode.
	WrongMode,
	/// The page is set to a different model.
	WrongModel,
	/// The answer is collapsed behind a control.
	Truncated,
}

impl Code {
	/// The string the manifest uses.
	pub const fn Name(self) -> &'static str {
		match self {
			Code::NotReady => "not-ready",
			Code::WrongMode => "wrong-mode",
			Code::WrongModel => "wrong-model",
			Code::Truncated => "truncated",
		}
	}

	/// Every code, so a harness can compare the manifest against the type.
	pub const fn All() -> [Code; 4] {
		[
			Code::NotReady,
			Code::WrongMode,
			Code::WrongModel,
			Code::Truncated,
		]
	}

	/// The code a manifest string names, if it names one of the four.
	pub fn Parse(Value: &str) -> Option<Code> {
		Code::All().into_iter().find(|Code| Code.Name() == Value)
	}
}

/// The one error type an `Ask` call fails with.
///
/// `NotReady` is the one code that covers a provider's failures as well, because
/// the manifest has no fifth code and "the page could not answer" is what a dead
/// transport, a navigation failure and a slow page all mean to this capability.
/// The cause is carried in `detail` rather than flattened, so the difference
/// survives.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Error {
	/// The page is not in a state where it can answer.
	NotReady {
		/// The sentence a human reads.
		Message: String,
		/// What was observed, kept apart from the sentence.
		Detail: String,
	},
	/// The composer would not enter search mode, so nothing was sent.
	WrongMode {
		/// The sentence a human reads.
		Message: String,
		/// What was observed.
		Detail: String,
	},
	/// The page is set to a different model, so nothing was sent.
	WrongModel {
		/// The sentence a human reads.
		Message: String,
		/// What was observed.
		Detail: String,
	},
	/// The answer is collapsed, so the text in the DOM is only part of it.
	Truncated {
		/// The sentence a human reads.
		Message: String,
		/// What was observed.
		Detail: String,
	},
}

impl Error {
	/// A page that could not answer.
	pub fn NotReady(Message: impl Into<String>, Detail: impl Into<String>) -> Self {
		Error::NotReady {
			Message: Message.into(),
			Detail: Detail.into(),
		}
	}

	/// A composer that would not enter search mode.
	pub fn WrongMode(Message: impl Into<String>, Detail: impl Into<String>) -> Self {
		Error::WrongMode {
			Message: Message.into(),
			Detail: Detail.into(),
		}
	}

	/// A page set to a different model.
	pub fn WrongModel(Message: impl Into<String>, Detail: impl Into<String>) -> Self {
		Error::WrongModel {
			Message: Message.into(),
			Detail: Detail.into(),
		}
	}

	/// An answer that is only part of itself.
	pub fn Truncated(Message: impl Into<String>, Detail: impl Into<String>) -> Self {
		Error::Truncated {
			Message: Message.into(),
			Detail: Detail.into(),
		}
	}

	/// The code this failure carries.
	pub const fn Code(&self) -> Code {
		match self {
			Error::NotReady { .. } => Code::NotReady,
			Error::WrongMode { .. } => Code::WrongMode,
			Error::WrongModel { .. } => Code::WrongModel,
			Error::Truncated { .. } => Code::Truncated,
		}
	}

	/// The sentence a human reads.
	pub fn Message(&self) -> &str {
		match self {
			Error::NotReady { Message, .. }
			| Error::WrongMode { Message, .. }
			| Error::WrongModel { Message, .. }
			| Error::Truncated { Message, .. } => Message,
		}
	}

	/// What was observed, which may be empty.
	pub fn Detail(&self) -> &str {
		match self {
			Error::NotReady { Detail, .. }
			| Error::WrongMode { Detail, .. }
			| Error::WrongModel { Detail, .. }
			| Error::Truncated { Detail, .. } => Detail,
		}
	}
}

impl fmt::Display for Error {
	fn fmt(&self, Formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
		write!(Formatter, "{}: {}", self.Code().Name(), self.Message())
	}
}

impl std::error::Error for Error {}

//! The three failure modes `module.json` declares, and no others.
//!
//! One enum, one variant per code, `detail` kept apart from the message in
//! every one of them: what was observed is the only thing that says which part
//! of the transport broke, and a sentence written for a human loses it.

use std::fmt;

/// The codes, in the order the manifest names them.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Code {
	/// The transport is gone: a dead socket, not a slow page.
	Unavailable,
	/// The page exists but could not be used: it failed to load, or it threw.
	Navigation,
	/// The signal fired or the deadline passed before an answer arrived.
	Timeout,
}

impl Code {
	/// The string the manifest uses.
	pub const fn Name(self) -> &'static str {
		match self {
			Code::Unavailable => "browser-unavailable",
			Code::Navigation => "navigation-failed",
			Code::Timeout => "timeout",
		}
	}

	/// Every code, so a harness can compare the manifest against the type.
	pub const fn All() -> [Code; 3] { [Code::Unavailable, Code::Navigation, Code::Timeout] }

	/// The code a manifest string names, if it names one of the three.
	pub fn Parse(Value:&str) -> Option<Code> { Code::All().into_iter().find(|Code| Code.Name() == Value) }
}

/// The one error type a `Browse` call fails with.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Error {
	/// The transport is gone.
	Unavailable {
		/// The sentence a human reads.
		Message:String,
		/// What was observed, kept apart from the sentence.
		Detail:String,
	},
	/// The page exists but could not be used.
	Navigation {
		/// The sentence a human reads.
		Message:String,
		/// What was observed.
		Detail:String,
	},
	/// The signal fired or the deadline passed.
	Timeout {
		/// The sentence a human reads.
		Message:String,
		/// What was observed.
		Detail:String,
	},
}

impl Error {
	/// A dead transport.
	pub fn Unavailable(Message:impl Into<String>, Detail:impl Into<String>) -> Self {
		Error::Unavailable { Message:Message.into(), Detail:Detail.into() }
	}

	/// A page that could not be used.
	pub fn Navigation(Message:impl Into<String>, Detail:impl Into<String>) -> Self {
		Error::Navigation { Message:Message.into(), Detail:Detail.into() }
	}

	/// A deadline that passed or a caller that gave up.
	pub fn Timeout(Message:impl Into<String>, Detail:impl Into<String>) -> Self {
		Error::Timeout { Message:Message.into(), Detail:Detail.into() }
	}

	/// The code this failure carries.
	pub const fn Code(&self) -> Code {
		match self {
			Error::Unavailable { .. } => Code::Unavailable,
			Error::Navigation { .. } => Code::Navigation,
			Error::Timeout { .. } => Code::Timeout,
		}
	}

	/// The sentence a human reads.
	pub fn Message(&self) -> &str {
		match self {
			Error::Unavailable { Message, .. } | Error::Navigation { Message, .. } | Error::Timeout { Message, .. } => {
				Message
			},
		}
	}

	/// What was observed, which may be empty.
	pub fn Detail(&self) -> &str {
		match self {
			Error::Unavailable { Detail, .. } | Error::Navigation { Detail, .. } | Error::Timeout { Detail, .. } => {
				Detail
			},
		}
	}
}

impl fmt::Display for Error {
	fn fmt(&self, Formatter:&mut fmt::Formatter<'_>) -> fmt::Result {
		write!(Formatter, "{}: {}", self.Code().Name(), self.Message())
	}
}

impl std::error::Error for Error {}

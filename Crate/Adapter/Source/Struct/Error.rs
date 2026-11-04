//! One failure an adapter reports, in the adapter's own vocabulary.
//!
//! Three parts, kept apart the way `agentic-runtime`'s error keeps them: a
//! consumer narrows on [`Error::Code`], a human reads [`Error::Message`], and a
//! caller that needs to know what was actually observed reads
//! [`Error::Detail`] instead of parsing a sentence.
//!
//! The code is a string rather than an enum this crate defines, and that is the
//! whole design: a code names a substrate failure - a dead socket, a page that
//! never answered, a game that stopped - and Agentic cannot enumerate the
//! failures of a substrate it has never seen. An adapter declares its codes in
//! its own `module.json`, and reports one of them here.

use std::fmt;

/// One failure, carrying the code the adapter's manifest declares.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Error {
	/// The code, spelled exactly as the adapter's `module.json` spells it.
	pub Code:&'static str,
	/// The sentence a human reads.
	pub Message:String,
	/// What was observed, which may be empty.
	pub Detail:String,
}

impl Error {
	/// A failure with nothing observed to add.
	pub fn New(Code:&'static str, Message:impl Into<String>) -> Self {
		Self { Code, Message:Message.into(), Detail:String::new() }
	}

	/// The same failure, with what was observed attached.
	pub fn WithDetail(self, Detail:impl Into<String>) -> Self { Self { Detail:Detail.into(), ..self } }

	/// The code the manifest declares.
	pub fn Code(&self) -> &'static str { self.Code }

	/// The sentence a human reads.
	pub fn Message(&self) -> &str { &self.Message }

	/// What was observed, which may be empty.
	pub fn Detail(&self) -> &str { &self.Detail }
}

impl fmt::Display for Error {
	fn fmt(&self, Formatter:&mut fmt::Formatter<'_>) -> fmt::Result {
		if self.Detail.is_empty() {
			return write!(Formatter, "{}: {}", self.Code, self.Message);
		}
		write!(Formatter, "{}: {} ({})", self.Code, self.Message, self.Detail)
	}
}

impl std::error::Error for Error {}

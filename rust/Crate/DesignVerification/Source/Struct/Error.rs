//! The three failure modes `module.json` declares, and no others.
//!
//! One enum, one variant per code, `detail` kept apart from the message in
//! every one of them.

use std::fmt;

use crate::Struct::Port::Verdict;

/// The codes, in the order the manifest names them.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Code {
	/// An element the design names is not in the page.
	Absent,
	/// An invariant did not hold.
	Broke,
	/// The page was never read.
	NotReady,
}

impl Code {
	/// The string the manifest uses.
	pub const fn Name(self) -> &'static str {
		match self {
			Code::Absent => "absent-element",
			Code::Broke => "invariant-broke",
			Code::NotReady => "not-ready",
		}
	}

	/// Every code, so a harness can compare the manifest against the type.
	pub const fn All() -> [Code; 3] { [Code::Absent, Code::Broke, Code::NotReady] }

	/// The code a manifest string names, if it names one of the three.
	pub fn Parse(Value:&str) -> Option<Code> { Code::All().into_iter().find(|Code| Code.Name() == Value) }
}

/// The one error type this capability has.
///
/// `NotReady` is the only code `Measure` fails with, because the manifest has
/// no fourth code and "the page was never read" is what a dead transport, a
/// failure mid-measurement, a passed deadline and a cancellation all mean to
/// this capability. `detail` says which one happened.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Error {
	/// An element the design names is not in the page.
	Absent {
		/// The sentence a human reads.
		Message:String,
		/// The selectors, kept apart from the sentence.
		Detail:String,
	},
	/// An invariant did not hold.
	Broke {
		/// The sentence a human reads.
		Message:String,
		/// The invariants that broke.
		Detail:String,
	},
	/// The page was never read.
	NotReady {
		/// The sentence a human reads.
		Message:String,
		/// What was observed.
		Detail:String,
	},
}

impl Error {
	/// A page that was never read.
	pub fn NotReady(Message:impl Into<String>, Detail:impl Into<String>) -> Self {
		Error::NotReady { Message:Message.into(), Detail:Detail.into() }
	}

	/// A verdict as the error the manifest names, or `None` when it held.
	///
	/// `Check` never fails, because a verdict is a result. A caller that must
	/// fail loudly - a command that exits non-zero - asks for the error
	/// here. Absence is reported before a broken invariant, because a page
	/// that lost a part is the more actionable failure and the rows that
	/// read through it are consequences of it.
	pub fn From(Verdict:&Verdict) -> Option<Error> {
		if !Verdict.Absent.is_empty() {
			return Some(Error::Absent {
				Message:format!("{} element(s) the design names are absent", Verdict.Absent.len()),
				Detail:Verdict.Absent.join(", "),
			});
		}
		let Broken:Vec<&str> = Verdict
			.Evidence
			.iter()
			.filter(|Row| !Row.Held)
			.map(|Row| Row.Label.as_str())
			.collect();
		if !Broken.is_empty() {
			return Some(Error::Broke {
				Message:format!("{} invariant(s) broke", Broken.len()),
				Detail:Broken.join(", "),
			});
		}
		None
	}

	/// The code this failure carries.
	pub const fn Code(&self) -> Code {
		match self {
			Error::Absent { .. } => Code::Absent,
			Error::Broke { .. } => Code::Broke,
			Error::NotReady { .. } => Code::NotReady,
		}
	}

	/// The sentence a human reads.
	pub fn Message(&self) -> &str {
		match self {
			Error::Absent { Message, .. } | Error::Broke { Message, .. } | Error::NotReady { Message, .. } => Message,
		}
	}

	/// What was observed, which may be empty.
	pub fn Detail(&self) -> &str {
		match self {
			Error::Absent { Detail, .. } | Error::Broke { Detail, .. } | Error::NotReady { Detail, .. } => Detail,
		}
	}
}

impl fmt::Display for Error {
	fn fmt(&self, Formatter:&mut fmt::Formatter<'_>) -> fmt::Result {
		write!(Formatter, "{}: {}", self.Code().Name(), self.Message())
	}
}

impl std::error::Error for Error {}

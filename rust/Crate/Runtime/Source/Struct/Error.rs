//! The three failure modes `module.json` declares, and no others.
//!
//! One enum, one variant per code, and `detail` kept apart from the message in
//! every one of them: a consumer narrows on the variant, a human reads the
//! message, and a caller that needs to know what was actually observed reads
//! `detail` instead of parsing a sentence.

use std::fmt;

/// The codes, in the order the manifest names them.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Code {
	/// The request may not proceed: it is malformed, or its authority ended.
	AdmissionRefused,
	/// The resource is held under a newer fence, or an effect is still in it.
	LeaseHeld,
	/// The run's own allowance is spent: its deadline passed or its count ran
	/// out.
	BudgetExhausted,
}

impl Code {
	/// The string the manifest uses.
	pub const fn Name(self) -> &'static str {
		match self {
			Code::AdmissionRefused => "admission-refused",
			Code::LeaseHeld => "lease-held",
			Code::BudgetExhausted => "budget-exhausted",
		}
	}

	/// Every code, so a harness can compare the manifest against the type.
	pub const fn All() -> [Code; 3] { [Code::AdmissionRefused, Code::LeaseHeld, Code::BudgetExhausted] }

	/// The code a manifest string names, if it names one of the three.
	pub fn Parse(Value:&str) -> Option<Code> { Code::All().into_iter().find(|Code| Code.Name() == Value) }
}

/// The one error type `claim`, `renew` and `withLease` reject with.
///
/// `admit` does not return one - its port answers a refusal - so a caller has
/// two shapes to handle and only one of them is an error. That asymmetry is the
/// port's: admission answers a question, while a claim or a write either
/// happened or did not.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Error {
	/// The request may not proceed.
	AdmissionRefused {
		/// The sentence a human reads.
		Message:String,
		/// What was observed, kept apart from the sentence.
		Detail:String,
	},
	/// The resource is held, or an effect is still in it.
	LeaseHeld {
		/// The sentence a human reads.
		Message:String,
		/// What was observed.
		Detail:String,
	},
	/// The run's allowance is spent.
	BudgetExhausted {
		/// The sentence a human reads.
		Message:String,
		/// What was observed.
		Detail:String,
	},
}

impl Error {
	/// A refusal with nothing observed to add.
	pub fn Refused(Message:impl Into<String>) -> Self {
		Error::AdmissionRefused { Message:Message.into(), Detail:String::new() }
	}

	/// A held resource with nothing observed to add.
	pub fn Held(Message:impl Into<String>) -> Self { Error::LeaseHeld { Message:Message.into(), Detail:String::new() } }

	/// A spent allowance with nothing observed to add.
	pub fn Spent(Message:impl Into<String>) -> Self {
		Error::BudgetExhausted { Message:Message.into(), Detail:String::new() }
	}

	/// The same failure, with what was observed attached.
	pub fn WithDetail(self, Detail:impl Into<String>) -> Self {
		let Detail = Detail.into();
		match self {
			Error::AdmissionRefused { Message, .. } => Error::AdmissionRefused { Message, Detail },
			Error::LeaseHeld { Message, .. } => Error::LeaseHeld { Message, Detail },
			Error::BudgetExhausted { Message, .. } => Error::BudgetExhausted { Message, Detail },
		}
	}

	/// The code this failure carries.
	pub const fn Code(&self) -> Code {
		match self {
			Error::AdmissionRefused { .. } => Code::AdmissionRefused,
			Error::LeaseHeld { .. } => Code::LeaseHeld,
			Error::BudgetExhausted { .. } => Code::BudgetExhausted,
		}
	}

	/// The sentence a human reads.
	pub fn Message(&self) -> &str {
		match self {
			Error::AdmissionRefused { Message, .. }
			| Error::LeaseHeld { Message, .. }
			| Error::BudgetExhausted { Message, .. } => Message,
		}
	}

	/// What was observed, which may be empty.
	pub fn Detail(&self) -> &str {
		match self {
			Error::AdmissionRefused { Detail, .. }
			| Error::LeaseHeld { Detail, .. }
			| Error::BudgetExhausted { Detail, .. } => Detail,
		}
	}
}

impl fmt::Display for Error {
	fn fmt(&self, Formatter:&mut fmt::Formatter<'_>) -> fmt::Result {
		write!(Formatter, "{}: {}", self.Code().Name(), self.Message())
	}
}

impl std::error::Error for Error {}

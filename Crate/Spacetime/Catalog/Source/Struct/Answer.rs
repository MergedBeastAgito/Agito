//! What a rule answers.
//!
//! A rule answers rather than throws, and the answer is a value: a refusal is
//! something a caller handles and a test asserts, so no malformed argument can
//! take down the reducer that received it.
//!
//! [`ProbeAnswer`] exists because "returns only what it wrote" needs the row to
//! be returnable. The observed branch is not a wider answer to the same
//! question: it is the reveal itself, and the caller that receives it is the
//! one the row names.

use crate::Struct::{Code::Code, Row::Row};

/// The answer a rule that writes nothing of its own gives.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Answer {
	/// The rule accepted the request.
	Accepted,
	/// The rule refused it, and wrote nothing.
	Refused(Code),
}

impl Answer {
	/// Whether the rule accepted the request, which is `result.ok` in the
	/// source this was ported from.
	pub fn Ok(&self) -> bool { matches!(self, Answer::Accepted) }

	/// The reason it refused, or `None` when it accepted.
	pub fn Code(&self) -> Option<Code> {
		match self {
			Answer::Accepted => None,
			Answer::Refused(Code) => Some(*Code),
		}
	}
}

/// What a reveal answers: the row it wrote, or the reason it wrote none.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ProbeAnswer {
	/// The observation the rule wrote, which is the whole of what it disclosed.
	Observed(Row),
	/// The rule refused, and wrote nothing.
	Refused(Code),
}

impl ProbeAnswer {
	/// Whether the reveal happened.
	pub fn Ok(&self) -> bool { matches!(self, ProbeAnswer::Observed(_)) }

	/// The row it wrote, or `None` when it refused.
	pub fn Row(&self) -> Option<&Row> {
		match self {
			ProbeAnswer::Observed(Row) => Some(Row),
			ProbeAnswer::Refused(_) => None,
		}
	}

	/// The reason it refused, or `None` when it observed.
	pub fn Code(&self) -> Option<Code> {
		match self {
			ProbeAnswer::Observed(_) => None,
			ProbeAnswer::Refused(Code) => Some(*Code),
		}
	}
}

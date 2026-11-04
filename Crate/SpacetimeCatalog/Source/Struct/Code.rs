//! The three reasons a rule refuses, and the only three it may give.
//!
//! They mirror `provides.errors` in `module.json`, which is what a harness
//! reads; the workspace's readiness check compares the two, so the manifest
//! cannot drift from the code.
//!
//! Three reasons, and each has exactly one source:
//!
//! - `admission-refused` - the request may not proceed: it is malformed, or the
//!   authority it presents has ended. A rule that could not tell a caller which
//!   of several things was wrong says this, on purpose: the caller learns that
//!   its write did not land, not a list of conditions.
//! - `lease-held` - the resource is held, or quarantined. A live lease is not a
//!   queue, because a queue in the reducer would be a second source of
//!   ownership.
//! - `budget-exhausted` - the reveal was already made to this audience at this
//!   version. The probe row is its own receipt, so the second probe finds it.

use std::fmt;

/// The codes, in the order the manifest names them.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Code {
	/// The request may not proceed.
	AdmissionRefused,
	/// The resource is held, or quarantined.
	LeaseHeld,
	/// The reveal was already made to this audience at this version.
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

impl fmt::Display for Code {
	fn fmt(&self, Formatter:&mut fmt::Formatter<'_>) -> fmt::Result { Formatter.write_str(self.Name()) }
}

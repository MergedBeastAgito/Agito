//! The port `agito.verify.design` promises, written the way its consumer asks
//! for it.
//!
//! `module.json` names three errors - `absent-element`, `invariant-broke` and
//! `not-ready` - and [`crate::Struct::Error`] is where they live. A caller
//! narrows on the variant; anything a provider does that is not one of the
//! three is mapped onto one of them at the boundary.
//!
//! The port states more than data shapes, because two implementations that
//! agreed on the shapes alone would still disagree about what a failure is:
//!
//! - **An absent element is a failure, never a precondition.** `Check` reports
//!   the selector in `Absent` and answers `Held: false`. It never fails, and it
//!   never reports "could not run": the run happened, and the page was wrong.
//! - **This package holds no design.** No selector, no pixel value, no colour.
//! - **`Measure` never fails for a missing element.** A selector that matches
//!   nothing is a measurement with `Present: false`, because "there is nothing
//!   there" is an observation. A dead transport is the only failure, and it is
//!   `not-ready`, so the two answers cannot be confused.
//! - **Evidence is per invariant.** One entry per invariant, in the order the
//!   caller gave them, so a failure names which invariant broke and what it
//!   read.

use ::RunTime::{Call, FutureBox};
use serde_json::Value;

/// One element, as the page answered for it.
///
/// The type exists to keep "not observed" apart from a real zero, `false` or
/// empty string: a measurement engine that cannot say "there is nothing there"
/// produces evidence that reads as a measurement.
#[derive(Clone, Debug, PartialEq)]
pub struct Measurement {
	/// The selector this was asked about, as the caller wrote it.
	pub Selector:String,
	/// Whether the element was there. `false` is an observation, not an error.
	pub Present:bool,
	/// What was read from it - a width, a count, a label - or `Null`.
	///
	/// An absent element always carries `Null`, so a stale number cannot travel
	/// with `Present: false` and be read by an invariant that forgot to check.
	/// A present element that read `Null` is a value, not an absence: the
	/// element was there and had nothing to report.
	pub Value:Value,
}

impl Measurement {
	/// A measurement of an element that was there.
	pub fn Present(Selector:impl Into<String>, Value:Value) -> Self {
		Self { Selector:Selector.into(), Present:true, Value }
	}

	/// A measurement of an element that was not there. It cannot carry a value.
	pub fn Absent(Selector:impl Into<String>) -> Self {
		Self { Selector:Selector.into(), Present:false, Value:Value::Null }
	}
}

/// A reading of the measurements, or why it could not be read.
///
/// A reading that fails - because it dereferenced a measurement it assumed was
/// there - is a failed invariant, not a blocked run.
pub type Read = Box<dyn Fn(&[Measurement]) -> Result<Value, String> + Send + Sync>;

/// A derivation of an expectation from the measurements themselves.
pub type Derive = Box<dyn Fn(&[Measurement]) -> Result<Value, String> + Send + Sync>;

/// A comparison of a reading against an expectation.
pub type Test = Box<dyn Fn(&Value, &Value) -> bool + Send + Sync>;

/// What an invariant's expectation is.
///
/// Either a value, compared for equality, or a function of the measurements
/// when the expectation is derived from the page itself - "the rail sits level
/// with the drawer" wants the drawer's own top, and no constant states it.
pub enum Want {
	/// A value the reading must equal.
	Value(Value),
	/// A reading derived from the measurements. A derivation that fails counts
	/// as not held, and the row records why.
	Derive(Derive),
}

impl std::fmt::Debug for Want {
	fn fmt(&self, Formatter:&mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		match self {
			Want::Value(Value) => Formatter.debug_tuple("Value").field(Value).finish(),
			Want::Derive(_) => Formatter.write_str("Derive(..)"),
		}
	}
}

/// One thing the design says must be true of the page.
pub struct Invariant {
	/// What the invariant is about, as a reader of a failure should hear it.
	pub Label:String,
	/// What to read out of the measurements.
	pub Read:Read,
	/// What the reading must equal, or how it is derived.
	pub Want:Want,
	/// How to compare, for an invariant that is a relation rather than an
	/// equality. Absent means equality; a comparator that panics counts as
	/// not held, because a comparison that could not be made has not been
	/// satisfied.
	pub Test:Option<Test>,
}

impl std::fmt::Debug for Invariant {
	fn fmt(&self, Formatter:&mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		Formatter
			.debug_struct("Invariant")
			.field("Label", &self.Label)
			.field("Want", &self.Want)
			.finish_non_exhaustive()
	}
}

impl Invariant {
	/// An invariant that reads one value and requires it to equal another.
	pub fn Equal(
		Label:impl Into<String>,
		Read:impl Fn(&[Measurement]) -> Result<Value, String> + Send + Sync + 'static,
		Want:Value,
	) -> Self {
		Self { Label:Label.into(), Read:Box::new(Read), Want:Want::Value(Want), Test:None }
	}

	/// An invariant whose expectation is derived from the measurements.
	pub fn Derived(
		Label:impl Into<String>,
		Read:impl Fn(&[Measurement]) -> Result<Value, String> + Send + Sync + 'static,
		Derive:impl Fn(&[Measurement]) -> Result<Value, String> + Send + Sync + 'static,
	) -> Self {
		Self {
			Label:Label.into(),
			Read:Box::new(Read),
			Want:Want::Derive(Box::new(Derive)),
			Test:None,
		}
	}

	/// The same invariant, compared by the caller's own relation.
	pub fn Tested(mut self, Test:impl Fn(&Value, &Value) -> bool + Send + Sync + 'static) -> Self {
		self.Test = Some(Box::new(Test));
		self
	}
}

/// One side of a comparison: what was read, or why it could not be.
///
/// A reading that failed is recorded as `Threw`, which cannot be mistaken for a
/// value it read - the Rust port's version of the TypeScript `{ threw: message
/// }`.
#[derive(Clone, Debug, PartialEq)]
pub enum Outcome {
	/// A value that was read, or an expectation that was derived.
	Value(Value),
	/// Why it could not be read or derived.
	Threw(String),
}

impl Outcome {
	/// Whether this side produced a value.
	pub fn Held(&self) -> bool { matches!(self, Outcome::Value(_)) }
}

/// One invariant's row in a verdict: what it read, what it wanted, and whether.
#[derive(Clone, Debug, PartialEq)]
pub struct Evidence {
	/// The invariant's own label, copied unchanged.
	pub Label:String,
	/// What the invariant read.
	pub Got:Outcome,
	/// What it was compared against.
	pub Want:Outcome,
	/// Whether the comparison held.
	pub Held:bool,
}

/// The whole answer to one run.
///
/// `Held` is true only when nothing was absent and every invariant held, so a
/// caller can branch on one field and still read the reason from the others.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Verdict {
	/// Every invariant held and every selector matched something.
	pub Held:bool,
	/// One row per invariant, in the order the caller gave them.
	pub Evidence:Vec<Evidence>,
	/// Every selector that was observed to match nothing, once each.
	pub Absent:Vec<String>,
}

/// What a transport reports when it fails.
///
/// A transport is not this capability, so it does not speak this capability's
/// three codes. It says what it observed, and the capability maps that onto
/// `not-ready` with the observation in `detail`.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Failure {
	/// The sentence a human reads.
	pub Message:String,
	/// What was observed.
	pub Detail:String,
}

impl Failure {
	/// A failure with a sentence and nothing observed.
	pub fn Of(Message:impl Into<String>) -> Self { Self { Message:Message.into(), Detail:String::new() } }
}

/// The browser as this capability needs it: read every named element.
///
/// The transport answers the *wire* shape - one JSON object per selector,
/// carrying `selector`, `present` and `value` - and the capability validates
/// it. That is the TypeScript port's arrangement, kept because it is what makes
/// the classification checkable: a provider is not this crate, so a provider
/// that answers a list of numbers, an object where a number belongs, or no
/// answer for a selector it was asked about is refused with `not-ready` rather
/// than trusted.
///
/// What the port demands of an implementation:
///
/// - one entry per selector it was asked about, carrying that selector, so the
///   capability can match by `Selector` rather than by position;
/// - a selector that matches nothing is `present: false` - an observation, not
///   a failure. A provider that fails because an element is missing turns a
///   page defect into a failure of the observer;
/// - `value` is a number, a string or `null`. Anything else is a provider that
///   has not finished serialising what it read, and it is refused rather than
///   flattened, because flattening it to `null` would read as "nothing to
///   report";
/// - a failure means the page was never read, and it is reported as
///   `not-ready`.
pub trait Browse: Send + Sync {
	/// Read every named element, or report that it is not in the page.
	fn Measure<'a>(&'a self, Selectors:&'a [String], Call:Call) -> FutureBox<'a, Result<Vec<Value>, Failure>>;
}

/// The one port this capability offers.
pub trait Verify: Send + Sync {
	/// Read the named elements from the page, one measurement per selector.
	fn Measure<'a>(
		&'a self,
		Selectors:&'a [String],
		Call:Call,
	) -> FutureBox<'a, Result<Vec<Measurement>, crate::Struct::Error::Error>>;
	/// Judge a set of measurements against a set of invariants. Pure, and it
	/// never fails: a verdict is a result, not an exception.
	fn Check(&self, Invariants:&[Invariant], Measurements:&[Measurement]) -> Verdict;
}

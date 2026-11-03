//! `agentic.verify.design` over a transport the composition root supplies.
//!
//! Two verbs, and only the first touches a page:
//!
//! 1. **`Measure` asks and bounds.** The selectors go to the transport as they were
//!    given, the answer is matched back by selector, and the deadline is enforced
//!    on this side as well as the provider's - a transport that ignores its budget
//!    must not be able to hang a caller. The one failure is `not-ready`: the page
//!    was never read. An element that is not in the page is a measurement, never a
//!    failure.
//! 2. **`Check` is pure.** It reads no page and keeps no state, so the same
//!    measurements answer the same verdict twice. It cannot fail either: a reading
//!    that fails, an expectation that fails and a comparator that panics are each
//!    recorded as a row that did not hold, because a page defect must not be able
//!    to end a run as an exception.
//!
//! Nothing here names a selector, a length or a colour. The transport supplies what
//! a page says; the caller supplies what the design expects.

use std::sync::Arc;

use AgenticRuntime::{Bound, Call, Expired, FutureBox, Pace};

use crate::Fn::Judge::Check;
use crate::Fn::Measure::Reconcile;
use crate::Struct::Error::Error;
use crate::Struct::Port::{Browse, Failure, Invariant, Measurement, Verdict, Verify};

/// A `Verify` over one transport.
///
/// Nothing here opens a browser or reads an environment variable: the transport is
/// the composition root's to supply, and the fake in `Test/` is the proof that the
/// rules hold without one.
pub struct Verifier {
	Browse: Arc<dyn Browse>,
	Pace: Arc<dyn Pace>,
}

/// Build a `Verify` over one transport, on the pace the caller supplied.
pub fn Create(Browse: Arc<dyn Browse>, Pace: Arc<dyn Pace>) -> Verifier {
	Verifier { Browse, Pace }
}

impl Verify for Verifier {
	fn Measure<'a>(
		&'a self,
		Selectors: &'a [String],
		Call: Call,
	) -> FutureBox<'a, Result<Vec<Measurement>, Error>> {
		Box::pin(async move {
			// Nothing asked is nothing read: a no-op request must not touch a page.
			if Selectors.is_empty() {
				return Ok(Vec::new());
			}
			if Call.Cancelled() {
				return Err(Error::NotReady(
					"measure was cancelled before it started",
					"the caller's signal had already fired",
				));
			}
			if Call.Expired(self.Pace.as_ref()) {
				return Err(Error::NotReady(
					"measure exceeded its deadline",
					"the transport was still working when the budget ran out",
				));
			}
			let Work = self.Browse.Measure(Selectors, Call.clone());
			let Answered = match Bound(Work, &Call, self.Pace.as_ref()).await {
				Ok(Ok(Answered)) => Answered,
				Ok(Err(Failure)) => {
					return Err(Error::NotReady(
						"the transport failed while measuring the page",
						Failure.Message,
					));
				}
				Err(Expired::Timeout) => {
					return Err(Error::NotReady(
						"measure exceeded its deadline",
						"the transport never answered",
					));
				}
				Err(Expired::Cancelled) => {
					return Err(Error::NotReady(
						"measure was cancelled by the caller",
						"the signal fired while the page was being read",
					));
				}
			};
			Reconcile(Selectors, &Answered)
		})
	}

	fn Check(&self, Invariants: &[Invariant], Measurements: &[Measurement]) -> Verdict {
		Check(Invariants, Measurements)
	}
}

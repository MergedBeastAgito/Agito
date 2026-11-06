//! `agentic.runtime`, from `agentic-runtime`: what one operation is bounded by,
//! and the two combinators that apply a bound.
//!
//! [`Call`] is the bound itself, [`FutureBox`] is the return type every port
//! method is written in, and [`Bound`]/[`Race`] are how an operation plus a
//! deadline becomes an answer or an expiry. [`Error`] and [`Code`] are the
//! runtime's own failure vocabulary - `admission-refused`, `lease-held`,
//! `budget-exhausted` - which is not the browser's, so this is the copy that
//! stays out of the crate root.
//!
//! What is not here: the policy engine. `Runner`, `Authority`, `StandIn`,
//! `Admission`, `Lease`, `Budget` and the rest of the lease vocabulary are the
//! composition root's business, and a consumer that held a lease would be a
//! second authority over the same resource. `BlockOn` is the runtime's std
//! executor and is deliberately absent: the browser adapter spawns on tokio, so
//! a consumer driving it with `BlockOn` would panic inside a launch that had
//! already succeeded.

pub use AgenticRuntime::{
	Bound,
	Call,
	Cancellation,
	Clock,
	Code,
	Either,
	Error,
	Expired,
	FutureBox,
	Never,
	Pace,
	Race,
	Signal,
	Wall,
};

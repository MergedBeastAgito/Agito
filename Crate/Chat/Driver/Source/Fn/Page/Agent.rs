//! The page-side agent, and the expression that installs it.
//!
//! The agent is one TypeScript file under `Source/Script/Agent/`, compiled at
//! build time by this crate's `build.rs` and baked in by [`script!`] - the same
//! rule every expression in `Fn/Expression.rs` follows, and for the same
//! reason: the file a compiler parses is the file the page runs.
//!
//! What it installs is one idempotent object under `window.__Agito`: a
//! `MutationObserver` over the answer container, a `ResizeObserver` over the
//! document (layout growth a mutation record can miss), and an
//! `IntersectionObserver` over the container, which only ever reads. Arming
//! twice replaces the agent rather than stacking a second one, which is what
//! makes a re-arm after a navigation safe.
//!
//! It writes nothing to the page. No scroll, no focus, no viewport: a person
//! may be looking at the thread this driver is reading.

use std::time::Duration;

use ::RunTime::script;

use crate::Struct::{Selectors::Selectors, Signal::Binding};

/// Install the agent in the page, replacing one that is already there.
///
/// The expression returns whether the agent installed and whether the binding
/// it will call is present. `bound: false` is the page saying it cannot signal:
/// a driver that read it would be waiting for a call that will never come, so
/// the caller polls instead.
///
/// `Quiet` is the window, in milliseconds, that the page must be still for
/// before it reports itself settled. It is spliced into the script as a number
/// rather than as a string, so the page compares with a number.
pub fn Arm(Selectors:&Selectors, Quiet:Duration) -> String {
	let Milliseconds = Quiet.as_millis().to_string();
	script!(
		"Agent/Arm";
		Binding = &Binding,
		Answer = &Selectors.Answer,
		Quiet = &Milliseconds
	)
}

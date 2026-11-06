//! The tokio implementation of the runtime's `Pace`.
//!
//! The runtime crate keeps its own tokio implementation behind a feature
//! nothing enables, so that a capability never inherits an executor. This is
//! where the executor actually lives: the adapter that drives a browser is the
//! one thing in the workspace that must have one, and the composition root
//! hands this pace to every capability it composes.

use std::time::{Duration, Instant};

use ::RunTime::{Clock, FutureBox, Pace};

/// The clock and the wait, on tokio.
#[derive(Debug, Default)]
pub struct Tokio;

impl Clock for Tokio {
	fn Now(&self) -> Instant { Instant::now() }
}

impl Pace for Tokio {
	fn Sleep(&self, Span:Duration) -> FutureBox<'_, ()> { Box::pin(tokio::time::sleep(Span)) }
}

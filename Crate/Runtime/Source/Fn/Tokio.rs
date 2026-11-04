//! The tokio implementation of `Pace`, behind a feature nothing enables by
//! default.
//!
//! This module exists so that "the runtime owns the executor" is a statement
//! with code behind it, and is not enabled by any crate in this workspace: the
//! composition root starts one tokio runtime itself and supplies this `Pace` to
//! the capabilities it composes. A capability that depended on the feature
//! would have an executor in its dependency list, which is the thing the port
//! shape is there to prevent.

use std::time::{Duration, Instant};

use crate::Struct::Call::{Clock, FutureBox, Pace};

/// The clock and the wait, on tokio.
#[derive(Debug, Default)]
pub struct Tokio;

impl Clock for Tokio {
	fn Now(&self) -> Instant { Instant::now() }
}

impl Pace for Tokio {
	fn Sleep(&self, Span:Duration) -> FutureBox<'_, ()> { Box::pin(tokio::time::sleep(Span)) }
}

//! A cancellation source a tokio caller can fire from anywhere.
//!
//! The runtime's `Signal` is executor-neutral and works here, but a tokio program
//! usually already has a token it shares with other tasks, and converting one into
//! the port's shape is cheaper than a second source of truth. This is that
//! conversion and nothing else.

use std::sync::Arc;

use tokio_util::sync::CancellationToken;

use AgenticRuntime::{Cancellation, FutureBox};

/// A token, seen as a cancellation.
#[derive(Debug, Clone, Default)]
pub struct Stop(CancellationToken);

impl Stop {
	/// A token nobody has cancelled.
	pub fn New() -> Arc<Self> {
		Arc::new(Self(CancellationToken::new()))
	}

	/// Cancel it, waking every waiter.
	pub fn Cancel(&self) {
		self.0.cancel();
	}

	/// The token itself, for a caller that wants to share it.
	pub fn Token(&self) -> CancellationToken {
		self.0.clone()
	}
}

impl Cancellation for Stop {
	fn IsCancelled(&self) -> bool {
		self.0.is_cancelled()
	}

	fn Cancelled(&self) -> FutureBox<'_, ()> {
		let Token = self.0.clone();
		Box::pin(async move { Token.cancelled().await })
	}
}

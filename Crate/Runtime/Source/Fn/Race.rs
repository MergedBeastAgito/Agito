//! Race two futures, first one to finish wins.
//!
//! The primitive the deadline is built from. It is written here rather than
//! taken from a combinator crate because the runtime core is std-only: a
//! `select` is thirty lines of `poll`, and a dependency that exists to save
//! thirty lines is a dependency every capability inherits.

use std::{future::Future, pin::pin};

/// Whichever side finished first.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Either<Left, Right> {
	/// The left side.
	Left(Left),
	/// The right side.
	Right(Right),
}

impl<Left, Right> Either<Left, Right> {
	/// Whether the left side won.
	pub const fn IsLeft(&self) -> bool { matches!(self, Either::Left(_)) }
}

/// Resolve with whichever of `Left` and `Right` resolves first.
///
/// Both sides are polled in order on every wake, so a side that is already
/// ready wins without the other being polled at all. The loser is dropped,
/// which is the only cancellation Rust gives a future for free - and the reason
/// the port says a caller's cancellation stops *its wait* rather than closing a
/// page.
pub async fn Race<Left, Right>(Left:Left, Right:Right) -> Either<Left::Output, Right::Output>
where
	Left: Future,
	Right: Future, {
	let mut Left = pin!(Left);
	let mut Right = pin!(Right);
	std::future::poll_fn(|Context| {
		if let std::task::Poll::Ready(Value) = Left.as_mut().poll(Context) {
			return std::task::Poll::Ready(Either::Left(Value));
		}
		if let std::task::Poll::Ready(Value) = Right.as_mut().poll(Context) {
			return std::task::Poll::Ready(Either::Right(Value));
		}
		std::task::Poll::Pending
	})
	.await
}

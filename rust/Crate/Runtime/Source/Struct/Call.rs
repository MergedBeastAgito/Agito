//! What every operation is bounded by, and the facilities needed to apply a
//! bound without naming an executor.
//!
//! A deadline is an explicit parameter, not a global: [`Call`] carries it, every
//! port method takes one, and a provider that wants a different bound is handed a
//! different `Call` rather than reading a process-wide setting. The caller's
//! cancellation rides beside it, so "I gave up" and "the deadline passed" stay
//! two facts even where a port collapses them into one code.

use std::future::Future;
use std::pin::Pin;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, MutexGuard};
use std::task::{Poll, Waker};
use std::time::{Duration, Instant};

/// One boxed future.
///
/// Every port method returns one, which is what keeps `Arc<dyn Port>` possible:
/// a port with an `async fn` in it is not object-safe, and a port that is not
/// object-safe cannot be substituted by a fake in a test.
pub type FutureBox<'a, T> = Pin<Box<dyn Future<Output = T> + Send + 'a>>;

/// The caller's cancellation.
///
/// Two readings of one fact: a flag, for a provider that checks before it starts
/// work, and a future, for one that must stop in the middle. An implementation
/// must be race-safe and must resolve immediately when already cancelled - a
/// future that only resolves on the *next* fire would make an already-abandoned
/// call wait for an event that has happened.
pub trait Cancellation: Send + Sync {
	/// Whether the signal has fired.
	fn IsCancelled(&self) -> bool;
	/// Resolves once the signal has fired, or at once if it already has.
	fn Cancelled(&self) -> FutureBox<'_, ()>;
}

/// The only clock a deadline is compared against.
///
/// Supplied rather than reached for, so a test can expire a lease without
/// sleeping and a caller can express a deadline on the same authority the
/// runtime compares it against.
pub trait Clock: Send + Sync {
	/// The current instant, in whatever epoch the caller and the runtime agreed.
	fn Now(&self) -> Instant;
}

/// The clock, plus the ability to wait.
///
/// A capability paces itself between reads; the wait is supplied so that the
/// pacing does not decide which executor is running. A test supplies a wait that
/// advances its own clock, which is how a poll loop becomes deterministic
/// instead of slow.
pub trait Pace: Clock {
	/// Resolve after `Span`, or at once when it is zero.
	fn Sleep(&self, Span: Duration) -> FutureBox<'_, ()>;
}

/// What one operation is bounded by.
#[derive(Clone)]
pub struct Call {
	/// The instant the operation may no longer continue.
	pub Deadline: Instant,
	/// The caller's cancellation.
	pub Cancellation: Arc<dyn Cancellation>,
}

impl Call {
	/// A call whose deadline is `Span` from `Clock`'s reading now.
	pub fn Within(Span: Duration, Clock: &dyn Clock, Cancellation: Arc<dyn Cancellation>) -> Self {
		Self {
			Deadline: Clock.Now() + Span,
			Cancellation,
		}
	}

	/// How long is left, saturating at zero rather than panicking on a clock
	/// that has already passed the deadline.
	pub fn Remaining(&self, Clock: &dyn Clock) -> Duration {
		self.Deadline.saturating_duration_since(Clock.Now())
	}

	/// Whether the deadline has passed. The interval is half-open, so the
	/// instant itself is already outside it.
	pub fn Expired(&self, Clock: &dyn Clock) -> bool {
		Clock.Now() >= self.Deadline
	}

	/// Whether the caller has given up.
	pub fn Cancelled(&self) -> bool {
		self.Cancellation.IsCancelled()
	}
}

impl std::fmt::Debug for Call {
	fn fmt(&self, Formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		Formatter
			.debug_struct("Call")
			.field("Deadline", &self.Deadline)
			.field("Cancelled", &self.Cancellation.IsCancelled())
			.finish()
	}
}

/// Why a bounded operation stopped before it produced a value.
///
/// Two reasons and not one: a consumer that maps both onto its own single code
/// still needs `detail` to say which happened, and a provider that reported
/// "timeout" for its own cancellation would be blaming the page for the caller.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Expired {
	/// The deadline passed.
	Timeout,
	/// The caller's signal fired.
	Cancelled,
}

impl Expired {
	/// The phrase a failure detail uses.
	pub const fn Name(self) -> &'static str {
		match self {
			Expired::Timeout => "the deadline passed",
			Expired::Cancelled => "the caller cancelled",
		}
	}
}

/// A cancellation that never fires.
///
/// The default for a call nobody can cancel, and the honest way to say "this
/// operation has a deadline and no signal" instead of inventing one.
#[derive(Debug, Default)]
pub struct Never;

impl Cancellation for Never {
	fn IsCancelled(&self) -> bool {
		false
	}

	fn Cancelled(&self) -> FutureBox<'_, ()> {
		Box::pin(std::future::pending())
	}
}

/// A cancellation a caller fires when it gives up.
///
/// The waiters are woken rather than polled, so a cancelled call resolves on the
/// thread that fired the signal instead of on the next tick of a loop. A waker
/// that arrives after the fire is answered by the second flag read, which is why
/// the read happens on both sides of the registration.
#[derive(Debug, Default)]
pub struct Signal {
	Fired: AtomicBool,
	Waiters: Mutex<Vec<Waker>>,
}

impl Signal {
	/// A signal nobody has fired.
	pub fn New() -> Arc<Self> {
		Arc::new(Self::default())
	}

	/// Fire it, waking everyone waiting. Firing twice is a no-op.
	pub fn Fire(&self) {
		self.Fired.store(true, Ordering::SeqCst);
		let Waiters = std::mem::take(&mut *Self::Lock(&self.Waiters));
		for Waker in Waiters {
			Waker.wake();
		}
	}

	/// A lock whose poisoning is not a failure: a panicked waiter must not make
	/// cancellation itself unavailable.
	fn Lock<T>(Guard: &Mutex<T>) -> MutexGuard<'_, T> {
		Guard.lock().unwrap_or_else(|Poison| Poison.into_inner())
	}
}

impl Cancellation for Signal {
	fn IsCancelled(&self) -> bool {
		self.Fired.load(Ordering::SeqCst)
	}

	fn Cancelled(&self) -> FutureBox<'_, ()> {
		Box::pin(std::future::poll_fn(move |Context| {
			if self.Fired.load(Ordering::SeqCst) {
				return Poll::Ready(());
			}
			Self::Lock(&self.Waiters).push(Context.waker().clone());
			if self.Fired.load(Ordering::SeqCst) {
				return Poll::Ready(());
			}
			Poll::Pending
		}))
	}
}

/// The wall clock, for a caller that has nothing better to supply.
#[derive(Debug, Default)]
pub struct Wall;

impl Clock for Wall {
	fn Now(&self) -> Instant {
		Instant::now()
	}
}

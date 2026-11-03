//! Bound one operation by a deadline and the caller's cancellation.
//!
//! This is the combinator every port's guarantee rests on, and it is written
//! against three traits - a boxed future, a `Pace` and a `Cancellation` - so no
//! capability has to know which executor is running it.
//!
//! The two bounds are separate on purpose. A caller's cancellation and a passed
//! deadline are both "no answer", but they are not the same fact: one is the
//! caller giving up and the other is the page not finishing, and a consumer that
//! collapsed them without keeping `detail` could not tell a slow page from its own
//! impatience.

use crate::Struct::Call::{Call, Expired, FutureBox, Pace};
use crate::Fn::Race::{Either, Race};

/// Resolve with `Work`'s value, or with why it was cut short.
///
/// A signal that has already fired rejects before `Work` is polled at all: a
/// caller that has given up must not cause one more page operation to start.
pub fn Bound<'a, T: 'a>(
	Work: FutureBox<'a, T>,
	Call: &'a Call,
	Pace: &'a dyn Pace,
) -> FutureBox<'a, Result<T, Expired>> {
	Box::pin(async move {
		if Call.Cancellation.IsCancelled() {
			return Err(Expired::Cancelled);
		}
		let Guard = async {
			let Wait = Pace.Sleep(Call.Remaining(Pace));
			let Stop = Call.Cancellation.Cancelled();
			match Race(Wait, Stop).await {
				Either::Left(()) => Expired::Timeout,
				Either::Right(()) => Expired::Cancelled,
			}
		};
		match Race(Work, Guard).await {
			Either::Left(Value) => Ok(Value),
			Either::Right(Why) => Err(Why),
		}
	})
}

//! Drive a future to completion on this thread.
//!
//! A capability's port is a boxed future, and a test that exercises one has no
//! executor - that is the point of the port being runtime-neutral. This is the
//! smallest honest answer: poll until ready, yielding between polls.
//!
//! It is not an executor and does not pretend to be one: nothing here parks or
//! schedules, so it is correct for futures that are driven by something else
//! (a signal fired on another thread, a fake clock advanced by a fake wait) and
//! wrong for a future that only ever resolves from a real timer. A test that
//! wants time to pass advances a clock instead of waiting for one.

use std::{
	future::Future,
	pin::pin,
	task::{Context, Waker},
};

/// Poll `Work` until it produces a value.
pub fn BlockOn<Work:Future>(Work:Work) -> Work::Output {
	let mut Work = pin!(Work);
	let Waker = Waker::noop();
	let mut Context = Context::from_waker(Waker);
	loop {
		if let std::task::Poll::Ready(Value) = Work.as_mut().poll(&mut Context) {
			return Value;
		}
		std::thread::yield_now();
	}
}

//! One round trip: post the request, parse the reply.
//!
//! Deliberately small, and deliberately not metered. The three steps it joins -
//! post through the port, read the body, fill in what the request knows - are
//! one statement of "how an answer is obtained", so a consumer that wants the
//! round trip without the [`crate::Struct::Decider::Decider`]'s policy can have
//! exactly that and meter it itself.
//!
//! The meter is the caller's because a meter has to be written across the await
//! and a lock cannot be: [`crate::Struct::Decider::Decider`] reads its ceiling,
//! drops the guard, awaits this, and records the spend afterwards. Putting the
//! meter in here would mean taking the lock on the caller's behalf and holding
//! it through a network call, which is the one shape
//! [`AgenticRuntime::FutureBox`] refuses.
//!
//! # A cancelled call is not posted
//!
//! The caller's signal is checked before the request leaves, so an operation
//! somebody has already abandoned does not become a request somebody pays for.
//! The poster is still handed the [`AgenticRuntime::Call`]: it owns the
//! in-flight bound, and a poster that ignored it could turn a bounded call into
//! an unbounded one.

use AgenticRuntime::Call;

use crate::Struct::{Answer::Answer, Error::Error, Port::Poster, Request::Request};

/// Post one request and return the answer it caused.
pub async fn Ask(Request:&Request, Poster:&dyn Poster, Call:Call) -> Result<Answer, Error> {
	if Call.Cancelled() {
		return Err(Error::PostFailed(
			"the caller cancelled before the request was posted",
			Request.Path(),
		));
	}
	let Body = Poster
		.Post(Request, Call)
		.await
		.map_err(|Failure| Error::PostFailed(Failure.Message, Failure.Detail))?;
	// The request is in hand here and the reply may have omitted a score's
	// legend, so this is the one place the two can be joined.
	Ok(Parse::Parse(&Body)?.Filled(Request))
}

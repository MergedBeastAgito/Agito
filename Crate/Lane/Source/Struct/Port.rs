//! The port `agentic.lane.decide` promises, and the seam it needs.
//!
//! Two traits, and the split is the crate's whole claim about networks:
//!
//! - [`Poster`] is what the crate needs and never implements. One request in,
//!   one reply body out, bounded by the caller's [`AgenticRuntime::Call`]. The
//!   credentials, the transport, the status codes and the retry policy are an
//!   adapter's business; a fixture that answers from a captured body is an
//!   adapter too, which is why the suite needs no socket.
//! - [`Decide`] is what the crate provides: select a lane, ask about it, report
//!   the meter.
//!
//! # What the port states beyond its signatures
//!
//! - **The poster returns the body, not a parsed answer.** One statement of the
//!   reply's shape, in [`crate::Fn::Parse::Parse`], so two posters cannot
//!   disagree about what a `choice` looks like and a captured body is still the
//!   thing the parser is tested against.
//! - **Selection is a function of the state and the rules, and nothing else.**
//!   No clock, no counter, no cache: two calls with the same state select the
//!   same lane, which is what makes a trace reproducible.
//! - **A call is bounded.** The poster honours the [`AgenticRuntime::Call`] it is
//!   handed; the crate checks the caller's cancellation before it posts, so a
//!   call that was abandoned does not become a request somebody pays for.
//! - **The meter is written around the call, never across it.** A lock held over
//!   an await would make the future `!Send` and the port unusable from a
//!   spawned task, so [`crate::Struct::Decider::Decider`] reads its ceiling,
//!   drops the lock, awaits, and writes back.

use AgenticRuntime::{Call, FutureBox};

use crate::Struct::{Answer::Answer, Budget::Budget, Error::Error, Lane::Selection, Request::Request, State::State};

/// What a poster reports when it cannot answer.
///
/// A poster is not this capability, so it does not speak this capability's
/// codes. It says what it observed, and the round trip maps that onto
/// `post-failed` with the observation in `detail` - which is the whole point of
/// keeping the detail apart from the sentence.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Failure {
	/// The sentence a human reads.
	pub Message:String,
	/// What was observed.
	pub Detail:String,
}

impl Failure {
	/// A failure with a sentence and nothing observed.
	pub fn Of(Message:impl Into<String>) -> Self { Self { Message:Message.into(), Detail:String::new() } }

	/// The same failure, with what was observed.
	pub fn With(mut self, Detail:impl Into<String>) -> Self {
		self.Detail = Detail.into();
		self
	}
}

/// The endpoint as this capability needs it: one request, one reply body.
///
/// What the port demands of an implementation:
///
/// - the body returned is the endpoint's own bytes, unmodified. A poster that
///   unwrapped `result` would be a second parser;
/// - the [`Call`] is honoured: a request that never answers is failed, not
///   awaited. A poster that ignores it cannot quietly turn a bounded call into
///   an unbounded one, because the deadline is the caller's;
/// - nothing secret appears in a [`Failure`]. A token in a detail string is a
///   token in a log.
pub trait Poster: Send + Sync {
	/// Post one request and return the reply body.
	fn Post<'a>(&'a self, Request:&'a Request, Call:Call) -> FutureBox<'a, Result<String, Failure>>;
}

/// The one port this capability offers.
pub trait Decide: Send + Sync {
	/// The lane and template this state selects, by the rules this value holds.
	fn Select(&self, State:&State) -> Result<Selection, Error>;

	/// Ask about one selection, and answer what the model said.
	///
	/// The selection is an argument rather than something re-derived here, so a
	/// caller that has already traced its choice asks about exactly that choice
	/// - and a run's log and its request cannot disagree about which lane ran.
	fn Decide<'a>(
		&'a self,
		Selection:&'a Selection,
		State:&'a State,
		Call:Call,
	) -> FutureBox<'a, Result<Answer, Error>>;

	/// What this run has spent, as it stands.
	fn Budget(&self) -> Budget;
}

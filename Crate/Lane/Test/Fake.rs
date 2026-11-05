//! A poster that answers from a canned body, with no socket in sight.
//!
//! The port is the crate's only way out, so a test that supplies this fake
//! exercises the whole round trip - select, compose, post, parse, meter - and
//! still reaches nothing. What it *records* is the other half of the
//! instrument: a call refused before it was posted, or posted with a body that
//! is not the lane's, leaves no trace in an answer and is obvious in the
//! record.
//!
//! It lives beside the tests rather than in `Source/`, for the reason the
//! catalog's fake gives: `Source/` is what ships, and a capability that shipped
//! a poster would be a capability that could reach the network. This is the
//! instrument, not the model.
//!
//! # Two instruments, one port
//!
//! - [`Fake`] is *scripted*: the test decides the reply, or the failure, and the
//!   same reply answers every call.
//! - [`FixturePoster`] is *shipped*: the reply is the document at
//!   `Test/Fixture/Clef/Response.json`, compiled in with `include_str!`, so the
//!   body a run is exercised against is a file a person edits without touching
//!   Rust - and the `budget` beside it says what that run is worth.
//!
//! They answer the same two questions with different numbers on purpose. A test
//! that meant the shipped document and was handed the scripted body fails on the
//! choice and the shares, instead of agreeing by accident.
//!
//! # It ignores the call's deadline, on purpose
//!
//! A real poster owns the in-flight bound. A fake that enforced it as well
//! would mask the crate's own check: [`AgenticLane::Ask`] refuses to post a
//! call the caller has already cancelled, and a test of that proves nothing if
//! the poster would have refused too. So the fake always answers, and
//! `a_cancelled_call_is_never_posted` reads an empty record rather than a
//! failure it cannot attribute.
//!
//! # Using it from another test binary
//!
//! Include it the way `Test/Lane.rs` does - `mod Fake;` beside the test root -
//! then name what it offers:
//!
//! ```ignore
//! mod Fake;
//!
//! use Fake::{Captured, Fake as Scripted, Fixture, FixturePoster};
//! ```
//!
//! The module and the struct cannot share a name in one scope, which is why the
//! import renames `Fake`. Nothing here is `pub(crate)`: a test binary is its own
//! crate, so the items a test reaches for have to be `pub`.

#![allow(dead_code)]

use std::{
	collections::BTreeMap,
	future,
	sync::{Arc, Mutex},
};

use AgenticLane::{Failure, Poster, Request};
use AgenticRuntime::{Call, FutureBox};
use serde_json::Value;

/// A reply the endpoint actually answered, kept as the bytes a poster returns.
///
/// Captured rather than composed, because the parser's job is to read what the
/// endpoint writes - a body this crate built for itself would agree with its own
/// assumptions and prove nothing. Two questions, one of each shape: a choice
/// with a distribution over both its options, and a score with no legend of its
/// own (the request supplies that, which is the other thing a canned body
/// cannot fake for itself).
pub const Captured:&str = r#"{
	"success": true,
	"result": {
		"model": "clef-flash",
		"answers": {
			"course": {
				"type": "choice",
				"choice": "closer",
				"probabilities": { "closer": 0.62, "farther": 0.38 }
			},
			"disagreement": { "type": "score", "score": 2 }
		},
		"usage": { "input_tokens": 375, "output_tokens": 0 }
	}
}"#;

/// A poster that answers what a test scripted, and records what it was asked.
///
/// Construct it with [`Fake::Answering`] or [`Fake::Failing`]; both hand back an
/// `Arc<Self>` so a test can keep a handle to read [`Fake::Posted`] after the
/// call while the decider holds the trait object.
pub struct Fake {
	/// The reply to return, or the failure to report.
	Reply:Result<String, Failure>,
	/// Every request it was handed, as the path and the body it would send.
	Asked:Mutex<Vec<(String, String)>>,
}

impl Fake {
	/// A poster that answers with one body, whatever it is asked.
	pub fn Answering(Body:impl Into<String>) -> Arc<Self> {
		Arc::new(Self { Reply:Ok(Body.into()), Asked:Mutex::new(Vec::new()) })
	}

	/// A poster that reports one failure instead of answering.
	pub fn Failing(Message:impl Into<String>, Detail:impl Into<String>) -> Arc<Self> {
		Arc::new(Self { Reply:Err(Failure::Of(Message).With(Detail)), Asked:Mutex::new(Vec::new()) })
	}

	/// Every request posted so far, in the order it was posted: the path it
	/// would have gone to and the body it would have sent.
	pub fn Posted(&self) -> Vec<(String, String)> { self.Asked.lock().unwrap().clone() }

	/// The one request posted, for a test that expects exactly one.
	///
	/// A test that reads this and finds two has a round trip that posted twice,
	/// which is the failure it was looking for - so the count is asserted here
	/// rather than left to each caller.
	pub fn Only(&self) -> (String, String) {
		let Posted = self.Posted();
		assert_eq!(Posted.len(), 1, "expected exactly one post, saw {}", Posted.len());
		Posted.into_iter().next().expect("one post")
	}
}

impl Poster for Fake {
	fn Post<'a>(&'a self, Request:&'a Request, _Call:Call) -> FutureBox<'a, Result<String, Failure>> {
		self.Asked.lock().unwrap().push((Request.Path(), Request.Text()));
		Box::pin(future::ready(self.Reply.clone()))
	}
}

/// The canned reply this crate ships, and what the run it describes is worth.
///
/// The document has two top-level keys, and the split is the point: `response`
/// is the bytes a poster returns, kept in the endpoint's own shape so the parser
/// is tested against something this crate did not write, and `budget` is a fact
/// about the stand-in that the endpoint never sends. Folding the bookkeeping
/// into the reply would put a field in the body no real reply carries, and every
/// assertion about the body's shape would then be an assertion about this file.
///
/// A document that cannot be read is a mistake in the fixture, not a condition
/// at run time, so [`Fixture::Read`] panics with the fact rather than answering
/// a default: a fixture that quietly substituted one would let every test that
/// used it pass for the wrong reason.
pub struct Fixture {
	/// How many composed requests the canned run is worth.
	Budget:u32,
	/// The reply, as the endpoint writes it.
	Response:Value,
}

impl Fixture {
	/// The document this crate ships beside its tests.
	///
	/// `include_str!` rather than a path read, because a test binary's working
	/// directory is the package root and not the test's own directory: a
	/// relative path that worked under `cargo test` would break under anything
	/// that ran the binary from elsewhere. The document is compiled in, so
	/// editing it means recompiling - which `cargo test` does on its own.
	pub fn Shipped() -> Fixture { Self::Read(include_str!("Fixture/Clef/Response.json")) }

	/// A document from its own text, checked as it is read.
	///
	/// Three things are refused here rather than at an assertion that would
	/// otherwise trip over them later: a budget that is not a non-negative
	/// integer, a reply that answers no question, and a choice whose shares do
	/// not sum to one. The last is the one worth having: a distribution that
	/// does not sum to one is a body no probability can be compared against, and
	/// a test that found it out through a floating-point assertion would report
	/// the symptom and not the fixture.
	pub fn Read(Text:&str) -> Fixture {
		let Document:Value =
			serde_json::from_str(Text).unwrap_or_else(|Why| panic!("the Clef fixture is not JSON: {Why}"));
		let Requests = Document
			.get("budget")
			.and_then(Value::as_u64)
			.unwrap_or_else(|| panic!("the Clef fixture declares no `budget` as a non-negative integer: {Text}"));
		let Budget = u32::try_from(Requests).unwrap_or_else(|_| {
			panic!("the Clef fixture's `budget` ({Requests}) is larger than a request count can be")
		});
		let Response = Document
			.get("response")
			.cloned()
			.unwrap_or_else(|| panic!("the Clef fixture declares no `response`: {Text}"));

		let Answers = Response
			.get("result")
			.and_then(|Result| Result.get("answers"))
			.and_then(Value::as_object)
			.filter(|Answers| !Answers.is_empty())
			.unwrap_or_else(|| panic!("the Clef fixture's `response` answers no question: {Text}"));

		for (Name, One) in Answers {
			let Some(Shares) = One.get("probabilities").and_then(Value::as_object) else {
				continue;
			};
			let Sum:f64 = Shares.values().filter_map(Value::as_f64).sum();
			if (Sum - 1.0).abs() > 1e-6 {
				panic!(
					"the Clef fixture's `{Name}` probabilities sum to {Sum}, not 1: a distribution that does not is a \
					 reply nothing can be compared against"
				);
			}
			let Chosen = One
				.get("choice")
				.and_then(Value::as_str)
				.unwrap_or_else(|| panic!("the Clef fixture's `{Name}` carries probabilities but no `choice`"));
			if !Shares.contains_key(Chosen) {
				panic!("the Clef fixture chose `{Chosen}` for `{Name}`, which is not one of its own options");
			}
		}

		Fixture { Budget, Response }
	}

	/// How many composed requests the canned run is worth.
	pub fn Budget(&self) -> u32 { self.Budget }

	/// The reply, as the endpoint writes it.
	pub fn Response(&self) -> &Value { &self.Response }

	/// The reply as the bytes a poster returns.
	pub fn Body(&self) -> String { self.Response.to_string() }

	/// The option a choice question in the canned reply chose.
	///
	/// A lens, not a second parser: it reads the two fields a test compares an
	/// answer against and interprets nothing else, so a score, a type this crate
	/// does not know and a question the reply never answered are all `None`. What
	/// a reply *means* stays in [`AgenticLane::Parse`], which is the one
	/// statement of its shape.
	pub fn Choice(&self, Question:&str) -> Option<&str> { self.Answers()?.get(Question)?.get("choice")?.as_str() }

	/// Every option's share of a choice question in the canned reply.
	pub fn Probabilities(&self, Question:&str) -> BTreeMap<String, f64> {
		self.Answers()
			.and_then(|Answers| Answers.get(Question))
			.and_then(|One| One.get("probabilities"))
			.and_then(Value::as_object)
			.map(|Shares| {
				Shares
					.iter()
					.filter_map(|(Option, Share)| Share.as_f64().map(|Share| (Option.clone(), Share)))
					.collect()
			})
			.unwrap_or_default()
	}

	/// The `answers` object, which is where every fact above is read from.
	fn Answers(&self) -> Option<&serde_json::Map<String, Value>> {
		self.Response.get("result")?.get("answers")?.as_object()
	}
}

/// A poster that answers from a [`Fixture`], and records every request it was
/// handed.
///
/// This is the stand-in a run is wired with: the same port as the endpoint, one
/// canned reply, no socket, no clock and no credential. What it records is the
/// request *value*, so a test asserts on the composed shape -
/// [`Request::Body`], [`Request::Path`], [`Request::Names`] - instead of parsing
/// a string back into the thing it started as.
///
/// It answers every call with the same body and never fails, and it does not
/// refuse past its own `budget`. A poster with a ceiling would be a second
/// authority over the same spend; the run's own [`AgenticLane::Budget`] is where
/// that ceiling belongs, and the fixture's `budget` is the number a test
/// configures it with.
pub struct FixturePoster {
	/// The reply it answers with, checked when it was read.
	Fixture:Fixture,
	/// Every request it was handed, in the order it was handed them.
	Asked:Mutex<Vec<Request>>,
}

impl FixturePoster {
	/// The poster for the document this crate ships.
	pub fn Shipped() -> Arc<Self> { Self::Of(Fixture::Shipped()) }

	/// The poster for a document a test wrote itself.
	pub fn Of(Fixture:Fixture) -> Arc<Self> { Arc::new(Self { Fixture, Asked:Mutex::new(Vec::new()) }) }

	/// The document it answers from, for a test that asserts against the file.
	pub fn Fixture(&self) -> &Fixture { &self.Fixture }

	/// How many composed requests the canned run is worth.
	pub fn Budget(&self) -> u32 { self.Fixture.Budget() }

	/// Every request posted so far, in the order it was posted.
	pub fn Posted(&self) -> Vec<Request> { self.Asked.lock().unwrap().clone() }

	/// The one request posted, for a test that expects exactly one.
	///
	/// A test that reads this and finds two has a round trip that posted twice,
	/// which is the failure it was looking for - so the count is asserted here
	/// rather than left to each caller.
	pub fn Only(&self) -> Request {
		let Posted = self.Posted();
		assert_eq!(Posted.len(), 1, "expected exactly one post, saw {}", Posted.len());
		Posted.into_iter().next().expect("one post")
	}
}

impl Poster for FixturePoster {
	fn Post<'a>(&'a self, Request:&'a Request, _Call:Call) -> FutureBox<'a, Result<String, Failure>> {
		self.Asked.lock().unwrap().push(Request.clone());
		Box::pin(future::ready(Ok(self.Fixture.Body())))
	}
}

/// The instrument's own claims, beside the instrument.
///
/// They run in whichever test binary includes this module, which is what keeps
/// the fixture from breaking silently: the reply is data, and a document that
/// stopped parsing would otherwise be found by the first test that happened to
/// need it, in a failure naming the answer rather than the file.
#[cfg(test)]
mod Smoke {
	use std::{sync::Arc, time::Duration};

	use AgenticLane::{Ask, Parse, Request, State};
	use AgenticRuntime::{BlockOn, Call, Never, Wall};

	use super::{Fixture, FixturePoster};

	/// One call's bound, on the wall clock.
	fn Budget() -> Call { Call::Within(Duration::from_secs(1), &Wall, Arc::new(Never)) }

	/// A request that asks nothing, for a poster that answers whatever it is
	/// handed: what is under test here is the stand-in, not a composition.
	fn Asked() -> Request {
		Request { Model:"clef-flash".to_string(), State:State::New(), Questions:Default::default() }
	}

	#[test]
	fn the_shipped_fixture_parses_with_a_whole_distribution_and_a_budget() {
		let Fixture = Fixture::Shipped();
		assert!(Fixture.Budget() >= 1, "a canned run worth no request is not a fixture");

		let Shares = Fixture.Probabilities("course");
		assert_eq!(Shares.len(), 2, "every option the reply named is carried: {Shares:?}");
		let Sum:f64 = Shares.values().sum();
		assert!((Sum - 1.0).abs() < 1e-6, "the shares sum to one, saw {Sum}");

		let Chosen = Fixture.Choice("course").expect("the canned reply chose an option");
		assert!(
			Shares.contains_key(Chosen),
			"`{Chosen}` is one of the options it was chosen from"
		);

		// The same three facts, read through the parser the run itself uses: a
		// body this crate cannot read is a fixture for nothing.
		let Answer = Parse(&Fixture.Body()).expect("the canned body is a reply");
		assert_eq!(Answer.Choice("course"), Some((Chosen, Shares[Chosen])));
		assert!(Answer.Usage.Input > 0, "the reply reports what it spent");
	}

	#[test]
	fn the_fixture_poster_answers_from_the_file_and_records_what_it_was_asked() {
		let Fixture = Fixture::Shipped();
		let Poster = FixturePoster::Shipped();
		let Request = Asked();

		let Answer = BlockOn(Ask(&Request, &*Poster, Budget())).expect("the fixture answers with no socket");
		assert_eq!(Answer.Choice("course").map(|(Chosen, _)| Chosen), Fixture.Choice("course"));

		let Posted = Poster.Only();
		assert_eq!(Posted, Request, "the request it answered is the request it recorded");
		assert_eq!(Posted.Body()["model"], "clef-flash");
		assert_eq!(Posted.Path(), "/ai/run/@cf/cloudflare/clef-flash");
		assert_eq!(Poster.Budget(), Fixture.Budget());
	}

	#[test]
	#[should_panic(expected = "probabilities sum to")]
	fn a_fixture_whose_distribution_does_not_sum_to_one_is_refused() {
		Fixture::Read(
			r#"{
				"budget": 1,
				"response": { "success": true, "result": { "answers": {
					"course": { "type": "choice", "choice": "closer", "probabilities": { "closer": 0.6, "farther": 0.2 } }
				} } }
			}"#,
		);
	}
}

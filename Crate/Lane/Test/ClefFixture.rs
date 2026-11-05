//! The shipped Clef fixture: the document, the poster that answers from it, and
//! the promises a run may rely on.
//!
//! The reply a run is exercised against is a file rather than a Rust literal -
//! `Test/Fixture/Clef/Response.json` - and this is the suite that holds the file
//! to its own three claims: a chosen option, a share for every option that sums
//! to one, and a budget. Editing the document is the whole of changing the reply;
//! nothing here needs re-deriving when it changes.
//!
//! Nothing opens a socket. The poster is `Test/Fake.rs`'s, the corpus is not read
//! at all - a request is written directly, because what is under test is the
//! stand-in and not a composition - and the document is compiled in with
//! `include_str!`, so the test cannot pass against a file the binary did not
//! carry.

#![allow(non_snake_case, non_upper_case_globals)]

mod Fake;

use std::{sync::Arc, time::Duration};

use AgenticLane::{Ask, Parse, Request, State};
use AgenticRuntime::{BlockOn, Call, Never, Wall};
use Fake::{Fixture, FixturePoster};

/// One call's bound, on the wall clock.
fn Budget() -> Call { Call::Within(Duration::from_secs(1), &Wall, Arc::new(Never)) }

/// A request that asks nothing, for a poster that answers whatever it is handed:
/// what is under test here is the stand-in, not a composition.
fn Asked() -> Request { Request { Model:"clef-flash".to_string(), State:State::New(), Questions:Default::default() } }

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

	// The same three facts, read through the parser the run itself uses: a body
	// this crate cannot read is a fixture for nothing.
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

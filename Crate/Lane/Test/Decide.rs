//! The entry point as a contract: a corpus on disk and a state in, the poster's
//! canned reply through the port, a typed answer out - and then the three ways
//! the same call fails.
//!
//! Every case runs [`Decide`] whole - select by the corpus's own condition,
//! choose the template, compose, post, decode, fill - against the fixture corpus
//! this crate ships beside its tests (`Test/Fixture/Select`) and the canned Clef
//! reply (`Test/Fixture/Clef/Response.json`, read through `Test/Fake.rs`).
//! Nothing here opens a connection and nothing here reads the clock beyond a
//! one-second bound that never expires.
//!
//! `Test/EndToEnd.rs` runs the same pipeline against the same fixture and holds
//! the answer and the body; this file holds the *signature's* promises, which a
//! happy path cannot show - one error type with three origins, the order they
//! are reached in, and the two spellings of the poster argument. The overlap is
//! deliberate and small: a case that asserts the answer here exists so a failure
//! in the entry point cannot hide behind a failure in another target.
//!
//! # What each case holds down
//!
//! - **the answer**, typed: the option the document chose, a share for every
//!   option summing to one, the score the document also answered, and the
//!   legend the *request* had to supply because the reply omitted it;
//! - **the spend**: what the reply reported, charged as input plus output, with
//!   no remaining invented for a ceiling nobody declared, and one request spent
//!   of the one the canned run is worth;
//! - **the body**, asserted on the bytes a poster would send rather than on the
//!   value it was built from: exactly `{model, state, questions}`;
//! - **the three origins of one error**, each reached through the same entry
//!   point: a selection that names no lane, a poster that does not answer, and
//!   a reply the decoder refuses - the last carrying the decoder's own variant
//!   rather than a sentence to parse back out of a string;
//! - **the order they are reached in**: a failure before the post leaves the
//!   poster's record empty, which is what "nobody was billed" looks like.
//!
//! # Why the corpus is read from disk
//!
//! A corpus built inline in this file would let the entry point pass against a
//! shape only this test writes. `Test/Fixture/Select` is a document a person
//! edits, with four lanes whose conditions are mutually exclusive and two
//! templates that ask different numbers of questions - so "the lane the corpus
//! names was the lane that was asked" is a claim about data, not about Rust.
//!
//! # No socket anywhere
//!
//! The transport is [`Poster`] and this target implements none: every case wires
//! `Test/Fake.rs`. `the_crate_depends_on_nothing_that_can_open_a_socket` states
//! that as a claim about the manifest rather than a habit of this file, because
//! a dependency added later is how a socket would arrive.

#![allow(non_snake_case, non_upper_case_globals)]

mod Fake;

use std::{
	path::{Path, PathBuf},
	sync::Arc,
	time::Duration,
};

use AgenticLane::{Answer, Answered, Code, Context, Decide, DecideOn, DecodeError, Load, Poster, SelectLane, State};
use AgenticRuntime::{BlockOn, Call, Never, Wall};
use Fake::{Fake as Scripted, Fixture, FixturePoster};
use serde_json::{Value, json};

/// The fixture corpus this crate ships beside its test: four lanes, each with
/// its own condition, and two templates.
fn Directory() -> PathBuf { Path::new(env!("CARGO_MANIFEST_DIR")).join("Test/Fixture/Select") }

/// One call's bound, on the wall clock.
///
/// The fixture answers immediately, so this never expires - it exists because
/// the port takes a [`Call`] and a test that passed no bound would not be
/// exercising the signature a caller uses.
fn Bound() -> Call { Call::Within(Duration::from_secs(1), &Wall, Arc::new(Never)) }

/// A reading that is turning at speed.
///
/// Exactly one lane holds for it - `fast`, whose condition is `speed > 120` and
/// `yaw > 5` - and that lane serves `overview`, the template that asks both
/// questions the canned reply answers. The numbers are chosen so that no other
/// lane's condition can hold: `turning` needs `speed < 120` and `steady` needs
/// `yaw < 5`.
fn Racing() -> State {
	let mut State = State::New();
	State.Set("speed", 200);
	State.Set("yaw", 12);
	State
}

/// The request's three keys, which are also the body's three keys.
const Keys:[&str; 3] = ["model", "questions", "state"];

/// The chat-completions vocabulary, none of which a decision request may carry.
///
/// Listed rather than described because the failure this guards against is a
/// payload somebody copied from a chat client: `messages` and `temperature` are
/// what such a payload arrives with, and the endpoint answers 400 with no
/// diagnosis in it when it gets one.
const Chat:[&str; 14] = [
	"messages",
	"role",
	"roles",
	"content",
	"prompt",
	"temperature",
	"top_p",
	"max_tokens",
	"max_completion_tokens",
	"stream",
	"choices",
	"n",
	"stop",
	"response_format",
];

#[test]
fn the_entry_point_answers_the_canned_reply_with_a_typed_answer() {
	let Corpus = Load(&Directory()).expect("the fixture corpus loads");
	let Poster = FixturePoster::Shipped();
	let Fixture = Poster.Fixture();
	let State = Racing();

	// Printed before anything is asserted, so a failure names the lane it was
	// about instead of leaving the reader to re-derive it.
	let Lane = SelectLane(&Corpus, &Context::from(&State)).expect("exactly one lane holds for a reading at speed");
	println!("lane selected: {} · {}", Lane.Name, Lane.Select.Line());

	let Answer = BlockOn(Decide(&*Poster, &Corpus, &State, Bound())).expect("the fixture answers without a socket");
	println!("choice: {:?} · usage {:?}", Answer.Choice("course"), Answer.Usage);

	// --- the lane the corpus names is the lane that was asked -------------
	assert_eq!(Lane.Name, "fast", "the corpus's own condition names this lane");
	assert_eq!(
		Lane.Templates.first().map(String::as_str),
		Some("overview"),
		"and this lane's first template is the one it asks"
	);
	let Sent = Poster.Only();
	assert_eq!(
		Sent.Names(),
		vec!["course", "disagreement"],
		"the request asks exactly what the selected template asks"
	);
	assert_eq!(
		Sent.State.Fields().cloned().collect::<Vec<String>>(),
		Lane.Fields,
		"and sends exactly the fields the selected lane names - another lane's would be a different shape"
	);

	// --- the typed answer -------------------------------------------------
	let Expected = Fixture.Choice("course").expect("the canned reply chose an option");
	let (Choice, Share) = Answer.Choice("course").expect("the reply answered `course`");
	assert_eq!(Choice, Expected, "the typed answer names the option the canned document chose");

	let Answered::Choice { Choice: Chosen, Probabilities } = Answer
		.Answer("course")
		.expect("the reply answered `course`, so the answer carries it")
	else {
		panic!("`course` is a choice question and the reply answered it as one");
	};
	assert_eq!(Chosen, Expected);
	assert_eq!(
		Probabilities,
		&Fixture.Probabilities("course"),
		"a share is kept for every option the document named, and no option it did not"
	);
	assert_eq!(
		Probabilities.get(Chosen).copied(),
		Some(Share),
		"the chosen option's share is its own, not a re-normalised confidence"
	);
	let Sum:f64 = Probabilities.values().sum();
	assert!((Sum - 1.0).abs() < 1e-6, "the shares sum to one within 1e-6, saw {Sum}");
	assert!(
		Probabilities.values().all(|Share| (0.0..=1.0).contains(Share)),
		"every share is a share: {Probabilities:?}"
	);

	// The document also answers a score and omits its legend, so this is the
	// end-to-end proof that the entry point joined the request to the reply:
	// without `Answer::Filled` the position could not be computed at all.
	assert_eq!(Answer.Score("disagreement"), Some(2.0), "the second answer is a score of 2");
	assert_eq!(
		Answer.Position("disagreement"),
		Some(1.0),
		"2 on the request's three-word scale is the top"
	);

	// --- the spend --------------------------------------------------------
	let Allowed = Fixture.Budget();
	assert!(Allowed >= 1, "a canned run worth no request is not a fixture, saw {Allowed}");
	assert_eq!(
		Poster.Posted().len(),
		1,
		"one round trip is one request, inside the fixture's own budget of {Allowed}"
	);

	let Reported = Fixture.Response()["result"]["usage"]["input_tokens"]
		.as_u64()
		.expect("the canned reply reports the input tokens it spent");
	assert_eq!(Answer.Usage.Input, Reported, "the answer carries what the reply reported");
	assert_eq!(Answer.Usage.Output, 0, "clef-flash prices output at zero and reported none");
	assert_eq!(
		Answer.Usage.Consumed, Reported,
		"the meter charges input plus output, computed once at decode time"
	);
	assert_eq!(
		Answer.Usage.Remaining, None,
		"a decoder given no token ceiling invents no remaining"
	);
	assert!(!Answer.Usage.Exhausted(), "no ceiling means no exhaustion");
	assert_eq!(Answer.Model, "clef-flash", "the answer names the model the lane asked");
}

#[test]
fn the_composed_request_is_exactly_model_state_questions() {
	let Corpus = Load(&Directory()).expect("the fixture corpus loads");
	let Poster = FixturePoster::Shipped();
	let State = Racing();
	let _ = BlockOn(Decide(&*Poster, &Corpus, &State, Bound())).expect("the fixture answers");

	let Sent = Poster.Only();

	// The bytes, not the value: a poster puts `Text()` on the wire, so the
	// assertion is made on what the endpoint would actually receive.
	let Text = Sent.Text();
	println!("posted {} · {} bytes · {Text}", Sent.Path(), Text.len());
	let Body:Value = serde_json::from_str(&Text).expect("the posted body is JSON");
	let Object = Body.as_object().expect("a decision request is a JSON object");

	let mut Named:Vec<&str> = Object.keys().map(String::as_str).collect();
	Named.sort_unstable();
	assert_eq!(Named, Keys, "the body is exactly three keys and nothing else · {Body}");

	for Field in Chat {
		assert!(
			!Object.contains_key(Field),
			"`{Field}` is a chat-completions field, and this is a decision request · {Body}"
		);
	}

	assert_eq!(
		Sent.Path(),
		"/ai/run/@cf/cloudflare/clef-flash",
		"the route is the decision endpoint"
	);
	assert_eq!(Body["model"], json!("clef-flash"), "the body names the bare model");
	assert_eq!(
		Sent.Body(),
		Body,
		"the bytes a poster sends are the request's own body, not a second description of it"
	);

	// The state is the lane's fields and nothing else the reading carried.
	let Lane = SelectLane(&Corpus, &Context::from(&State)).expect("exactly one lane holds");
	assert_eq!(
		Body["state"],
		State.Keep(&Lane.Fields).Body(),
		"the state is the lane's own selection of the reading, fields {0:?}",
		Lane.Fields
	);
	assert_eq!(
		Body["state"],
		json!({ "speed": 200, "yaw": 12 }),
		"with the values the reading carried"
	);

	// The questions are the template's, whole and by their corpus names.
	let Asked = Body["questions"].as_object().expect("`questions` is a map keyed by name");
	assert_eq!(
		Asked.keys().map(String::as_str).collect::<Vec<&str>>(),
		Sent.Names(),
		"the body's question names are the request's own, not a second description"
	);
	assert_eq!(
		Asked["course"]["type"],
		json!("choice"),
		"the question travels as the corpus wrote it"
	);
	assert_eq!(
		Asked["disagreement"]["criteria"],
		json!(["none", "slight", "clear"]),
		"including the scale the reply left out and the request filled in"
	);
}

/// One reply that is not an answer: why the case exists, the body, the word the
/// decoder must use for it, and a check on the variant's own payload.
///
/// A named type rather than the tuple spelled out, because a four-part tuple of
/// which one part is a function pointer is a type nobody reads twice.
type Case = (&'static str, &'static str, &'static str, fn(&DecodeError) -> bool);

#[test]
fn a_reply_this_crate_cannot_read_surfaces_the_decoders_own_reason() {
	let Corpus = Load(&Directory()).expect("the fixture corpus loads");
	let State = Racing();

	// A choice naming an option its own distribution does not carry: well
	// formed JSON, a valid reply shape, and still not an answer.
	const Stray:&str = r#"{
		"success": true,
		"result": {
			"answers": {
				"course": { "type": "choice", "choice": "nearer", "probabilities": { "closer": 0.7, "farther": 0.3 } }
			},
			"usage": { "input_tokens": 10 }
		}
	}"#;

	let Cases:[Case; 3] = [
		("a body that is not JSON", "the endpoint said no", "malformed", |Why| {
			matches!(Why, DecodeError::Malformed { .. })
		}),
		(
			"a reply that carries no result",
			r#"{"success": true}"#,
			"missing",
			|Why| matches!(Why, DecodeError::Missing { Field, .. } if Field == "result"),
		),
		(
			"a choice outside its own distribution",
			Stray,
			"unknown-choice",
			|Why| matches!(Why, DecodeError::UnknownChoice { Question, Choice, .. } if Question == "course" && Choice.as_deref() == Some("nearer")),
		),
	];

	for (Case, Body, Word, Check) in Cases {
		let Poster = Scripted::Answering(Body);
		let Failed = BlockOn(Decide(&*Poster, &Corpus, &State, Bound()))
			.err()
			.unwrap_or_else(|| panic!("{Case} is not an answer"));

		assert_eq!(Failed.Code(), Code::Unparseable, "{Case}: the port reports one code");
		let Cause = Failed
			.Cause()
			.unwrap_or_else(|| panic!("{Case}: the decoder's reason rides inside the port's error"));
		assert_eq!(
			Cause.Name(),
			Word,
			"{Case}: the reason is the decoder's own variant, not a sentence"
		);
		assert!(
			Check(Cause),
			"{Case}: and the variant carries the field it was about · {Cause:?}"
		);
		assert_eq!(
			Failed.Message(),
			Cause.Sentence(),
			"{Case}: the sentence is the decoder's, not a flattened copy of it"
		);
		assert_eq!(Failed.Detail(), Cause.Detail(), "{Case}: and so is what was observed");
		assert_eq!(Poster.Posted().len(), 1, "{Case}: the reply was paid for, so it was posted");
	}
}

#[test]
fn a_state_no_lane_holds_is_refused_before_anything_is_posted() {
	let Corpus = Load(&Directory()).expect("the fixture corpus loads");
	let Poster = Scripted::Answering(Fixture::Shipped().Body());

	// `speed` and no `yaw`: every lane's condition reads a field this state
	// does not carry, and a missing field is not a zero - so nothing holds.
	let mut State = State::New();
	State.Set("speed", 200);

	let Failed = BlockOn(Decide(&*Poster, &Corpus, &State, Bound())).expect_err("no lane holds");
	assert_eq!(Failed.Code(), Code::NoMatch, "a selection that names nothing is `no-match`");
	assert!(
		Failed.Detail().contains("speed=200"),
		"the refusal says what it was held against: {}",
		Failed.Detail()
	);
	assert!(
		Poster.Posted().is_empty(),
		"a selection that failed is not a request somebody paid for"
	);
}

#[test]
fn a_poster_that_does_not_answer_is_a_post_failure_not_an_empty_answer() {
	let Corpus = Load(&Directory()).expect("the fixture corpus loads");
	let Poster = Scripted::Failing("the socket closed", "dial tcp: connection refused");

	let Failed = BlockOn(Decide(&*Poster, &Corpus, &Racing(), Bound())).expect_err("the poster did not answer");
	assert_eq!(Failed.Code(), Code::PostFailed);
	assert_eq!(
		Failed.Message(),
		"the socket closed",
		"the poster's sentence is kept as it gave it"
	);
	assert!(
		Failed.Detail().contains("connection refused"),
		"and so is what it observed: {}",
		Failed.Detail()
	);
	assert_eq!(
		Failed.Cause(),
		None,
		"no decoder saw a body, so there is no decoder reason to narrow on"
	);
	assert_eq!(Poster.Posted().len(), 1, "the request left, and is the one that failed");
}

#[test]
fn the_entry_point_takes_a_trait_object_as_well_as_a_concrete_poster() {
	let Corpus = Load(&Directory()).expect("the fixture corpus loads");
	let Poster = FixturePoster::Shipped();

	// A caller holding `Arc<dyn Poster>` - the decider's own shape - pays one
	// indirection rather than a monomorphised copy per adapter.
	let Port:&dyn Poster = &*Poster;
	let Answer = BlockOn(Decide(Port, &Corpus, &Racing(), Bound())).expect("the fixture answers");
	assert_eq!(
		Answer.Choice("course").map(|(Choice, _)| Choice),
		Poster.Fixture().Choice("course")
	);
	assert_eq!(
		Poster.Only().Names(),
		vec!["course", "disagreement"],
		"one call, one post, whichever spelling of the poster was used"
	);
}

#[test]
fn deciding_on_the_state_is_deciding_on_a_context_of_it() {
	let Corpus = Load(&Directory()).expect("the fixture corpus loads");
	let State = Racing();

	let Narrow = FixturePoster::Shipped();
	let Wide = FixturePoster::Shipped();
	let One:Answer = BlockOn(Decide(&*Narrow, &Corpus, &State, Bound())).expect("the fixture answers");
	let Two:Answer =
		BlockOn(DecideOn(&*Wide, &Corpus, &Context::from(&State), &State, Bound())).expect("the fixture answers");

	assert_eq!(One, Two, "the narrow form is the wider one over the state's own fields");
	assert_eq!(Narrow.Only(), Wide.Only(), "and it composes the same request, byte for byte");
}

#[test]
fn a_lane_may_read_a_context_wider_than_the_state_it_sends() {
	let Corpus = Load(&Directory()).expect("the fixture corpus loads");

	// `blocked` reads `blocked_ticks`, a condition the ingestion derives rather
	// than a reading it takes. A state carrying only the readings cannot select
	// it; a context that also carries the derived count can - and the request
	// still sends only what the lane names, with the unread field as `null`.
	let mut State = State::New();
	State.Set("speed", 0);
	State.Set("surface", "ice");
	let Context = Context::from(&State).With("blocked_ticks", 5);

	let Narrow = Scripted::Answering(Fixture::Shipped().Body());
	let Failed = BlockOn(Decide(&*Narrow, &Corpus, &State, Bound())).expect_err("the readings alone select nothing");
	assert_eq!(Failed.Code(), Code::NoMatch);
	assert!(Narrow.Posted().is_empty(), "and nothing was posted for them");

	let Wide = Scripted::Answering(Fixture::Shipped().Body());
	let Answer =
		BlockOn(DecideOn(&*Wide, &Corpus, &Context, &State, Bound())).expect("the derived counter selects `blocked`");
	assert_eq!(
		Answer.Choice("course").map(|(Choice, _)| Choice),
		Some("closer"),
		"and the canned reply is read the same way"
	);

	// The scripted fake records the path and the body it would have sent, which
	// is what an assertion about a composed request has to be made on.
	let (Path, Text) = Wide.Only();
	assert_eq!(Path, "/ai/run/@cf/cloudflare/clef-flash", "the route is the decision endpoint");
	let Sent:Value = serde_json::from_str(&Text).expect("the posted body is JSON");
	assert_eq!(
		Sent["questions"]
			.as_object()
			.map(|Asked| Asked.keys().cloned().collect::<Vec<String>>()),
		Some(vec!["course".to_string(), "disagreement".to_string()]),
		"`blocked` serves `overview`, the template that asks both"
	);
	assert_eq!(
		Sent["state"],
		json!({ "speed": 0, "surface": "ice", "blocked_ticks": null }),
		"the lane names a derived field the state does not carry, and it travels as null rather than omitted"
	);
}

#[test]
fn the_crate_depends_on_nothing_that_can_open_a_socket() {
	// A socket needs a crate to open one with, so the dependency list is the
	// whole of the egress a test binary can reach. `Poster` is the one seam
	// out, and this crate never implements it - which is what makes "the suite
	// needs no network" a property of the manifest rather than a habit of the
	// tests that happen to exist today.
	let Manifest = include_str!("../Cargo.toml");
	let Section = Manifest
		.split("[dependencies]")
		.nth(1)
		.expect("the manifest declares a `[dependencies]` table")
		.split('[')
		.next()
		.expect("the section ends at the next table");
	let Named:Vec<&str> = Section
		.lines()
		.map(str::trim)
		.filter(|Line| !Line.is_empty() && !Line.starts_with('#'))
		.map(|Line| Line.split('=').next().unwrap_or_default().trim())
		.collect();
	assert_eq!(
		Named,
		vec!["agentic-runtime", "serde", "serde_json"],
		"the crate's dependencies are its own value layer and the runtime's futures - add one and say why here"
	);
}

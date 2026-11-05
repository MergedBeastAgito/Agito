//! The whole entry point, once: a corpus on disk, a lane chosen by a rule, a
//! request composed and posted, and the typed answer that comes back.
//!
//! This is the only target that runs every step in one pass - [`Load`],
//! [`Decide::Select`], [`Decide::Decide`], and inside it compose, post, parse
//! and meter - and it does so against the shipped fixture
//! (`Test/Fixture/Clef/Response.json`, read through `Test/Fake.rs`), so the
//! bytes it reads are a document a person edits and the poster that returns
//! them has no socket to reach. Nothing here opens a connection, and nothing
//! here reads the clock beyond a one-second bound that never expires.
//!
//! # What it holds down, in the order the run happens
//!
//! - **the lane the rules select**, printed before anything is asserted, so a
//!   failure names the lane it was about instead of leaving the reader to
//!   re-derive it;
//! - **the typed answer**: the choice the document names, a share for *every*
//!   option summing to one, and the score the document also answered;
//! - **the meter**: the fixture's own `budget` as the run's ceiling, one
//!   request spent, nothing failed, and the token count the reply reported;
//! - **the composed body**, asserted on the bytes a poster would send rather
//!   than on the value it was built from: exactly `{model, state, questions}`,
//!   with no chat-completions field anywhere in it.
//!
//! # Why the shape guard is a function of its own
//!
//! [`Shape`] is asserted against bodies that must fail it, in two cases at the
//! bottom of this file. An assertion nobody has watched fail is an assertion
//! nobody knows is wired up: a guard that had quietly been reading the wrong
//! map would pass every case here and prove nothing. The two refusals are the
//! two the card names - a chat-completions field, and a missing key - and each
//! is checked for the sentence it panics with, not merely that it panicked.

#![allow(non_snake_case, non_upper_case_globals)]

mod Fake;

use std::{
	path::{Path, PathBuf},
	sync::Arc,
	time::Duration,
};

use AgenticLane::{
	Answer,
	Answered,
	Budget,
	Create,
	Decide,
	Decider,
	Load,
	Options,
	Poster,
	Rule,
	Rules,
	Selection,
	State,
	When,
};
use AgenticRuntime::{BlockOn, Call, Never, Wall};
use Fake::FixturePoster;
use serde_json::{Value, json};

/// The corpus this crate ships beside its tests: two lanes, two templates, and
/// an observation that declares the fields both lanes send.
fn Directory() -> PathBuf { Path::new(env!("CARGO_MANIFEST_DIR")).join("Test/Fixture/Corpus") }

/// One call's bound, on the wall clock. The fixture answers immediately, so
/// this never expires - it exists because the port takes a [`Call`] and a test
/// that passed no bound would not be exercising the signature a caller uses.
fn Bound() -> Call { Call::Within(Duration::from_secs(1), &Wall, Arc::new(Never)) }

/// The observation one run is asked about: the three fields the `wide` lane
/// sends, all present, so the request carries numbers rather than nulls.
fn Observed() -> State {
	let mut State = State::New();
	State.Set("depth", 60);
	State.Set("heading", 90);
	State.Set("note", "the mark is close");
	State
}

/// Two lanes, chosen by depth: `wide` above 40 and `narrow` otherwise.
///
/// The rule is the caller's rather than the corpus's because a condition is
/// often a counter the caller keeps - and because this is the entry point's
/// own policy argument, which a test of the entry point has to supply.
fn Lanes() -> Rules { Rules::New(vec![Rule::New("wide", When::Above("depth", 40.0))]).Falling("narrow") }

/// The request's three keys, which are also the body's three keys.
const Keys:[&str; 3] = ["model", "state", "questions"];

/// The chat-completions vocabulary, none of which a decision request may carry.
///
/// Listed rather than described because the failure this guards against is a
/// payload somebody copied from a chat client: `messages` and `temperature`
/// are what such a payload arrives with, and the endpoint answers 400 with no
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

/// Assert that one body is a decision request and not a chat-completions one.
///
/// Three statements, in the order that gives the loudest sentence: every key
/// the request has must be present, no chat-completions field may appear, and
/// nothing else may appear under any other name. The third is what catches a
/// fourth field somebody added to the request type - `deny_unknown_fields`
/// stops the crate *reading* such a body, and this stops it *writing* one.
fn Shape(Body:&Value) {
	let Object = Body
		.as_object()
		.unwrap_or_else(|| panic!("a decision request is a JSON object, and this body is {Body}"));
	for Key in Keys {
		assert!(
			Object.contains_key(Key),
			"the body carries no `{Key}`: a decision request is {{{}, {}, {}}} and nothing else · {Body}",
			Keys[0],
			Keys[1],
			Keys[2]
		);
	}
	for Field in Chat {
		assert!(
			!Object.contains_key(Field),
			"`{Field}` is a chat-completions field, and this is a decision request · {Body}"
		);
	}
	let Stray:Vec<&str> = Object.keys().map(String::as_str).filter(|Key| !Keys.contains(Key)).collect();
	assert!(
		Stray.is_empty(),
		"the body carries {Stray:?}, which a decision request does not · {Body}"
	);
}

/// One complete pass through the entry point, kept as the values a claim is
/// made about.
///
/// The decider is returned rather than dropped because the meter is read from
/// it: a run's spend is only observable on the value that did the spending.
struct Pass {
	/// The poster the run was wired with, holding every request it was handed.
	Poster:Arc<FixturePoster>,
	/// The port's value, for its corpus and its meter.
	Decider:Decider,
	/// The lane and template the rules selected.
	Selection:Selection,
	/// The observation both halves read.
	State:State,
	/// What came back, typed.
	Answer:Answer,
}

/// Run the entry point once: load, select, decide.
///
/// The ceiling is the fixture's own `budget`, which is what that number is for
/// - the poster does not enforce it, so the run's meter is the only authority
/// over spend and a test that wants the ceiling has to configure it.
fn Run() -> Pass {
	let Poster = FixturePoster::Shipped();
	let Decider = Create(
		Load(&Directory()).expect("the fixture corpus loads"),
		Poster.clone() as Arc<dyn Poster>,
		Options {
			Model:"clef-flash".to_string(),
			Lanes:Lanes(),
			Templates:Rules::default(),
			Budget:Budget::New(Poster.Budget(), 0),
		},
	);
	let State = Observed();
	let Selection = Decider.Select(&State).expect("a lane holds for a depth of 60");
	let Answer = BlockOn(Decider.Decide(&Selection, &State, Bound())).expect("the fixture answers without a socket");
	Pass { Poster, Decider, Selection, State, Answer }
}

#[test]
fn the_entry_point_answers_the_shipped_fixture_without_a_socket() {
	let Pass { Poster, Decider, Selection, State:_, Answer } = Run();
	let Fixture = Poster.Fixture();

	// Printed before anything is asserted: a failure below says which lane it
	// was about without the reader having to re-run with a trace.
	println!("lane selected: {} · {}", Selection.Name(), Selection.Lane.Why);
	println!("meter: {}", Decider.Budget().Line());

	assert_eq!(
		Selection.Name(),
		"wide / overview",
		"the rule over depth chose the wide lane, and its own first template"
	);

	// --- the typed answer -------------------------------------------------
	let Expected = Fixture.Choice("course").expect("the canned reply chose an option");
	let (Choice, Share) = Answer.Choice("course").expect("the reply answered `course`");
	println!("choice: {Choice} at {Share}");
	assert_eq!(
		Choice, Expected,
		"the typed answer names the option the shipped document chose"
	);

	let Answered::Choice { Choice:Chosen, Probabilities } = Answer
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
	assert!(Probabilities.values().all(|Share| (0.0..=1.0).contains(Share)), "{Probabilities:?}");

	// The document also answers a score, and the reply omits its legend - so
	// this is the end-to-end proof that the request filled it in.
	assert_eq!(Answer.Score("disagreement"), Some(2.0), "the second answer is a score of 2");
	assert_eq!(
		Answer.Position("disagreement"),
		Some(1.0),
		"2 on the request's three-word scale is the top"
	);

	// --- the meter --------------------------------------------------------
	let Meter = Decider.Budget();
	assert_eq!(
		Meter.MaxRequests,
		Fixture.Budget(),
		"the run's ceiling is the fixture's own budget, which is what that number is for"
	);
	assert!(Meter.MaxRequests >= 1, "a canned run is worth a request, saw {}", Meter.MaxRequests);
	assert_eq!(Meter.Requests, 1, "one round trip is one request");
	assert!(
		Meter.Requests <= Meter.MaxRequests,
		"the run stayed inside the ceiling it was given: {} of {}",
		Meter.Requests,
		Meter.MaxRequests
	);
	assert_eq!(Meter.Failed, 0, "nothing failed: {:?}", Meter.Last);
	assert_eq!(Meter.MaxTokens, 0, "the fixture's budget is a request count, not a token ceiling");

	let Reported = Fixture.Response()["result"]["usage"]["input_tokens"]
		.as_u64()
		.expect("the canned reply reports the input tokens it spent");
	assert_eq!(Answer.Usage.Input, Reported, "the answer carries what the reply reported");
	assert_eq!(Meter.Input, Reported, "and the meter charged exactly that");
	assert_eq!(
		Answer.Usage.Remaining,
		None,
		"a decoder given no token ceiling invents no remaining"
	);
	assert_eq!(Answer.Model, "clef-flash", "the answer names the model the lane asked");
}

#[test]
fn the_composed_request_is_exactly_model_state_questions() {
	let Pass { Poster, Selection, State, Answer:_, Decider:_ } = Run();
	let Posted = Poster.Only();

	// The bytes, not the value: a poster puts `Text()` on the wire, so the
	// assertion is made on what the endpoint would actually receive.
	let Text = Posted.Text();
	println!("posted {} · {} bytes · {Text}", Posted.Path(), Text.len());

	let Sent:Value = serde_json::from_str(&Text).expect("the posted body is JSON");
	Shape(&Sent);

	assert_eq!(
		Posted.Path(),
		"/ai/run/@cf/cloudflare/clef-flash",
		"the route is the decision endpoint, never a chat-completions one"
	);
	assert_eq!(Sent["model"], json!("clef-flash"), "the body names the bare model");

	// The state is the lane's three fields and nothing else the observation
	// carried - the lane is a request shape, and this is where that shows.
	assert_eq!(
		Sent["state"],
		json!({ "depth": 60, "heading": 90, "note": "the mark is close" }),
		"the state is the lane's fields, with the values the observation carried"
	);
	assert_eq!(
		Sent["state"],
		State.Keep(&Selection.Lane.Fields).Body(),
		"and it is the lane's own selection of the observation"
	);

	// The questions are the template's, whole and by their corpus names.
	assert_eq!(
		Posted.Names(),
		vec!["course", "disagreement"],
		"the request asks what the selected template asks"
	);
	let Asked = Sent["questions"].as_object().expect("`questions` is a map keyed by name");
	assert_eq!(
		Asked.keys().map(String::as_str).collect::<Vec<&str>>(),
		Posted.Names(),
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

	// Nothing else reached the poster, and nothing reached it twice.
	assert_eq!(Poster.Posted().len(), 1, "one round trip is one post");
}

#[test]
#[should_panic(expected = "chat-completions field")]
fn the_shape_guard_refuses_a_chat_completions_body() {
	Shape(&json!({
		"model": "clef-flash",
		"state": {},
		"questions": {},
		"messages": [{ "role": "user", "content": "which way?" }],
		"temperature": 0.2
	}));
}

#[test]
#[should_panic(expected = "carries no `questions`")]
fn the_shape_guard_refuses_a_body_missing_a_key() {
	Shape(&json!({ "model": "clef-flash", "state": {} }));
}

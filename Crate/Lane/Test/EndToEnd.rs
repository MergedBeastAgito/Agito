//! The whole entry point, once, against the shipped reply.
//!
//! Two ways into this crate, and both are exercised here because they are two
//! halves of one claim. [`Decider`] is the value a consumer holds - corpus,
//! poster, rules and the meter - and it is the one that can report what a run
//! spent. [`Decide`] is the same pipeline with the corpus's own conditions
//! choosing the lane, for a caller that has no rules of its own to write. A
//! target that covered only one would leave the other's composition untested
//! end-to-end, and composition is where a lane, a template and a model have to
//! agree.
//!
//! # What makes this an end-to-end test and not a unit test
//!
//! Every step runs in one pass, against the files this crate ships beside its
//! tests: the corpus is read from disk by [`Load`], the lane is chosen by a
//! condition, the request is composed, posted through the [`Poster`] port,
//! decoded and metered. The poster is `Test/Fake.rs`'s [`FixturePoster`], which
//! answers from `Test/Fixture/Clef/Response.json` - a document a person edits
//! without touching Rust - and holds no socket, no clock and no credential. The
//! only clock in the file is a one-second bound that never expires, present
//! because the port takes a [`Call`] and a test that passed none would not be
//! exercising the signature a caller uses.
//!
//! # The claims, in the order a run makes them
//!
//! - **the lane the conditions select**, printed before anything is asserted, so
//!   a failure names the lane it was about instead of leaving the reader to
//!   re-derive it;
//! - **the typed answer**: the choice the document names, a share for *every*
//!   option summing to one, the score the document also answered, and the token
//!   count it reported;
//! - **the meter**: the fixture's own `budget` as the run's ceiling, one request
//!   spent, nothing failed, and no token ceiling invented;
//! - **the composed body**, asserted on the bytes a poster would send rather
//!   than on the value it was built from: exactly `{model, state, questions}`,
//!   with no chat-completions field anywhere in it.
//!
//! # Why the shape guard is a function of its own
//!
//! [`Shape`] is also asserted against bodies that must fail it, in the two cases
//! at the bottom of this file. An assertion nobody has watched fail is an
//! assertion nobody knows is wired up: a guard that had quietly been reading the
//! wrong map would pass every case above and prove nothing. The two refusals are
//! the two the card names - a chat-completions field, and a missing key - and
//! each is checked for the sentence it panics with, not merely that it panicked.
//!
//! # Why this is a target of its own
//!
//! `Test/Lane.rs` covers the same pipeline against the *scripted* fake and a
//! hand-written body, one claim per case; `Test/ClefFixture.rs` holds the shipped
//! document to its own promises. This file is the join: the fixture answering the
//! composed request, so a document edited into something the pipeline cannot read
//! fails a suite named after the round trip rather than an assertion somewhere in
//! the middle of another one.
//!
//! The path is `Test/EndToEnd.rs` rather than `tests/end_to_end.rs` because this
//! crate registers every target explicitly in `Cargo.toml`, and a target named
//! after the file it holds is what makes `cargo test -p agentic-lane --test
//! EndToEnd` the command a failure report can name.

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
	Context,
	Create,
	Decide,
	DecidePort,
	Decider,
	Load,
	Options,
	Poster,
	Rule,
	Rules,
	SelectLane,
	Selection,
	State,
	When,
};
use AgenticRuntime::{BlockOn, Call, Never, Wall};
use Fake::FixturePoster;
use serde_json::{Value, json};

/// The corpus whose lanes are chosen by the caller's rules: two lanes, two
/// templates, and an observation that declares the fields both lanes send.
fn Directory() -> PathBuf { Path::new(env!("CARGO_MANIFEST_DIR")).join("Test/Fixture/Corpus") }

/// The corpus whose lanes state their own conditions, so the lane is a JSON edit
/// rather than a rule the caller writes.
fn Conditional() -> PathBuf { Path::new(env!("CARGO_MANIFEST_DIR")).join("Test/Fixture/Select") }

/// One call's bound, on the wall clock. The fixture answers immediately, so this
/// never expires.
fn Bound() -> Call { Call::Within(Duration::from_secs(1), &Wall, Arc::new(Never)) }

/// The observation the rules corpus is asked about: the three fields the `wide`
/// lane sends, all present, so the request carries numbers rather than nulls.
fn Observed() -> State {
	let mut State = State::New();
	State.Set("depth", 60);
	State.Set("heading", 90);
	State.Set("note", "the mark is close");
	State
}

/// The observation the conditional corpus is asked about: fast and turning, which
/// is the one lane of its four whose condition holds.
fn Driving() -> State {
	let mut State = State::New();
	State.Set("speed", 150);
	State.Set("yaw", 20);
	State
}

/// Two lanes, chosen by depth: `wide` above 40 and `narrow` otherwise.
///
/// The rule is the caller's rather than the corpus's because this is the entry
/// point's own policy argument - a condition is often a counter the caller keeps,
/// and a test of the entry point has to supply the policy it is handed.
fn Lanes() -> Rules { Rules::New(vec![Rule::New("wide", When::Above("depth", 40.0))]).Falling("narrow") }

/// The request's three keys, which are also the body's three keys.
const Keys:[&str; 3] = ["model", "state", "questions"];

/// The chat-completions vocabulary, none of which a decision request may carry.
///
/// Listed rather than described because the failure this guards against is a
/// payload somebody copied from a chat client: `messages` and `temperature` are
/// what such a payload arrives with, and the endpoint answers 400 with no
/// diagnosis in it when it gets one.
const Chat:[&str; 13] = [
	"messages",
	"role",
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
/// Three statements, in the order that gives the loudest sentence: every key the
/// request must have is present, no chat-completions field may appear, and nothing
/// else may appear under any other name. The third is what catches a fourth field
/// somebody added to the request type - `deny_unknown_fields` stops the crate
/// *reading* such a body, and this stops it *writing* one.
fn Shape(Body:&Value) {
	let Object = Body
		.as_object()
		.unwrap_or_else(|| panic!("a decision request is a JSON object, and this body is {Body}"));
	for Key in Keys {
		assert!(
			Object.contains_key(Key),
			"the body carries no `{Key}`: a decision request is {{model, state, questions}} and nothing else · {Body}"
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

/// The path a decision request goes to, which is never a chat-completions route.
const Route:&str = "/ai/run/@cf/cloudflare/clef-flash";

/// One complete pass through the metered entry point, kept as the values a claim
/// is made about.
///
/// The decider is returned rather than dropped because the meter is read from it:
/// a run's spend is only observable on the value that did the spending.
struct Pass {
	/// The poster the run was wired with, holding every request it was handed.
	Poster:Arc<FixturePoster>,
	/// The port's value, for its corpus and its meter.
	Decider:Decider,
	/// The lane and template the rules selected.
	Selection:Selection,
	/// The observation the run read.
	State:State,
	/// What came back, typed.
	Answer:Answer,
}

/// Run the metered entry point once: load, select, decide.
///
/// The ceiling is the fixture's own `budget`, which is what that number is for -
/// the poster does not enforce it, so the run's meter is the only authority over
/// spend and a test that wants the ceiling has to configure it.
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
	let Pass { Poster, Decider, Selection, State: _, Answer } = Run();
	let Fixture = Poster.Fixture();

	// Printed before anything is asserted: a failure below says which lane it was
	// about without the reader having to re-run with a trace.
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
	assert_eq!(Choice, Expected, "the typed answer names the option the shipped document chose");

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
		"every share is a probability · {Probabilities:?}"
	);

	// The document also answers a score, and the reply omits its legend - so this
	// is the end-to-end proof that the request filled it in.
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
	assert!(
		Meter.MaxRequests >= 1,
		"a canned run is worth a request, saw {}",
		Meter.MaxRequests
	);
	assert_eq!(Meter.Requests, 1, "one round trip is one request");
	assert!(
		Meter.Requests <= Meter.MaxRequests,
		"the run stayed inside the ceiling it was given: {} of {}",
		Meter.Requests,
		Meter.MaxRequests
	);
	assert_eq!(Meter.Failed, 0, "nothing failed: {:?}", Meter.Last);
	assert_eq!(
		Meter.MaxTokens, 0,
		"the fixture's budget is a request count, not a token ceiling"
	);

	let Reported = Fixture.Response()["result"]["usage"]["input_tokens"]
		.as_u64()
		.expect("the canned reply reports the input tokens it spent");
	assert_eq!(Answer.Usage.Input, Reported, "the answer carries what the reply reported");
	assert_eq!(Meter.Input, Reported, "and the meter charged exactly that");
	assert_eq!(
		Answer.Usage.Remaining, None,
		"a decoder given no token ceiling invents no remaining"
	);
	assert_eq!(Answer.Model, "clef-flash", "the answer names the model the lane asked");
}

#[test]
fn the_composed_request_is_exactly_model_state_questions() {
	let Pass { Poster, Selection, State, Answer: _, Decider: _ } = Run();
	let Posted = Poster.Only();

	// The bytes, not the value: a poster puts `Text()` on the wire, so the
	// assertion is made on what the endpoint would actually receive.
	let Text = Posted.Text();
	println!("posted {} · {} bytes · {Text}", Posted.Path(), Text.len());

	let Sent:Value = serde_json::from_str(&Text).expect("the posted body is JSON");
	Shape(&Sent);

	assert_eq!(
		Posted.Path(),
		Route,
		"the route is the decision endpoint, never a chat-completions one"
	);
	assert!(
		!Posted.Path().contains("chat"),
		"a chat-completions route would answer this body with a 400 · {}",
		Posted.Path()
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

/// The other way in: the corpus's own conditions choose the lane, so a lane added
/// as a JSON edit is reachable without a rule in Rust.
///
/// This is the case the caller-rules entry point cannot express - four lanes with
/// mutually exclusive conditions and no fallback - and the case where a wrong lane
/// is silent: the request would still be well formed, and only its fields and
/// questions would say which lane actually ran. So the assertions are made on the
/// posted body, not on the returned answer alone.
#[test]
fn the_corpus_condition_entry_point_selects_composes_and_answers() {
	let Poster = FixturePoster::Shipped();
	let Corpus = Load(&Conditional()).expect("the conditional corpus loads");
	let State = Driving();

	// Printed before the run: a failure below names the lane it was about.
	let Lane = SelectLane(&Corpus, &Context::from(&State)).expect("exactly one lane holds for a fast turn");
	println!("lane selected: {} when {}", Lane.Name, Lane.Select.Line());

	let Answer = BlockOn(Decide(&*Poster, &Corpus, &State, Bound())).expect("the fixture answers without a socket");
	let (Choice, Share) = Answer.Choice("course").expect("the reply answered `course`");
	println!("choice: {Choice} at {Share}");

	assert_eq!(Lane.Name, "fast", "speed above 120 and yaw above 5 is the fast lane");
	assert_eq!(
		Choice,
		Poster.Fixture().Choice("course").expect("the canned reply chose an option"),
		"the answer names the option the shipped document chose"
	);
	assert_eq!(
		Answer.Position("disagreement"),
		Some(1.0),
		"the request's scale reached the answer"
	);

	// The request is the lane's own shape: its two fields, and the questions of
	// the template that lane serves.
	let Posted = Poster.Only();
	let Sent:Value = serde_json::from_str(&Posted.Text()).expect("the posted body is JSON");
	Shape(&Sent);
	assert_eq!(Posted.Path(), Route, "the corpus path reaches the same decision route");
	assert_eq!(
		Sent["state"],
		json!({ "speed": 150, "yaw": 20 }),
		"the fast lane sends two fields, not the whole observation"
	);
	assert_eq!(
		Posted.Names(),
		vec!["course", "disagreement"],
		"and asks what its own template asks, not what another lane's does"
	);
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
fn the_shape_guard_refuses_a_body_missing_a_key() { Shape(&json!({ "model": "clef-flash", "state": {} })); }

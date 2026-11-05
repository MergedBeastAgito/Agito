//! `agentic.lane.decide`, exercised against a corpus on disk and a poster that
//! answers from a captured body.
//!
//! No network and no browser: the corpus is the fixture beside this test, the
//! reply is a string, and a cancelled call is a signal the test fired. The
//! poster is `Test/Fake.rs` - it answers from a captured body and records what
//! it was asked, which is how a call that was never posted stays visible. Every
//! case below is a claim the crate makes in prose somewhere, stated as an
//! assertion instead - and the refusals matter as much as the answers, because
//! a corpus that disagrees with itself is the failure this crate exists to find
//! before a request is paid for.
//!
//! The vocabulary is deliberately generic: a lane, a template, a reading. If a
//! game or a page name appeared here, the crate would be claiming a caller it
//! does not have.

#![allow(non_snake_case, non_upper_case_globals)]

mod Fake;

use std::{
	path::{Path, PathBuf},
	sync::Arc,
	time::Duration,
};

use AgenticLane::{
	Answered,
	Ask,
	Bare,
	Budget,
	Code,
	Compose,
	Create,
	DecidePort,
	Load,
	Options,
	Parse,
	Poster,
	Price,
	Read,
	Route,
	Rule,
	Rules,
	Select,
	State,
	When,
};
use AgenticRuntime::{BlockOn, Call, Never, Signal, Wall};
use serde_json::{Value, json};
use Fake::{Captured, Fake as Scripted};

/// One call's budget, on the wall clock.
fn Budget() -> Call { Call::Within(Duration::from_secs(1), &Wall, Arc::new(Never)) }

/// The fixture corpus this crate ships beside its test.
fn Directory() -> PathBuf { Path::new(env!("CARGO_MANIFEST_DIR")).join("Test/Fixture/Corpus") }

/// The observation and the questions of a corpus small enough to read inline,
/// for the cases that mutate one of them into a refusal.
const MinimalObservation:&str = r#"{ "state": { "depth": { "unit": "m" } } }"#;

/// One template, one question, one lane - the smallest corpus that loads.
const MinimalQuestions:&str = r#"{
	"version": 1,
	"templates": {
		"focused": {
			"questions": {
				"course": {
					"type": "choice",
					"instructions": "Which way?",
					"criteria": { "closer": "toward", "farther": "away" }
				}
			}
		}
	},
	"lanes": { "narrow": { "fields": ["depth"], "templates": ["focused"] } }
}"#;

/// A state that carries one number, for a rule to read.
fn Moving(Depth:f64) -> State {
	let mut State = State::New();
	State.Set("depth", Depth);
	State
}

/// The manifest's error list, in the order it names them, against the type's
/// own list.
///
/// `agentic verify --module` compares the same two things, but it compares them
/// with a built binary; this holds the claim where the crate is edited, so a
/// code added to `Code::All()` and not to `module.json` fails the suite rather
/// than a release check. Order is part of the claim: the manifest's list is the
/// order a harness prepares, and two lists that agree only as sets can still
/// describe different sequences.
#[test]
fn the_port_declares_exactly_the_errors_the_manifest_promises() {
	let Declared:Vec<&str> = Code::All().into_iter().map(Code::Name).collect();
	assert_eq!(
		Declared,
		vec![
			"corpus-unreadable",
			"corpus-invalid",
			"unknown-lane",
			"no-match",
			"budget-exhausted",
			"post-failed",
			"unparseable",
		]
	);
	let Manifest:Value = serde_json::from_str(include_str!("../module.json")).expect("the manifest parses");
	let Named:Vec<&str> = Manifest["provides"]["errors"]
		.as_array()
		.expect("the manifest names its errors as a list")
		.iter()
		.map(|Code| Code.as_str().expect("every code is a string"))
		.collect();
	assert_eq!(Named, Declared, "the manifest names the same codes in the same order");
}

#[test]
fn the_fixture_corpus_loads_and_is_checked_against_itself() {
	let Corpus = Load(&Directory()).expect("the fixture corpus loads");
	assert_eq!(Corpus.Version(), 2);
	assert_eq!(Corpus.Lanes().count(), 2, "two lanes");
	assert_eq!(Corpus.Templates().count(), 2, "two templates");
	assert!(Corpus.Observed("depth"), "the observation declares depth");
	assert!(!Corpus.Observed("unobserved"), "and nothing it did not declare");
	let Narrow = Corpus.Lane("narrow").expect("narrow is declared");
	assert_eq!(Narrow.Fields, vec!["depth".to_string()]);
	assert_eq!(Narrow.Templates, vec!["focused".to_string()]);
	assert_eq!(Narrow.Cadence, "every third observation");
	assert!(!Narrow.Why.is_empty(), "a lane carries the reason it exists");
	let Overview = Corpus.Template("overview").expect("overview is declared");
	assert_eq!(Overview.Questions.len(), 2);
	assert!(Overview.When.starts_with("the whole observation"));
}

#[test]
fn a_lane_that_sends_an_unobserved_field_is_refused() {
	let Questions = r#"{
		"templates": { "focused": { "questions": {
			"course": { "type": "choice", "criteria": { "closer": "toward", "farther": "away" } }
		} } },
		"lanes": { "narrow": { "fields": ["heading"], "templates": ["focused"] } }
	}"#;
	let Why = Read(MinimalObservation, Questions).expect_err("a lane may not send a field nobody declared");
	assert_eq!(Why.Code(), Code::CorpusInvalid);
	assert!(Why.Message().contains("does not carry"), "{}", Why.Message());
	assert!(
		Why.Detail().contains("heading"),
		"the refusal names the field: {}",
		Why.Detail()
	);
	assert!(
		Why.Detail().contains("depth"),
		"the refusal says what was available: {}",
		Why.Detail()
	);
}

#[test]
fn a_lane_that_serves_a_template_nobody_wrote_is_refused() {
	let Questions = r#"{
		"templates": { "focused": { "questions": {
			"course": { "type": "choice", "criteria": { "closer": "toward", "farther": "away" } }
		} } },
		"lanes": { "narrow": { "fields": [], "templates": ["absent"] } }
	}"#;
	let Why = Read(MinimalObservation, Questions).expect_err("a lane serves a template that exists");
	assert_eq!(Why.Code(), Code::CorpusInvalid);
	assert!(Why.Detail().contains("focused"), "{}", Why.Detail());
}

#[test]
fn a_choice_with_no_option_and_a_score_with_no_scale_are_refused() {
	let Empty = r#"{
		"templates": { "focused": { "questions": {
			"course": { "type": "choice", "criteria": {} }
		} } },
		"lanes": { "narrow": { "fields": [], "templates": ["focused"] } }
	}"#;
	let Why = Read(MinimalObservation, Empty).expect_err("a choice with no criteria cannot be answered");
	assert_eq!(Why.Code(), Code::CorpusInvalid);
	assert!(Why.Message().contains("offers no option"), "{}", Why.Message());

	let One = r#"{
		"templates": { "focused": { "questions": {
			"disagreement": { "type": "score", "criteria": ["only"] }
		} } },
		"lanes": { "narrow": { "fields": [], "templates": ["focused"] } }
	}"#;
	let Why = Read(MinimalObservation, One).expect_err("one word is not a scale");
	assert_eq!(Why.Code(), Code::CorpusInvalid);
	assert!(Why.Message().contains("no scale"), "{}", Why.Message());

	let Typeless = r#"{
		"templates": { "focused": { "questions": { "course": { "instructions": "Which way?" } } } },
		"lanes": { "narrow": { "fields": [], "templates": ["focused"] } }
	}"#;
	let Why = Read(MinimalObservation, Typeless).expect_err("a question names its type");
	assert_eq!(Why.Code(), Code::CorpusInvalid);
	assert!(Why.Message().contains("names no type"), "{}", Why.Message());
}

#[test]
fn a_corpus_with_no_lane_can_select_nothing() {
	let Questions = r#"{
		"templates": { "focused": { "questions": {
			"course": { "type": "choice", "criteria": { "closer": "toward", "farther": "away" } }
		} } },
		"lanes": {}
	}"#;
	let Why = Read(MinimalObservation, Questions).expect_err("a corpus with no lane is refused");
	assert_eq!(Why.Code(), Code::CorpusInvalid);
	assert!(Why.Message().contains("declares no lane"), "{}", Why.Message());
}

#[test]
fn a_question_type_this_crate_does_not_interpret_is_carried_not_dropped() {
	let Questions = r#"{
		"templates": { "focused": { "questions": {
			"order": { "type": "rank", "instructions": "Order these.", "criteria": ["a", "b"] }
		} } },
		"lanes": { "narrow": { "fields": [], "templates": ["focused"] } }
	}"#;
	let Corpus = Read(MinimalObservation, Questions).expect("an unknown type is not a refusal");
	let Lane = Corpus.Lane("narrow").expect("narrow is declared");
	let Template = Corpus.Template("focused").expect("focused is declared");
	let Question = Template.Questions.get("order").expect("the question is carried");
	assert_eq!(Question.Type(), "rank");
	let Selection = AgenticLane::Selection { Lane:Lane.clone(), Template:Template.clone() };
	let Request = Compose(&Selection, &State::New()).expect("composed");
	let Body = Request.Body();
	assert_eq!(
		Body["questions"]["order"],
		json!({ "type": "rank", "instructions": "Order these.", "criteria": ["a", "b"] }),
		"the body is the corpus's own, verbatim"
	);
}

#[test]
fn a_corpus_that_is_not_json_is_unreadable_rather_than_empty() {
	let Why = Read("{", MinimalQuestions).expect_err("a broken file is not an empty corpus");
	assert_eq!(Why.Code(), Code::CorpusUnreadable);
	assert!(Why.Message().contains("observation"), "{}", Why.Message());

	let Why = Read(MinimalObservation, "]").expect_err("the question file is read too");
	assert_eq!(Why.Code(), Code::CorpusUnreadable);
	assert!(Why.Message().contains("question file"), "{}", Why.Message());
}

#[test]
fn load_names_the_file_it_could_not_read() {
	let Missing = Directory().join("NoSuchCorpus");
	let Why = Load(&Missing).expect_err("a directory with no corpus is unreadable");
	assert_eq!(Why.Code(), Code::CorpusUnreadable);
	assert!(Why.Detail().contains("Observation.json"), "{}", Why.Detail());
	assert!(
		Why.Detail().contains("NoSuchCorpus"),
		"the path is in the detail: {}",
		Why.Detail()
	);
}

#[test]
fn selection_is_a_function_of_the_state_and_the_rules() {
	let Corpus = Load(&Directory()).expect("the fixture corpus loads");
	let Lanes = Rules::New(vec![Rule::New("narrow", When::Above("depth", 40.0))]).Falling("wide");

	let Deep = Select(&Corpus, &Moving(60.0), &Lanes, &Rules::default()).expect("a lane holds");
	assert_eq!(Deep.Name(), "narrow / focused");

	let Shallow = Select(&Corpus, &Moving(10.0), &Lanes, &Rules::default()).expect("the fallback holds");
	assert_eq!(Shallow.Name(), "wide / overview", "the lane's first template is its fallback");

	let Again = Select(&Corpus, &Moving(60.0), &Lanes, &Rules::default()).expect("a lane holds");
	assert_eq!(Deep, Again, "two calls with the same state select the same lane");

	// A template rule only applies to a lane that serves the template, so one
	// list for every lane is the ordinary case rather than a defect.
	let Templates = Rules::New(vec![Rule::New("focused", When::Above("depth", 100.0))]);
	let Wide = Rules::New(Vec::new()).Falling("wide");
	let Fallback = Select(&Corpus, &Moving(150.0), &Wide, &Templates).expect("a template holds");
	assert_eq!(Fallback.Name(), "wide / focused", "the rule outranks the lane's own order");
	let Narrow = Rules::New(Vec::new()).Falling("narrow");
	let Skipped = Select(&Corpus, &Moving(150.0), &Narrow, &Templates).expect("a lane holds");
	assert_eq!(Skipped.Name(), "narrow / focused");
	let Unserved = Rules::New(vec![Rule::New("overview", When::Always)]);
	let Narrow = Rules::New(Vec::new()).Falling("narrow");
	let Skipped = Select(&Corpus, &Moving(150.0), &Narrow, &Unserved).expect("a lane holds");
	assert_eq!(
		Skipped.Name(),
		"narrow / focused",
		"a rule about another lane's template is skipped"
	);
}

#[test]
fn a_rule_list_with_no_fallback_reports_no_match_instead_of_choosing() {
	let Corpus = Load(&Directory()).expect("the fixture corpus loads");
	let Lanes = Rules::New(vec![Rule::New("wide", When::Above("depth", 1_000.0))]);
	let Why = Select(&Corpus, &Moving(10.0), &Lanes, &Rules::default()).expect_err("nothing matched");
	assert_eq!(Why.Code(), Code::NoMatch);
	assert!(
		Why.Detail().contains("depth=10"),
		"the state is in the detail: {}",
		Why.Detail()
	);
}

#[test]
fn a_rule_naming_a_lane_the_corpus_lacks_is_unknown_lane() {
	let Corpus = Load(&Directory()).expect("the fixture corpus loads");
	let Lanes = Rules::New(vec![Rule::Always("absent")]);
	let Why = Select(&Corpus, &Moving(10.0), &Lanes, &Rules::default()).expect_err("no such lane");
	assert_eq!(Why.Code(), Code::UnknownLane);
	assert!(Why.Detail().contains("narrow"), "{}", Why.Detail());
	assert!(Why.Detail().contains("wide"), "{}", Why.Detail());
}

#[test]
fn the_lane_decides_which_fields_travel_and_a_missing_one_arrives_as_null() {
	let Corpus = Load(&Directory()).expect("the fixture corpus loads");
	let State = {
		let mut State = State::New();
		State.Set("depth", 42);
		State.Set("heading", 90);
		State.Set("note", "the mark is close");
		State
	};

	let Narrow = Rules::New(Vec::new()).Falling("narrow");
	let Selection = Select(&Corpus, &State, &Narrow, &Rules::default()).expect("a lane holds");
	let Request = Compose(&Selection, &State).expect("composed");
	assert_eq!(
		Request.State.Fields().cloned().collect::<Vec<String>>(),
		vec!["depth".to_string()],
		"only the fields the lane names travel"
	);
	assert_eq!(Request.Body()["state"], json!({ "depth": 42 }));

	// The wide lane names three fields and the state carries two of them: the
	// third travels as null, because "not observed" is a fact the model may
	// weigh and an omitted field leaves it guessing whether it was asked.
	let Wide = Rules::New(Vec::new()).Falling("wide");
	let Selection = Select(&Corpus, &State, &Wide, &Rules::default()).expect("a lane holds");
	let Partial = {
		let mut Partial = State::New();
		Partial.Set("depth", 42);
		Partial.Set("heading", 90);
		Partial
	};
	let Request = Compose(&Selection, &Partial).expect("composed");
	assert_eq!(Request.Body()["state"], json!({ "depth": 42, "heading": 90, "note": null }));
}

#[test]
fn the_body_names_the_bare_model_and_the_path_carries_the_prefix() {
	assert_eq!(Bare("clef-flash"), "clef-flash");
	assert_eq!(Bare("@cf/cloudflare/clef-flash"), "clef-flash", "either spelling is accepted");
	assert_eq!(Bare("  @cf/cloudflare/clef  "), "clef", "and surrounding space is not a model");
	assert_eq!(
		Bare(""),
		"clef-flash",
		"an empty name falls back rather than making an empty path"
	);
	assert_eq!(Price("clef"), 0.24);
	assert_eq!(Price("unknown"), Price("clef-flash"), "an unknown model is not priced at zero");

	let Corpus = Load(&Directory()).expect("the fixture corpus loads");
	let State = Moving(60.0);
	let Lanes = Rules::New(Vec::new()).Falling("narrow");
	let Selection = Select(&Corpus, &State, &Lanes, &Rules::default()).expect("a lane holds");
	// The model is the lane's now, not an argument: the fixture corpus names
	// none, so the crate's own fallback stands in.
	let Request = Compose(&Selection, &State).expect("composed");
	assert_eq!(Request.Model, "clef-flash");
	assert_eq!(Request.Path(), format!("{Route}clef-flash"));
	assert_eq!(Request.Path(), "/ai/run/@cf/cloudflare/clef-flash");
	assert_eq!(Request.Body()["model"], "clef-flash");
	assert_eq!(Request.Names(), vec!["course"]);
}

#[test]
fn a_reply_is_kept_as_a_distribution_not_a_winner() {
	let Answer = Parse(Captured).expect("the captured reply parses");
	assert_eq!(Answer.Model, "clef-flash");
	assert_eq!(Answer.Choice("course"), Some(("closer", 0.62)));
	assert_eq!(Answer.Probability("course", "farther"), Some(0.38), "the rival's share is kept");
	assert_eq!(Answer.Usage.Input, 375);
	assert_eq!(Answer.Usage.Output, 0);
	assert_eq!(Answer.Score("disagreement"), Some(2.0));
	assert_eq!(Answer.Position("disagreement"), None, "the reply named no legend to divide by");
	assert_eq!(Answer.Answer("absent"), None);
	assert_eq!(Answer.Choice("disagreement"), None, "a score is not a choice");
}

#[test]
fn a_failed_call_is_refused_rather_than_read_as_no_answer() {
	let Refused = r#"{"success": false, "errors": [{ "code": 1000, "message": "an expired credential" }]}"#;
	let Why = Parse(Refused).expect_err("a refused call is not an answer");
	assert_eq!(Why.Code(), Code::Unparseable);
	assert!(Why.Detail().contains("an expired credential"), "{}", Why.Detail());

	let Empty = r#"{"success": true, "result": { "answers": {} }}"#;
	let Why = Parse(Empty).expect_err("an answer to no question is not an answer");
	assert_eq!(Why.Code(), Code::Unparseable);

	let NotJson = "the endpoint said no";
	let Why = Parse(NotJson).expect_err("a body that is not JSON is refused");
	assert_eq!(Why.Code(), Code::Unparseable);
	assert!(Why.Detail().contains("the endpoint said no"), "{}", Why.Detail());
}

#[test]
fn a_score_reply_without_a_legend_is_filled_from_the_request() {
	let Body = r#"{
		"success": true,
		"result": {
			"answers": { "disagreement": { "type": "score", "score": 2 } },
			"usage": { "input_tokens": 10 }
		}
	}"#;
	let Parsed = Parse(Body).expect("the reply parses");
	let Answered::Score { Legend, .. } = Parsed.Answer("disagreement").expect("answered") else {
		panic!("a score reply is a score");
	};
	assert!(Legend.is_empty(), "the reply named none");

	let Corpus = Load(&Directory()).expect("the fixture corpus loads");
	let State = Moving(60.0);
	let Lanes = Rules::New(Vec::new()).Falling("wide");
	let Selection = Select(&Corpus, &State, &Lanes, &Rules::default()).expect("a lane holds");
	let Request = Compose(&Selection, &State).expect("composed");
	let Filled = Parsed.Filled(&Request);
	let Answered::Score { Legend, .. } = Filled.Answer("disagreement").expect("answered") else {
		panic!("a score reply is a score");
	};
	assert_eq!(
		Legend.clone(),
		vec!["none".to_string(), "slight".to_string(), "clear".to_string()]
	);
	assert_eq!(Filled.Position("disagreement"), Some(1.0), "2 on a three-word scale is the top");
	let Question = Request.Question("disagreement").expect("the request asks it");
	assert_eq!(Question.Position(1.0), Some(0.5), "and the scale is the corpus's own");
}

#[test]
fn the_round_trip_posts_through_the_port_and_the_body_is_the_lanes() {
	let Corpus = Load(&Directory()).expect("the fixture corpus loads");
	let State = Moving(60.0);
	let Lanes = Rules::New(Vec::new()).Falling("narrow");
	let Selection = Select(&Corpus, &State, &Lanes, &Rules::default()).expect("a lane holds");
	let Request = Compose(&Selection, &State).expect("composed");
	let Poster = Scripted::Answering(Captured);

	let Answer = BlockOn(Ask(&Request, &*Poster, Budget())).expect("answered");
	assert_eq!(Answer.Choice("course"), Some(("closer", 0.62)));

	let Posted = Poster.Posted();
	assert_eq!(Posted.len(), 1, "one request, one post");
	assert_eq!(Posted[0].0, Request.Path());
	let Sent:Value = serde_json::from_str(&Posted[0].1).expect("the posted body is JSON");
	assert_eq!(Sent, Request.Body(), "the poster sends the request's own body");
	assert_eq!(Sent["state"], json!({ "depth": 60.0 }), "and only the lane's fields");
}

#[test]
fn a_cancelled_call_is_never_posted() {
	let Corpus = Load(&Directory()).expect("the fixture corpus loads");
	let State = Moving(60.0);
	let Lanes = Rules::New(Vec::new()).Falling("narrow");
	let Selection = Select(&Corpus, &State, &Lanes, &Rules::default()).expect("a lane holds");
	let Request = Compose(&Selection, &State).expect("composed");
	let Poster = Scripted::Answering(Captured);

	let Signal = Signal::New();
	Signal.Fire();
	let Call = Call::Within(Duration::from_secs(1), &Wall, Signal.clone());
	let Why = BlockOn(Ask(&Request, &*Poster, Call)).expect_err("a cancelled call is not posted");
	assert_eq!(Why.Code(), Code::PostFailed);
	assert!(Why.Message().contains("cancelled"), "{}", Why.Message());
	assert!(Poster.Posted().is_empty(), "nothing was sent, so nothing was paid for");
}

#[test]
fn a_run_stops_at_its_ceiling_and_reports_what_it_spent() {
	let Corpus = Load(&Directory()).expect("the fixture corpus loads");
	let State = Moving(60.0);
	let Lanes = Rules::New(Vec::new()).Falling("narrow");
	let Poster = Scripted::Answering(Captured);
	let Decider = Create(
		Corpus,
		Poster.clone() as Arc<dyn Poster>,
		Options {
			Model:"clef-flash".to_string(),
			Lanes,
			Templates:Rules::default(),
			Budget:Budget::New(1, 0),
		},
	);
	assert_eq!(Decider.Model(), "clef-flash");
	assert_eq!(Decider.Corpus().Version(), 2);

	let Selection = Decider.Select(&State).expect("a lane holds");
	let Answer = BlockOn(Decider.Decide(&Selection, &State, Budget())).expect("answered");
	assert_eq!(Answer.Usage.Input, 375);

	let Why = BlockOn(Decider.Decide(&Selection, &State, Budget())).expect_err("the ceiling holds");
	assert_eq!(Why.Code(), Code::BudgetExhausted);
	assert_eq!(Poster.Posted().len(), 1, "the refused call never reached the poster");

	let Spent = Decider.Budget();
	assert_eq!(Spent.Requests, 1);
	assert_eq!(Spent.Tokens(), 375);
	assert!(Spent.Line().contains("ceiling 1 requests"), "{}", Spent.Line());
	assert!((Spent.Cost() - 375.0 / 1_000_000.0 * Price("clef-flash")).abs() < f64::EPSILON);
}

#[test]
fn a_failed_post_is_counted_as_a_request() {
	let Corpus = Load(&Directory()).expect("the fixture corpus loads");
	let State = Moving(60.0);
	let Lanes = Rules::New(Vec::new()).Falling("narrow");
	let Poster = Scripted::Failing("the socket closed", "dial tcp: connection refused");
	let Decider = Create(
		Corpus,
		Poster.clone() as Arc<dyn Poster>,
		Options {
			Model:"clef-flash".to_string(),
			Lanes,
			Templates:Rules::default(),
			Budget:Budget::New(0, 0),
		},
	);

	let Selection = Decider.Select(&State).expect("a lane holds");
	let Why = BlockOn(Decider.Decide(&Selection, &State, Budget())).expect_err("the post failed");
	assert_eq!(Why.Code(), Code::PostFailed);
	assert_eq!(Why.Message(), "the socket closed");
	assert!(Why.Detail().contains("connection refused"), "{}", Why.Detail());

	let Spent = Decider.Budget();
	assert_eq!(Spent.Requests, 1, "a failed call was still a request");
	assert_eq!(Spent.Failed, 1);
	assert_eq!(Spent.Tokens(), 0);
	assert!(
		Spent.Last.as_deref().unwrap_or_default().contains("post-failed"),
		"{:?}",
		Spent.Last
	);
	assert!(Spent.Line().contains("no ceiling"), "{}", Spent.Line());
}

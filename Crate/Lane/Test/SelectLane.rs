//! `agentic.lane.select`: one context in, the one lane whose condition holds
//! out - or the refusal that says why there is not one.
//!
//! No network and no browser: the corpus is the fixture beside this test, and
//! every context below is a JSON object the test wrote. The vocabulary is
//! deliberately generic (a reading, a lane, a template): if a game or a page
//! name appeared here, the crate would be claiming a caller it does not have.
//!
//! Three claims are held here, and each is stated in prose somewhere in the
//! crate first:
//!
//! - **selection is a function of the corpus and the context**, so the same
//!   context selects the same lane twice and adding a lane is a JSON edit;
//! - **exactly one lane holds**, so none and several are refusals rather than
//!   guesses;
//! - **a missing field is not a zero**, so a condition over a reading nobody
//!   took is false and the lane it guards is not selected.

#![allow(non_snake_case, non_upper_case_globals)]

use std::path::{Path, PathBuf};

use AgenticLane::{Context, Corpus, CorpusError, Load, Predicate, Read, SelectError, SelectLane};
use serde_json::{Value, json};

/// The fixture corpus this crate ships beside its test.
fn Directory() -> PathBuf { Path::new(env!("CARGO_MANIFEST_DIR")).join("Test/Fixture/Select") }

/// The observation every inline corpus below is checked against.
const Observed:&str = r#"{
	"state": {
		"speed": { "unit": "km/h" },
		"yaw": { "unit": "deg/s" },
		"surface": { "type": "string" },
		"corner.mode": { "type": "string" }
	},
	"derived": { "blocked_ticks": "how many ticks the reading has not moved" }
}"#;

/// One template, so a lane has something to ask.
fn Template() -> Value {
	json!({
		"focused": {
			"questions": {
				"course": {
					"type": "choice",
					"instructions": "Which way?",
					"criteria": { "closer": "toward", "farther": "away" }
				}
			}
		}
	})
}

/// A corpus whose lanes are the ones the test wrote.
fn Made(Lanes:Value) -> Corpus {
	let Questions = json!({ "version": 3, "templates": Template(), "lanes": Lanes });
	Read(Observed, &Questions.to_string()).expect("the inline corpus loads")
}

/// A context built from the pairs a test wrote.
fn Of(Body:Value) -> Context { Context::Of(Body).expect("a JSON object is a context") }

/// The name a selection chose, for a test that only cares which lane it was.
fn Chosen(Corpus:&Corpus, Context:&Context) -> String {
	SelectLane(Corpus, Context)
		.unwrap_or_else(|Why| panic!("a lane should hold: {Why}"))
		.Name
		.clone()
}

#[test]
fn a_context_selects_the_lane_whose_condition_holds() {
	let Corpus = Load(&Directory()).expect("the fixture corpus loads");
	let Cases = [
		("steady", json!({ "speed": 60, "yaw": 1 })),
		("turning", json!({ "speed": 60, "yaw": 12 })),
		("fast", json!({ "speed": 200, "yaw": 12 })),
		("blocked", json!({ "speed": 0, "blocked_ticks": 5 })),
	];
	for (Expected, Body) in Cases {
		let Context = Of(Body);
		assert_eq!(
			Chosen(&Corpus, &Context),
			Expected,
			"the context {Context:?} selects {Expected}"
		);
		assert_eq!(
			Chosen(&Corpus, &Context),
			Expected,
			"two calls with one context select the same lane"
		);
	}
}

#[test]
fn a_context_no_lane_holds_is_reported_with_what_it_carried() {
	let Corpus = Load(&Directory()).expect("the fixture corpus loads");
	// `speed` alone: `steady` and `turning` both read `yaw`, and `blocked` needs
	// a counter nobody sent - so nothing holds, which is a refusal and not a
	// lane chosen by exhaustion.
	let Context = Of(json!({ "speed": 60 }));
	let Why = SelectLane(&Corpus, &Context).expect_err("nothing holds");
	let SelectError::NoMatchingLane { Message, Context: Summary } = &Why else {
		panic!("expected NoMatchingLane, got {Why}");
	};
	assert!(Summary.contains("speed=60"), "the refusal carries the context: {Summary}");
	assert!(Message.contains("turning"), "the refusal names what was tried: {Message}");
	assert!(Why.to_string().starts_with("no-matching-lane:"), "{Why}");
	assert_eq!(Why.Detail(), *Summary, "the detail is the context, not the sentence");
}

#[test]
fn two_lanes_that_both_hold_are_ambiguous_rather_than_guessed() {
	let Corpus = Made(json!({
		"alpha": {
			"fields": ["speed"],
			"templates": ["focused"],
			"select": { "op": "gt", "field": "speed", "than": 1 }
		},
		"beta": {
			"fields": ["speed"],
			"templates": ["focused"],
			"select": { "op": "gt", "field": "speed", "than": 2 }
		}
	}));
	let Why = SelectLane(&Corpus, &Of(json!({ "speed": 60 }))).expect_err("both hold");
	let SelectError::AmbiguousMatch { Lanes, .. } = &Why else {
		panic!("expected AmbiguousMatch, got {Why}");
	};
	assert_eq!(Lanes, &vec!["alpha".to_string(), "beta".to_string()], "the corpus's own order");
	assert!(Why.to_string().contains("alpha, beta"), "{Why}");
}

#[test]
fn a_catch_all_answers_only_when_no_condition_does() {
	let Corpus = Made(json!({
		"plain": {
			"fields": ["speed"],
			"templates": ["focused"],
			"select": { "op": "gt", "field": "speed", "than": 100 }
		},
		"rest": {
			"fields": ["speed"],
			"templates": ["focused"],
			"select": { "op": "always" }
		}
	}));
	assert_eq!(
		Chosen(&Corpus, &Of(json!({ "speed": 200 }))),
		"plain",
		"a condition outranks the catch-all"
	);
	assert_eq!(
		Chosen(&Corpus, &Of(json!({ "speed": 10 }))),
		"rest",
		"the catch-all answers what nothing else does"
	);
	assert_eq!(
		Chosen(&Corpus, &Of(json!({}))),
		"rest",
		"and it answers an empty context too, which is the point of stating one"
	);
}

#[test]
fn a_corpus_whose_lanes_state_no_condition_is_ambiguous_rather_than_first_wins() {
	// No `select` at all: both lanes mean `always`, which is a corpus that has
	// not said when either applies. Taking the first would hide that.
	let Corpus = Made(json!({
		"one": { "fields": ["speed"], "templates": ["focused"] },
		"two": { "fields": ["speed"], "templates": ["focused"] }
	}));
	let Why = SelectLane(&Corpus, &Of(json!({ "speed": 60 }))).expect_err("both are catch-alls");
	assert!(
		matches!(&Why, SelectError::AmbiguousMatch { Lanes, .. } if Lanes.len() == 2),
		"{Why}"
	);
}

#[test]
fn a_missing_field_is_no_match_rather_than_a_zero() {
	let Empty = Context::New();
	assert!(!Predicate::Below("speed", 1.0).Matches(&Empty), "a missing reading is not zero");
	assert!(!Predicate::Above("speed", 1.0).Matches(&Empty));
	assert!(
		!Predicate::Ne { Field:"speed".to_string(), Value:json!(1) }.Matches(&Empty),
		"a field nobody read is not a field that differs"
	);
	assert!(
		!Predicate::In { Field:"speed".to_string(), Values:vec![json!(1)] }.Matches(&Empty),
		"nor is it one of a list"
	);
	assert!(
		Predicate::Always.Matches(&Empty),
		"`always` is the one condition that reads nothing"
	);

	let Zero = Context::New().With("speed", 0);
	assert!(Predicate::Below("speed", 1.0).Matches(&Zero), "a reading of zero is a reading");
	assert_eq!(Zero.Number("speed"), Some(0.0));
}

#[test]
fn a_negation_is_the_complement_of_its_case_including_for_a_missing_field() {
	// Stated in `Struct/Predicate.rs` and held here: `not` means "the inner case
	// does not hold", so it is true for a field nobody read. A corpus that must
	// not fire on a missing field writes the positive form.
	let Empty = Context::New();
	assert!(Predicate::Negate(Predicate::Equal("speed", 0)).Matches(&Empty));
	let Moving = Context::New().With("speed", 10);
	assert!(Predicate::Negate(Predicate::Equal("speed", 0)).Matches(&Moving));
	assert!(!Predicate::Negate(Predicate::Above("speed", 5.0)).Matches(&Moving));
}

#[test]
fn a_number_compares_as_a_number_whichever_way_it_was_written() {
	let Gear = Context::New().With("gear", 3);
	assert!(Predicate::Equal("gear", 3.0).Matches(&Gear), "3 and 3.0 are one reading");
	assert!(Predicate::Equal("gear", 3).Matches(&Gear));
	assert!(!Predicate::Equal("gear", 3.5).Matches(&Gear));
	assert!(
		!Predicate::Equal("gear", "3").Matches(&Gear),
		"a string is not a number, however it is spelled"
	);
}

#[test]
fn a_lane_added_only_as_json_is_selected_without_code_changes() {
	// The corpus this crate ships, plus one lane this crate has never heard of.
	// Nothing below compiles a new match arm: the lane is data.
	let Root = Directory();
	let Observation = std::fs::read_to_string(Root.join("Templates/Observation.json")).expect("the observation reads");
	let mut Questions:Value = serde_json::from_str(
		&std::fs::read_to_string(Root.join("Templates/Questions.json")).expect("the questions read"),
	)
	.expect("the questions are JSON");
	Questions["lanes"]["corner_mode"] = json!({
		"fields": ["speed"],
		"templates": ["focused"],
		"select": { "op": "eq", "field": "corner.mode", "value": "race" },
		"cadence": "while a corner is named",
		"why": "a lane this crate was never taught"
	});
	let Corpus = Read(&Observation, &Questions.to_string()).expect("the extended corpus loads");

	// The path is nested in the context and flat in the corpus, and the two are
	// the same lookup - which is what lets an adapter group its readings.
	let Context = Of(json!({ "corner": { "mode": "race" }, "speed": 90 }));
	assert_eq!(Chosen(&Corpus, &Context), "corner_mode");
}

#[test]
fn a_condition_over_a_field_the_observation_does_not_carry_is_refused_at_load() {
	let Questions = json!({
		"version": 3,
		"templates": Template(),
		"lanes": {
			"ghost": {
				"fields": ["speed"],
				"templates": ["focused"],
				"select": { "op": "gt", "field": "ghost_field", "than": 1 }
			}
		}
	});
	let Why = Read(Observed, &Questions.to_string()).expect_err("the lane can never hold");
	assert!(
		matches!(&Why, CorpusError::UnknownFieldRef { lane, field } if lane == "ghost" && field == "ghost_field"),
		"{Why}"
	);
	assert!(Why.to_string().contains("ghost_field"), "the refusal names the field: {Why}");
}

#[test]
fn a_condition_that_is_not_a_predicate_is_refused_at_load() {
	let Questions = json!({
		"version": 3,
		"templates": Template(),
		"lanes": {
			"nonsense": {
				"fields": ["speed"],
				"templates": ["focused"],
				"select": { "op": "roughly", "field": "speed", "than": 1 }
			}
		}
	});
	let Why = Read(Observed, &Questions.to_string()).expect_err("no such operator");
	assert!(
		matches!(&Why, CorpusError::BadCondition { lane, .. } if lane == "nonsense"),
		"the refusal names the lane whose condition it could not read: {Why}"
	);
	assert!(Why.to_string().contains("nonsense"), "{Why}");
}

#[test]
fn a_context_reads_a_flat_path_and_a_nested_one_alike() {
	let Context = Of(json!({ "car": { "gear": 3 }, "car.speed": 40 }));
	assert_eq!(Context.Number("car.speed"), Some(40.0), "the exact key wins over the walk");
	assert_eq!(Context.Number("car.gear"), Some(3.0), "a dot walks into an object");
	assert_eq!(Context.Get("car.rpm"), None, "a path that is not there is not a value");
	assert_eq!(Context.Get("car.gear.deeper"), None, "and a walk into a number stops there");
	assert!(Context.Summary().contains("car.speed=40"), "{}", Context.Summary());
	assert!(
		Context.Summary().contains("car={\"gear\":3}"),
		"a grouped reading is summarised as the object it was sent as: {}",
		Context.Summary()
	);

	let Null = Of(json!({ "speed": null }));
	assert_eq!(Null.Get("speed"), None, "null is absent, not zero");
	assert!(!Null.Has("speed"));
	assert_eq!(Null.Summary(), "speed=null", "and it is still reported as what was sent");

	assert!(Context::Of(json!([1, 2])).is_err(), "an array of readings has no field names");
	assert_eq!(Context::New().Summary(), "an empty context");
}

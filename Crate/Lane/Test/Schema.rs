//! The corpus's typed model, exercised as data: a lane written by hand, a
//! condition read out of it, and the context that condition is answered with.
//!
//! Every case here is about *shape* rather than about a run: nothing in this
//! file posts a request, opens a file or selects a lane. What it holds is the
//! claim the loader and the selector both rest on - that the JSON an adapter
//! writes becomes a typed value, that the typed value means what the module docs
//! say it means, and that a value written back out is the value that came in.
//!
//! The vocabulary is deliberately generic. A game or a page named here would be
//! this crate claiming a caller it does not have.

#![allow(non_snake_case, non_upper_case_globals)]

use AgenticLane::{Context, Corpus, Lane, Observation, Predicate};
use serde_json::{Value, json};

/// A lane written the way an adapter writes one: a request shape, a template it
/// serves, and a compound condition over two readings.
const Compound:&str = r#"{
	"fields": ["speed", "steer"],
	"templates": ["corner.entry"],
	"select": {
		"op": "and",
		"of": [
			{ "op": "gt", "field": "speed", "than": 55 },
			{ "op": "lt", "field": "steer", "than": 0 }
		]
	},
	"cadence": "on approach to a turn",
	"why": "the car is fast and the wheel is already turned"
}"#;

/// A context that satisfies it.
fn Cornering() -> Context { Context::New().With("speed", 60.0).With("steer", -0.2) }

#[test]
fn a_lane_with_a_compound_condition_deserializes_into_the_typed_value() {
	let Lane:Lane = serde_json::from_str(Compound).expect("the lane is the shape this crate reads");
	assert_eq!(Lane.Fields, vec!["speed".to_string(), "steer".to_string()]);
	assert_eq!(Lane.Templates, vec!["corner.entry".to_string()]);
	assert_eq!(
		Lane.Select,
		Predicate::And {
			Of:vec![
				Predicate::Gt { Field:"speed".to_string(), Than:55.0 },
				Predicate::Lt { Field:"steer".to_string(), Than:0.0 },
			],
		}
	);
	assert!(Lane.Select.Matches(&Cornering()), "a fast car with the wheel left holds");
	assert!(
		!Lane.Select.Matches(&Context::New().With("speed", 60.0).With("steer", 0.2)),
		"the same speed with the wheel right does not"
	);
	assert!(
		!Lane.Select.Matches(&Context::New().With("speed", 40.0).With("steer", -0.2)),
		"and neither does a slow one"
	);
}

#[test]
fn a_lane_that_states_no_condition_is_a_catch_all() {
	let Lane:Lane = serde_json::from_str(r#"{ "fields": ["speed"], "templates": ["t"] }"#)
		.expect("a lane with no select still loads");
	assert_eq!(Lane.Select, Predicate::Always);
	assert!(
		Lane.Select.Matches(&Context::New()),
		"and holds for every context, including an empty one"
	);
}

#[test]
fn every_operator_answers_the_reading_it_names() {
	let Reading = Context::New()
		.With("verdict", "blocked")
		.With("speed", 60)
		.With("steer", -0.25)
		.With("car", json!({ "gear":3 }));
	let Cases:Vec<(&str, bool)> = vec![
		(r#"{ "op": "always" }"#, true),
		(r#"{ "op": "eq", "field": "verdict", "value": "blocked" }"#, true),
		(r#"{ "op": "eq", "field": "verdict", "value": "moving" }"#, false),
		(r#"{ "op": "ne", "field": "verdict", "value": "moving" }"#, true),
		(r#"{ "op": "in", "field": "verdict", "values": ["blocked", "stalled"] }"#, true),
		(r#"{ "op": "in", "field": "verdict", "values": ["moving", "creeping"] }"#, false),
		(r#"{ "op": "gt", "field": "speed", "than": 55 }"#, true),
		(r#"{ "op": "gt", "field": "speed", "than": 60 }"#, false),
		(r#"{ "op": "lt", "field": "steer", "than": 0 }"#, true),
		(r#"{ "op": "lt", "field": "steer", "than": -0.25 }"#, false),
		(
			r#"{ "op": "or", "of": [ { "op": "eq", "field": "verdict", "value": "moving" },
		                          { "op": "gt", "field": "speed", "than": 50 } ] }"#,
			true,
		),
		(
			r#"{ "op": "and", "of": [ { "op": "eq", "field": "verdict", "value": "moving" },
		                           { "op": "gt", "field": "speed", "than": 50 } ] }"#,
			false,
		),
		(
			r#"{ "op": "not", "of": { "op": "eq", "field": "verdict", "value": "moving" } }"#,
			true,
		),
		(r#"{ "op": "and", "of": [] }"#, true),
		(r#"{ "op": "or", "of": [] }"#, false),
	];
	for (Written, Expected) in Cases {
		let Condition:Predicate = serde_json::from_str(Written).expect("the operator is one this crate reads");
		assert_eq!(Condition.Matches(&Reading), Expected, "{Written}");
	}
}

#[test]
fn a_path_with_a_dot_walks_into_an_object() {
	let Reading = Context::New().With("car", json!({ "gear":3, "speed":12.5 }));
	let Gear:Predicate = serde_json::from_str(r#"{ "op": "eq", "field": "car.gear", "value": 3 }"#).unwrap();
	assert!(Gear.Matches(&Reading), "car.gear reads the object under car");
	let Fast:Predicate = serde_json::from_str(r#"{ "op": "gt", "field": "car.speed", "than": 10 }"#).unwrap();
	assert!(Fast.Matches(&Reading));

	// A field whose own name contains a dot is still reachable by that name: the
	// exact key is tried before the name is read as a path.
	let Literal = Context::New().With("car.gear", 3);
	assert!(Gear.Matches(&Literal));
}

#[test]
fn a_missing_field_is_a_non_match_and_never_a_panic() {
	let Empty = Context::New();
	for Written in [
		r#"{ "op": "eq", "field": "gear", "value": 0 }"#,
		r#"{ "op": "ne", "field": "gear", "value": 0 }"#,
		r#"{ "op": "in", "field": "gear", "values": [0, 1] }"#,
		r#"{ "op": "gt", "field": "gear", "than": 0 }"#,
		r#"{ "op": "lt", "field": "gear", "than": 0 }"#,
		r#"{ "op": "and", "of": [ { "op": "gt", "field": "gear", "than": 0 } ] }"#,
	] {
		let Condition:Predicate = serde_json::from_str(Written).unwrap();
		assert!(!Condition.Matches(&Empty), "{Written} must not hold for a field nobody read");
	}

	// `not` is the complement of its inner case, which is why it *does* hold for
	// a missing field - the documented sharp edge, asserted so it stays documented.
	let Negated:Predicate =
		serde_json::from_str(r#"{ "op": "not", "of": { "op": "eq", "field": "gear", "value": 0 } }"#).unwrap();
	assert!(Negated.Matches(&Empty));

	// A null is a missing reading, not a value: the ingestion could not take it.
	let Nulled = Context::New().With("gear", Value::Null);
	let IsZero:Predicate = serde_json::from_str(r#"{ "op": "eq", "field": "gear", "value": 0 }"#).unwrap();
	assert!(!IsZero.Matches(&Nulled));
}

#[test]
fn numbers_compare_by_value_and_never_across_types() {
	let Integer = Context::New().With("gear", 0);
	let Decimal = Context::New().With("gear", 0.0);
	let IsZero:Predicate = serde_json::from_str(r#"{ "op": "eq", "field": "gear", "value": 0 }"#).unwrap();
	assert!(IsZero.Matches(&Integer));
	assert!(IsZero.Matches(&Decimal), "1 and 1.0 are one reading written two ways");

	let Text = Context::New().With("gear", "0");
	assert!(!IsZero.Matches(&Text), "a string is never a number");
}

#[test]
fn a_condition_round_trips_through_the_json_it_was_written_as() {
	let Lane:Lane = serde_json::from_str(Compound).expect("the lane reads");
	let Written = serde_json::to_value(&Lane.Select).expect("the condition writes");
	assert_eq!(
		Written,
		json!({
			"op": "and",
			"of": [
				{ "op": "gt", "field": "speed", "than": 55.0 },
				{ "op": "lt", "field": "steer", "than": 0.0 }
			]
		}),
		"the write is the read, so a corpus can be printed back as it was written"
	);
	let Again:Predicate = serde_json::from_value(Written).expect("and reads again");
	assert_eq!(Again, Lane.Select);
}

#[test]
fn an_unknown_operator_is_refused_rather_than_guessed() {
	let Why = serde_json::from_str::<Predicate>(r#"{ "op": "approximately", "field": "speed" }"#)
		.expect_err("an operator this crate has not been taught is not a condition");
	assert!(
		Why.to_string().contains("approximately"),
		"the refusal names what it did not understand: {Why}"
	);

	let TooMany = serde_json::from_str::<Predicate>(r#"{ "op": "gt", "field": "speed", "than": 1, "op": "lt" }"#);
	assert!(TooMany.is_err(), "two operators in one object is not a condition");
}

#[test]
fn a_condition_reads_the_fields_it_names() {
	let Lane:Lane = serde_json::from_str(Compound).expect("the lane reads");
	assert_eq!(
		Lane.Select.Fields(),
		vec!["speed", "steer"],
		"both paths, in the order they are read"
	);
	assert_eq!(Predicate::Always.Fields(), Vec::<&str>::new());
	assert_eq!(
		Lane.Select.Line(),
		"all of (speed > 55 and steer < 0)".to_string(),
		"the trace line is the condition in words"
	);
}

#[test]
fn the_observation_is_read_as_typed_entries() {
	let Written = json!({
		"$comment": "what the ingestion can hand over",
		"version": 1,
		"state": {
			"speed_kmh": {
				"type": "number", "source": "memory", "address": "car+0x430",
				"unit": "km/h", "range": [0, 400], "note": "signed forward speed"
			},
			"grounded": { "type": "array", "items": "boolean", "length": 4 },
			"verdict": { "type": "string" }
		},
		"derived": { "moved_m": "metres the car advanced over the last six ticks" }
	});
	let Observation = Observation::Of(Written.clone()).expect("the observation is the shape this crate reads");
	assert_eq!(Observation.Version, 1);
	assert_eq!(Observation.Note.as_deref(), Some("what the ingestion can hand over"));
	let Speed = Observation.Field("speed_kmh").expect("speed_kmh is declared");
	assert_eq!(Speed.Kind(), Some("number"));
	assert_eq!(Speed.Unit.as_deref(), Some("km/h"));
	assert_eq!(Speed.Range, Some(vec![0.0, 400.0]));
	assert_eq!(Speed.Address.as_deref(), Some("car+0x430"));
	let Grounded = Observation.Field("grounded").expect("grounded is declared");
	assert_eq!(Grounded.Items.as_deref(), Some("boolean"));
	assert_eq!(Grounded.Length, Some(4));
	assert_eq!(Observation.Derived().count(), 1);
	assert!(Observation.Declares("verdict") && !Observation.Declares("slip"));

	let Expected = json!({
		"$comment": "what the ingestion can hand over",
		"version": 1,
		"state": {
			"speed_kmh": {
				"type": "number", "source": "memory", "address": "car+0x430",
				"unit": "km/h", "range": [0.0, 400.0], "note": "signed forward speed"
			},
			"grounded": { "type": "array", "items": "boolean", "length": 4 },
			"verdict": { "type": "string" }
		},
		"derived": { "moved_m": "metres the car advanced over the last six ticks" }
	});
	assert_eq!(
		serde_json::to_value(&Observation).expect("the observation writes"),
		Expected,
		"every key survives the round trip; a numeric range writes its bounds as numbers"
	);
}

#[test]
fn a_corpus_round_trips_as_one_document() {
	let Written = json!({
		"version": 2,
		"fields": { "speed": { "unit": "km/h" }, "steer": { "range": [-1, 1] } },
		"derived": { "verdict": "one of moving, blocked" },
		"templates": {
			"corner.entry": {
				"when": "approaching a turn",
				"questions": {
					"action": { "type": "choice", "instructions": "What next?",
								"criteria": { "brake": "brake", "throttle": "go" } }
				}
			}
		},
		"lanes": {
			"corner": {
				"fields": ["speed", "steer"],
				"templates": ["corner.entry"],
				"select": { "op": "gt", "field": "speed", "than": 55 }
			},
			"cruise": { "fields": ["speed"], "templates": ["corner.entry"] }
		},
		"order": ["cruise", "corner"]
	});
	let Corpus:Corpus = serde_json::from_value(Written.clone()).expect("the corpus is the shape this crate reads");
	assert_eq!(Corpus.Version(), 2);
	assert_eq!(Corpus.Root(), None, "a corpus read from a value came from no directory");
	assert_eq!(Corpus.Order, vec!["cruise".to_string(), "corner".to_string()]);
	assert_eq!(
		Corpus.Ordered().iter().map(|(Id, _)| Id.as_str()).collect::<Vec<&str>>(),
		vec!["cruise", "corner"],
		"the declared order is kept, not the sorted one"
	);
	assert_eq!(
		Corpus.Lanes.keys().map(String::as_str).collect::<Vec<&str>>(),
		vec!["corner", "cruise"],
		"the map stays sorted, which is the other question"
	);
	assert!(
		Corpus.Observation().Declares("speed"),
		"the typed view is built from the descriptors"
	);
	assert_eq!(Corpus.Observation().Fields().count(), 2);
	assert!(Corpus.Observed("verdict"), "a derived field is as sendable as a read one");
	assert!(!Corpus.Observed("slip"), "and a field nobody declared is not");

	assert_eq!(
		serde_json::to_value(&Corpus).expect("the corpus writes"),
		Written,
		"a corpus prints back as the document it was read from"
	);
}

#[test]
fn an_order_that_names_a_lane_nobody_wrote_hides_nothing() {
	let Corpus:Corpus = serde_json::from_value(json!({
		"lanes": {
			"drive": { "fields": [], "templates": ["t"] },
			"recover": { "fields": [], "templates": ["t"] }
		},
		"order": ["recover", "retired"]
	}))
	.expect("an order naming a lane that is gone is not a refusal");
	assert_eq!(
		Corpus.Ordered().iter().map(|(Id, _)| Id.as_str()).collect::<Vec<&str>>(),
		vec!["recover", "drive"],
		"a lane the order does not mention is appended, never hidden"
	);
}

//! `Fn::Decode`, exercised against bodies and nothing else.
//!
//! Every case here is a body a transport hands back - one well-formed reply, and
//! one per way a reply can be wrong. There is no corpus, no poster and no
//! request: the decoder's contract is "bytes in, typed answer or a named
//! reason out", and a test that needed the rest of the crate to state it would
//! be testing the crate.
//!
//! The malformed cases assert the *variant*, not "it failed". A decoder that
//! answers `Err` for everything is the shape this vocabulary exists to replace,
//! so a test that only checked `is_err()` would pass against the code it is
//! meant to catch.

#![allow(non_snake_case, non_upper_case_globals)]

use AgenticLane::{Answered, Code, Decode, DecodeBytes, DecodeError, DecodeWithin, Parse};

/// A reply the endpoint actually answered, kept as the bytes a poster returns.
///
/// Captured rather than composed, because the decoder's job is to read what the
/// endpoint writes - a body this crate built for itself would agree with its own
/// assumptions and prove nothing. `output_tokens` is zero because every answer
/// measured on this account reported zero: `clef-flash` prices output at nothing.
const WellFormed:&str = r#"{
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

#[test]
fn a_well_formed_reply_decodes_to_a_choice_a_distribution_and_a_budget() {
	let Answer = DecodeWithin(WellFormed, 1000).expect("the captured reply decodes");
	assert_eq!(Answer.Model, "clef-flash");

	// The chosen option, and the whole distribution it was chosen from - a
	// caller that only kept the winner could not tell 0.62 against a rival from
	// certainty.
	let Answered::Choice { Choice, Probabilities } = Answer.Answer("course").expect("answered") else {
		panic!("a choice reply is a choice");
	};
	assert_eq!(Choice, "closer");
	assert_eq!(Probabilities.len(), 2, "both options are kept, not just the winner");
	assert_eq!(Probabilities["closer"], 0.62);
	assert_eq!(Probabilities["farther"], 0.38);

	// The typed answer is inspectable without reading the decoder: these are the
	// accessors a caller uses.
	assert_eq!(Answer.Choice("course"), Some(("closer", 0.62)));
	assert_eq!(Answer.Probability("course", "farther"), Some(0.38));
	assert_eq!(Answer.Score("disagreement"), Some(2.0));

	// What the reply spent, and what the ceiling has left after it.
	assert_eq!(Answer.Usage.Input, 375);
	assert_eq!(Answer.Usage.Output, 0);
	assert_eq!(
		Answer.Usage.Consumed, 375,
		"consumed is input plus output, as the meter charges it"
	);
	assert_eq!(Answer.Usage.Remaining, Some(625), "1000 less the 375 this reply consumed");
	assert!(!Answer.Usage.Exhausted(), "625 tokens are left");
}

#[test]
fn a_reply_read_without_a_ceiling_has_no_remaining() {
	let Answer = Decode(WellFormed).expect("the captured reply decodes");
	assert_eq!(Answer.Usage.Consumed, 375);
	assert_eq!(
		Answer.Usage.Remaining, None,
		"a remaining without a ceiling would be a number this crate invented"
	);
	assert!(!Answer.Usage.Exhausted(), "an unbounded run cannot have reached a ceiling");
}

#[test]
fn a_reply_that_spent_past_the_ceiling_reports_nothing_left_rather_than_wrapping() {
	let Answer = DecodeWithin(WellFormed, 100).expect("the reply still decodes: the ceiling is a report, not a policy");
	assert_eq!(Answer.Usage.Consumed, 375);
	assert_eq!(
		Answer.Usage.Remaining,
		Some(0),
		"375 spent of 100 leaves nothing, not a negative"
	);
	assert!(Answer.Usage.Exhausted());
}

#[test]
fn a_body_that_is_not_json_is_malformed() {
	let Why = Decode("the endpoint said no").expect_err("a body that is not JSON is refused");
	let DecodeError::Malformed { Detail } = Why else { panic!("expected Malformed, got {Why:?}") };
	assert!(Detail.contains("not JSON"), "{Detail}");
	assert!(
		Detail.contains("the endpoint said no"),
		"the body's opening is in the detail: {Detail}"
	);
}

#[test]
fn a_body_that_is_not_utf8_is_malformed() {
	let Why = DecodeBytes(&[0xFF, 0xFE, 0x00]).expect_err("bytes that are not UTF-8 are refused");
	let DecodeError::Malformed { Detail } = Why else { panic!("expected Malformed, got {Why:?}") };
	assert!(Detail.contains("not UTF-8"), "{Detail}");
}

#[test]
fn a_reply_without_an_answers_object_names_the_missing_field() {
	let Body = r#"{"success": true, "result": { "usage": { "input_tokens": 1 } }}"#;
	let Why = Decode(Body).expect_err("a reply that answers nothing cannot become an answer");
	let DecodeError::Missing { Field, Detail } = Why else { panic!("expected Missing, got {Why:?}") };
	assert_eq!(Field, "result.answers");
	assert!(Detail.contains("no answers"), "{Detail}");
}

#[test]
fn a_reply_that_reports_no_spend_names_the_missing_budget() {
	let Body =
		r#"{"success": true, "result": { "answers": { "course": { "choice": "a", "probabilities": { "a": 1 } } } }}"#;
	let Why = Decode(Body).expect_err("a typed answer with no budget is not typed enough");
	let DecodeError::Missing { Field, .. } = Why else { panic!("expected Missing, got {Why:?}") };
	assert_eq!(Field, "result.usage");
}

#[test]
fn a_result_that_is_not_an_object_is_a_shape_error() {
	let Body = r#"{"success": true, "result": "the model said no"}"#;
	let Why = Decode(Body).expect_err("a string is not a result");
	let DecodeError::Shape { Field, Detail } = Why else { panic!("expected Shape, got {Why:?}") };
	assert_eq!(Field, "result");
	assert!(Detail.contains("found a string"), "{Detail}");
}

#[test]
fn a_choice_outside_its_own_distribution_is_refused() {
	let Body = r#"{
		"success": true,
		"result": {
			"answers": { "course": { "choice": "nearer", "probabilities": { "closer": 0.7, "farther": 0.3 } } },
			"usage": { "input_tokens": 12, "output_tokens": 0 }
		}
	}"#;
	let Why = Decode(Body).expect_err("a choice the reply did not carry is not a choice");
	let DecodeError::UnknownChoice { Question, Choice, Options } = Why else {
		panic!("expected UnknownChoice, got {Why:?}")
	};
	assert_eq!(Question, "course");
	assert_eq!(Choice.as_deref(), Some("nearer"));
	assert_eq!(Options, vec!["closer".to_string(), "farther".to_string()]);
}

#[test]
fn a_choice_that_names_no_option_is_refused() {
	let Body = r#"{
		"success": true,
		"result": {
			"answers": { "course": { "type": "choice", "probabilities": { "closer": 0.7, "farther": 0.3 } } },
			"usage": { "input_tokens": 12, "output_tokens": 0 }
		}
	}"#;
	let Why = Decode(Body).expect_err("a distribution with no chosen option is not an answer");
	let DecodeError::UnknownChoice { Choice, Options, .. } = Why else { panic!("expected UnknownChoice, got {Why:?}") };
	assert_eq!(Choice, None, "the reply named no option");
	assert_eq!(Options.len(), 2, "the options it did carry are still reported");
}

#[test]
fn a_probability_that_is_not_a_share_is_refused() {
	let Body = r#"{
		"success": true,
		"result": {
			"answers": { "course": { "choice": "closer", "probabilities": { "closer": 1.5, "farther": 0.3 } } },
			"usage": { "input_tokens": 12, "output_tokens": 0 }
		}
	}"#;
	let Why = Decode(Body).expect_err("1.5 is not a probability");
	let DecodeError::ProbabilityShape { Question, Detail } = Why else {
		panic!("expected ProbabilityShape, got {Why:?}")
	};
	assert_eq!(Question, "course");
	assert!(Detail.contains("outside 0..=1"), "{Detail}");
	assert!(Detail.contains("closer"), "the offending option is named: {Detail}");
}

#[test]
fn probabilities_that_are_not_a_map_are_refused() {
	let Body = r#"{
		"success": true,
		"result": {
			"answers": { "course": { "choice": "closer", "probabilities": [0.6, 0.4] } },
			"usage": { "input_tokens": 12, "output_tokens": 0 }
		}
	}"#;
	let Why = Decode(Body).expect_err("an array is not a distribution by option");
	let DecodeError::ProbabilityShape { Detail, .. } = Why else { panic!("expected ProbabilityShape, got {Why:?}") };
	assert!(Detail.contains("not an object of options"), "{Detail}");
}

#[test]
fn a_spend_that_is_not_a_token_count_is_refused() {
	let Body = r#"{
		"success": true,
		"result": {
			"answers": { "course": { "choice": "closer", "probabilities": { "closer": 1 } } },
			"usage": { "input_tokens": "many", "output_tokens": 0 }
		}
	}"#;
	let Why = Decode(Body).expect_err("\"many\" is not a token count");
	let DecodeError::BudgetShape { Detail } = Why else { panic!("expected BudgetShape, got {Why:?}") };
	assert!(Detail.contains("input_tokens"), "{Detail}");
	assert!(Detail.contains("not an unsigned token count"), "{Detail}");
}

#[test]
fn a_call_the_endpoint_refused_is_not_read_as_a_missing_answer() {
	let Body = r#"{"success": false, "errors": [{ "code": 1000, "message": "an expired credential" }]}"#;
	let Why = Decode(Body).expect_err("a refused call is not an answer");
	let DecodeError::Refused { Detail } = Why else { panic!("expected Refused, got {Why:?}") };
	assert!(
		Detail.contains("an expired credential"),
		"the endpoint's own words survive: {Detail}"
	);
}

#[test]
fn a_reply_that_answered_no_question_is_empty() {
	let Body = r#"{"success": true, "result": { "answers": {}, "usage": { "input_tokens": 9 } }}"#;
	let Why = Decode(Body).expect_err("an answer to no question is not an answer");
	let DecodeError::Empty { Detail } = Why else { panic!("expected Empty, got {Why:?}") };
	assert!(Detail.contains("empty object"), "{Detail}");
}

#[test]
fn a_question_type_this_crate_does_not_interpret_is_carried_not_refused() {
	// `noul` is a type the endpoint knows and this crate does not. Refusing it
	// would be this crate claiming a vocabulary it lacks; dropping it would be
	// an answer that is not one.
	let Body = r#"{
		"success": true,
		"result": {
			"answers": {
				"confidence": { "type": "noul", "choice": "sure", "probabilities": { "sure": 0.9, "unsure": 0.1 } }
			},
			"usage": { "input_tokens": 5, "output_tokens": 0 }
		}
	}"#;
	let Answer = Decode(Body).expect("a type this crate cannot read is still an answer");
	let Answered::Other(Raw) = Answer.Answer("confidence").expect("answered") else {
		panic!("expected Other, got {:?}", Answer.Answer("confidence"))
	};
	assert_eq!(Raw["type"], "noul", "the answer arrives as it was written");
	assert_eq!(
		Answer.Choice("confidence"),
		None,
		"this crate does not threshold a type it does not know"
	);
}

#[test]
fn the_ports_error_keeps_the_decoders_reason() {
	// The port speaks `module.json`'s seven codes, so an unreadable reply is
	// `unparseable` - but the reason it was unreadable is not flattened into
	// that code's sentence.
	let Why = Parse("the endpoint said no").expect_err("a body that is not JSON is refused");
	assert_eq!(Why.Code(), Code::Unparseable);
	assert!(Why.Message().contains("not JSON"), "{}", Why.Message());
	match Why.Cause() {
		Some(DecodeError::Malformed { Detail }) => assert!(Detail.contains("the endpoint said no"), "{Detail}"),
		Other => panic!("expected a Malformed cause, got {Other:?}"),
	}
}

#[test]
fn every_reason_names_itself_for_a_trace() {
	// A trace line is `name: sentence`, and the name is stable across releases:
	// a log reader greps for it, so it may not drift with a message.
	assert!(DecodeError::Missing("result.answers", "").Why().starts_with("missing:"));
	assert!(DecodeError::BudgetShape("").Why().starts_with("budget-shape:"));
	assert!(
		DecodeError::UnknownChoice("course", None, Vec::new())
			.Why()
			.starts_with("unknown-choice:")
	);
	assert_eq!(DecodeError::Empty("").Name(), "empty");
}

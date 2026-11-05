//! The request layer's own claims: three keys, no chat field, the lane's model.
//!
//! A target of its own rather than more cases in `Test/Lane.rs`, because these
//! are claims about the body alone - no poster, no decoder, no file on disk. A
//! failure here is a failure of the request type and should not be read as a
//! failure of the round trip.
//!
//! The corpora are inline for the same reason: this file must not care where the
//! fixture lives, and it must keep working while somebody moves it.

#![allow(non_snake_case, non_upper_case_globals)]

use std::collections::BTreeSet;

use AgenticLane::{Compose, Read, Request, Route, Selection, State};
use serde_json::json;

/// The observation every corpus below reads.
const Observation:&str = r#"{ "state": { "depth": { "unit": "m" }, "heading": { "unit": "deg" } } }"#;

/// Three lanes: one that names a model with the route's prefix, one that names a
/// bare one, and one that says nothing and so asks whatever the caller's default
/// is. The last is the case a corpus author leaves out, and it has to keep
/// working.
const Questions:&str = r#"{
	"version": 1,
	"templates": {
		"focused": {
			"questions": {
				"course": {
					"type": "choice",
					"instructions": "Which way?",
					"criteria": { "closer": "toward", "farther": "away" }
				},
				"disagreement": {
					"type": "score",
					"instructions": "How far?",
					"criteria": ["none", "slight", "clear"]
				}
			}
		}
	},
	"lanes": {
		"prefixed": {
			"model": "@cf/cloudflare/clef-flash",
			"fields": ["depth"],
			"templates": ["focused"]
		},
		"sharp": { "model": "clef", "fields": ["depth", "heading"], "templates": ["focused"] },
		"silent": { "fields": ["depth"], "templates": ["focused"] }
	}
}"#;

/// One lane and one template, named rather than selected: these cases are about
/// what a selection composes into, not about how the selection was reached.
fn Selected(Lane:&str) -> Selection {
	let Corpus = Read(Observation, Questions).expect("the inline corpus loads");
	Selection {
		Lane:Corpus.Lane(Lane).expect("the lane is declared").clone(),
		Template:Corpus.Template("focused").expect("the template is declared").clone(),
	}
}

/// A reading with more fields than any lane sends.
fn Observed() -> State {
	let mut State = State::New();
	State.Set("depth", 42);
	State.Set("heading", 90);
	State
}

#[test]
fn a_composed_request_is_exactly_three_keys_and_no_chat_field() {
	let Request = Compose(&Selected("prefixed"), &Observed()).expect("composed");
	let Body = Request.Body();
	let Object = Body.as_object().expect("a request is a JSON object");

	let Keys:BTreeSet<&str> = Object.keys().map(String::as_str).collect();
	assert_eq!(
		Keys,
		BTreeSet::from(["model", "state", "questions"]),
		"the body's top-level keys are the request's three fields and nothing else"
	);
	assert_eq!(Object.len(), 3, "and there is no fourth key under any other name");

	for Chat in [
		"messages",
		"roles",
		"role",
		"temperature",
		"prompt",
		"content",
		"stream",
		"max_tokens",
	] {
		assert!(
			Object.get(Chat).is_none(),
			"{Chat} is a chat-completions field, and this is a decision request"
		);
	}

	assert_eq!(
		serde_json::to_value(&Request).expect("a request serialises"),
		Body,
		"the body is serde's own output, not a second description of the same three keys"
	);
}

#[test]
fn a_chat_completions_payload_is_not_a_request() {
	let Chat = json!({
		"model": "clef-flash",
		"messages": [{ "role": "user", "content": "which way?" }],
		"temperature": 0.2,
		"state": {},
		"questions": {}
	});
	let Why = serde_json::from_value::<Request>(Chat)
		.expect_err("a chat payload is not a decision request, and the type says so");
	assert!(Why.to_string().contains("unknown field"), "{Why}");
}

#[test]
fn the_lane_names_the_model_and_the_route_carries_the_prefix() {
	let Selection = Selected("prefixed");
	assert_eq!(
		Selection.Lane.Model, "@cf/cloudflare/clef-flash",
		"the corpus's own spelling is kept on the lane"
	);
	let Request = Compose(&Selection, &Observed()).expect("composed");
	assert_eq!(Request.Model, "clef-flash", "the body gets the bare name");
	assert_eq!(Request.Body()["model"], json!("clef-flash"));
	assert_eq!(Request.Path(), format!("{Route}clef-flash"));
	assert_eq!(Request.Path(), "/ai/run/@cf/cloudflare/clef-flash");

	let Sharp = Compose(&Selected("sharp"), &Observed()).expect("composed");
	assert_eq!(Sharp.Model, "clef", "a lane that names another model asks that one");
	assert_eq!(Sharp.Path(), "/ai/run/@cf/cloudflare/clef");
}

#[test]
fn a_lane_that_names_no_model_asks_the_crates_default() {
	let Selection = Selected("silent");
	assert_eq!(Selection.Lane.Model, "", "the corpus named none");
	let Request = Compose(&Selection, &Observed()).expect("composed");
	assert_eq!(Request.Model, "clef-flash");
	assert_eq!(Request.Path(), "/ai/run/@cf/cloudflare/clef-flash");
}

#[test]
fn the_body_reads_back_as_the_request_it_came_from() {
	let Request = Compose(&Selected("sharp"), &Observed()).expect("composed");
	let Back:Request = serde_json::from_str(&Request.Text()).expect("the body reads back");
	assert_eq!(Back, Request, "the type is the body in both directions");
	assert_eq!(
		Back.Names(),
		vec!["course", "disagreement"],
		"and the questions keep their names"
	);
}

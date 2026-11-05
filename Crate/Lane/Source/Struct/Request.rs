//! What is posted: `{model, state, questions}`, and the route it goes to.
//!
//! A request is a value rather than a string so that the body, the path and the
//! questions can each be asserted separately - a test that compared the whole
//! serialised body against a literal would pass while the field order changed
//! and fail while nothing was wrong.
//!
//! # The body is a derive, and the derive is the whole definition
//!
//! Three named fields, each with an explicit `rename`, and no `#[serde(flatten)]`
//! beside them. That is what makes "exactly three top-level keys" a property of
//! the type rather than of a literal somebody remembered to update: a fourth
//! field is a fourth key by construction, and a body that carries `messages` or
//! `temperature` is a body this type refuses to read (`deny_unknown_fields`). A
//! decision model asked with a chat-completions payload answers 400 with no
//! diagnosis in it, so the type is where that mistake has to be impossible.
//!
//! `questions` is a map because the endpoint's own shape is a map keyed by
//! question name. That is not a catch-all beside the three fields - it is the
//! third field's value, and the names in it are the corpus's.
//!
//! The route is here and not in the poster, because it is the one part of the
//! endpoint that is not a credential or a transport: a decision model asked
//! through a chat-completions route is a 400 with no diagnosis in it, and a
//! caller that had to remember the difference would eventually not.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::{
	Fn::Model::{Bare, Route},
	Struct::{Question::Question, State::State},
};

/// One composed request.
///
/// The fields are the body's keys and there are no others: `model`, `state`,
/// `questions`. A fourth field would be a fourth key, which is why nothing else
/// lives here - not a lane name, not a template name, not a trace id. Those are
/// the caller's business and the endpoint would reject them.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Request {
	/// The model to ask, as the body names it (the bare name, without the
	/// route's `@cf/cloudflare/` prefix).
	#[serde(rename = "model")]
	pub Model:String,
	/// The fields this lane sends.
	#[serde(rename = "state")]
	pub State:State,
	/// The questions this template asks, by the name the corpus gave each.
	#[serde(rename = "questions")]
	pub Questions:BTreeMap<String, Question>,
}

impl Request {
	/// The body the endpoint receives.
	///
	/// The three keys come from the derive above, so this cannot disagree with
	/// [`Request`]'s own field list. Serialising a struct of a string, a JSON
	/// object and a map of JSON objects has no failing case - every number in a
	/// [`Value`] came from JSON and every map key is a string - so the fallback
	/// is unreachable. It is `null` rather than a half-built body because half a
	/// request is one the endpoint would answer wrongly, and "nothing was
	/// composed" is the honest thing to send when nothing could be.
	pub fn Body(&self) -> Value { serde_json::to_value(self).unwrap_or(Value::Null) }

	/// The body as the bytes a poster sends.
	pub fn Text(&self) -> String { self.Body().to_string() }

	/// The path the request goes to, which is never a chat-completions route.
	pub fn Path(&self) -> String { format!("{Route}{}", Bare(&self.Model)) }

	/// One question, by name.
	pub fn Question(&self, Name:&str) -> Option<&Question> { self.Questions.get(Name) }

	/// The names of the questions, in the order the request carries them.
	pub fn Names(&self) -> Vec<&str> { self.Questions.keys().map(String::as_str).collect() }
}

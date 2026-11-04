//! What is posted: `{model, state, questions}`, and the route it goes to.
//!
//! A request is a value rather than a string so that the body, the path and the
//! questions can each be asserted separately - a test that compared the whole
//! serialised body against a literal would pass while the field order changed
//! and fail while nothing was wrong.
//!
//! The route is here and not in the poster, because it is the one part of the
//! endpoint that is not a credential or a transport: a decision model asked
//! through a chat-completions route is a 400 with no diagnosis in it, and a
//! caller that had to remember the difference would eventually not.

use std::collections::BTreeMap;

use serde_json::{Value, json};

use crate::{
	Fn::Model::{Bare, Route},
	Struct::{Question::Question, State::State},
};

/// One composed request.
#[derive(Clone, Debug, PartialEq)]
pub struct Request {
	/// The model to ask, as the body names it (the bare name, without the
	/// route's `@cf/cloudflare/` prefix).
	pub Model:String,
	/// The fields this lane sends.
	pub State:State,
	/// The questions this template asks.
	pub Questions:BTreeMap<String, Question>,
}

impl Request {
	/// The body the endpoint receives.
	pub fn Body(&self) -> Value {
		let Questions:BTreeMap<String, Value> = self
			.Questions
			.iter()
			.map(|(Name, Question)| (Name.clone(), Question.Body()))
			.collect();
		json!({
			"model": self.Model,
			"state": self.State.Body(),
			"questions": Questions,
		})
	}

	/// The body as the bytes a poster sends.
	pub fn Text(&self) -> String { self.Body().to_string() }

	/// The path the request goes to, which is never a chat-completions route.
	pub fn Path(&self) -> String { format!("{Route}{}", Bare(&self.Model)) }

	/// One question, by name.
	pub fn Question(&self, Name:&str) -> Option<&Question> { self.Questions.get(Name) }

	/// The names of the questions, in the order the request carries them.
	pub fn Names(&self) -> Vec<&str> { self.Questions.keys().map(String::as_str).collect() }
}

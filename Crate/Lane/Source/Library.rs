//! `agentic.lane.decide`: a template corpus as data, and the round trip that
//! turns one state into one typed answer.
//!
//! A decision model answers a *shape*, not a sentence: a `state` it may read
//! and a set of typed `questions` it must answer with a probability per allowed
//! option. The shape is the same for every game, every page and every device -
//! what differs is the vocabulary, and that vocabulary is a corpus on disk.
//!
//! # A directory's worth of knowledge, in one place
//!
//! A corpus is a directory: a `Templates/` holding an *observation* naming every
//! field a caller may send and a *question* file naming the templates, with the
//! lanes either inline in that file or in a JSON file of their own beside it.
//! Everything this crate does is a function of those files:
//!
//! - [`Fn::Load::Load`] reads the directory and validates it against itself: a
//!   lane naming a field the observation neither reads nor derives, or a
//!   template that does not exist, is a corpus that cannot answer and is refused
//!   at load rather than mid-run. Its failure is [`Struct::CorpusError::CorpusError`],
//!   which names the path or the id that was wrong, and which narrows to the
//!   port's own [`Struct::Error::Error`] with `?`;
//! - [`Fn::Select::Select`] picks the lane and, inside it, the template - by a
//!   predicate over the state, never by guessing which question is current;
//! - [`Fn::Compose::Compose`] builds `{model, state, questions}` from the
//!   selection, sending only the fields the lane names and asking the model the
//!   lane names. Those three keys are [`Struct::Request::Request`]'s three
//!   fields and nothing else: the body is a derive, so a chat-completions
//!   payload - `messages`, `temperature` - is not a shape this crate can write
//!   or read;
//! - [`Fn::Ask::Ask`] posts it through a [`Struct::Port::Poster`] and
//!   [`Fn::Parse::Parse`] returns the answer typed: a choice with a probability
//!   for every option, a score with its legend, or the reply's own value.
//!
//! # Two levels of selection, because a lane is a request *shape*
//!
//! A lane names the fields a question actually reads, and the templates it
//! serves - `drive` sends three numbers where `context` sends the full state,
//! which is the difference between a quarter of the tokens and all of them.
//! Within a lane, its template list is an ordered preference: the first one
//! whose predicate holds wins, and the lane's own order is the fallback. That
//! is why [`Struct::Lane::Selection`] carries both halves: a request composed
//! from a lane and the wrong template would be a shape with nothing in it.
//!
//! The predicates are Rust values ([`Struct::When::When`]) rather than an
//! expression language in the corpus, because the honest predicate often needs
//! what the corpus cannot see: "not moving for three ticks" is a counter the
//! caller keeps, and a JSON comparison can only say "not moving now".
//!
//! # No network in the crate
//!
//! [`Struct::Port::Poster`] is this crate's own trait - a request in, a reply
//! body out, bounded by the caller's [`AgenticRuntime::Call`] - and the crate
//! never implements it. The credentials, the transport and the status codes are
//! an adapter's business, which is what lets a test answer from a captured
//! fixture and lets a caller swap curl for anything else without touching the
//! machinery.
//!
//! The route is not the adapter's business, though: this crate states it once
//! ([`Fn::Model::Route`]), because a decision model read through a
//! chat-completions endpoint is a 400 with no diagnosis in it. The body is
//! `{model, state, questions}` and the path is `/ai/run/@cf/cloudflare/<name>`.
//!
//! # What is metered
//!
//! [`Struct::Budget::Budget`] is the run's meter: requests, tokens and the
//! price they carry, checked before a call and reported after it. It lives
//! beside [`Struct::Decider::Decider`] rather than inside the round trip,
//! because a guard held across an await would make the port's future `!Send` -
//! so [`Fn::Ask::Ask`] is a pure round trip and the meter is written in two
//! short locks around it.

#![allow(non_snake_case)]
#![allow(non_upper_case_globals)]
#![allow(clippy::upper_case_acronyms)]

pub mod Fn;
pub mod Struct;

pub use crate::{
	Fn::{
		Ask::Ask,
		Compose::Compose,
		Load::{Load, Read},
		Model::{Bare, Models, Price, Route},
		Parse::Parse,
		Select::{Choose, Select},
	},
	Struct::{
		Answer::{Answer, Answered, Usage},
		Budget::Budget,
		Context::Context,
		Corpus::Corpus,
		CorpusError::CorpusError,
		Decider::{Create, Decider, Options},
		Error::{Code, Error},
		Lane::{Lane, Selection},
		Observation::{Observation, ObservationField},
		Port::{Decide, Failure, Poster},
		Predicate::Predicate,
		Question::Question,
		Request::Request,
		Rule::{Rule, Rules},
		State::State,
		Template::Template,
		When::When,
	},
};

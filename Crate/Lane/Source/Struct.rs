//! The nouns: the corpus, the two levels of selection, the request, the answer
//! and the meter.

pub mod Answer;
pub mod Budget;
pub mod Corpus;
pub mod Decider;
pub mod Error;
pub mod Lane;
pub mod Port;
pub mod Question;
pub mod Request;
pub mod Rule;
pub mod State;
pub mod Template;
pub mod When;

pub use crate::Struct::{
	Answer::Answer,
	Budget::Budget,
	Corpus::Corpus,
	Error::Error,
	Lane::{Lane, Selection},
	Port::{Decide, Poster},
	Question::Question,
	Request::Request,
	Rule::Rules,
	State::State,
	Template::Template,
	When::When,
};

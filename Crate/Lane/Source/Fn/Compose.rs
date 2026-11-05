//! Turning a selection and a state into the request that is posted.
//!
//! Three decisions, and each is a refusal of something easier:
//!
//! - **Only the lane's fields travel.** The state a caller observes is usually
//!   larger than the question reads - a full observation against three numbers - and
//!   sending the full state every tick is what a lane exists to avoid. The
//!   fields the lane names are kept even when the state does not carry them, as
//!   `null`: "this was not observed" is a fact the model may weigh, while a
//!   field quietly omitted leaves it guessing whether it was asked at all.
//! - **The template's questions travel whole.** A template is the unit of
//!   asking; composing half of one would be a request nobody wrote.
//! - **The model is named bare.** The route carries `@cf/cloudflare/`, the body
//!   carries the name, and a caller that passed the full id to the body would
//!   be naming a model the endpoint does not have.

use crate::{
	Fn::Model::Bare,
	Struct::{Error::Error, Lane::Selection, Question::Question, Request::Request, State::State},
};

/// The request one selection asks for.
pub fn Compose(Selection:&Selection, State:&State, Model:&str) -> Result<Request, Error> {
	if Selection.Template.Questions.is_empty() {
		return Err(Error::CorpusInvalid(
			format!("template {} asks nothing", Selection.Template.Name),
			"a request with no question is a request with no answer".to_string(),
		));
	}
	let Questions:std::collections::BTreeMap<String, Question> = Selection.Template.Questions.clone();
	Ok(Request { Model:Bare(Model), State:State.Keep(&Selection.Lane.Fields), Questions })
}

//! Turning a selection and a state into the request that is posted.
//!
//! Four decisions, and each is a refusal of something easier:
//!
//! - **Only the lane's fields travel.** The state a caller observes is usually
//!   larger than the question reads - a full observation against three numbers - and
//!   sending the full state every tick is what a lane exists to avoid. The
//!   fields the lane names are kept even when the state does not carry them, as
//!   `null`: "this was not observed" is a fact the model may weigh, while a
//!   field quietly omitted leaves it guessing whether it was asked at all.
//! - **The template's questions travel whole.** A template is the unit of
//!   asking; composing half of one would be a request nobody wrote.
//! - **The model is the lane's.** A lane is a request shape *and* a price, so
//!   the model is chosen with the shape rather than passed beside it: a caller
//!   that selected the three-number lane and then asked the expensive model has
//!   paid for the fields it saved. The caller's own default reaches the lane at
//!   [`crate::Struct::Decider::Create`], which stamps it onto every lane whose
//!   corpus named none.
//! - **The model is named bare.** The route carries `@cf/cloudflare/`, the body
//!   carries the name, and a caller that passed the full id to the body would
//!   be naming a model the endpoint does not have.

use crate::{
	Fn::Model::Bare,
	Struct::{Error::Error, Lane::Selection, Question::Question, Request::Request, State::State},
};

/// The request one selection asks for.
///
/// The two arguments are everything the request is a function of: the selection
/// says which fields and which questions, the state says what they read. The
/// model is not a third argument because it is not the caller's per call - it is
/// part of the shape the lane was selected for.
pub fn Compose(Selection:&Selection, State:&State) -> Result<Request, Error> {
	if Selection.Template.Questions.is_empty() {
		return Err(Error::CorpusInvalid(
			format!("template {} asks nothing", Selection.Template.Name),
			"a request with no question is a request with no answer".to_string(),
		));
	}
	let Questions:std::collections::BTreeMap<String, Question> = Selection.Template.Questions.clone();
	Ok(Request {
		Model:Bare(&Selection.Lane.Model),
		State:State.Keep(&Selection.Lane.Fields),
		Questions,
	})
}

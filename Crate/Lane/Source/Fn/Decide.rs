//! The whole pipeline in one call: a corpus and a state in, a typed answer out.
//!
//! Four steps, each stated once somewhere else and joined here rather than
//! repeated: [`SelectLane`] picks the lane the corpus's own conditions name,
//! [`Choose`] picks the template inside that lane, [`Compose`] builds
//! `{model, state, questions}`, and [`Ask`] posts it through the caller's
//! [`Poster`] and decodes the reply into [`Answer`]. An entry point that
//! re-stated any of the four would be a second authority over a shape the rest
//! of the crate already argues for.
//!
//! # Nothing is kept between calls
//!
//! No budget, no cache, no remembered lane. A caller that wants a meter or a
//! policy keeps it itself - [`crate::Struct::Decider::Decider`] is that caller -
//! so this function answers one question about one state and forgets it. That is
//! what lets a test drive the whole pipeline from a poster that answers a
//! string, against a corpus built in memory: no file, no socket, no clock.
//!
//! # A state is not a context
//!
//! [`Decide`] selects from the state, which is right for a corpus whose lanes
//! read fields the request carries. [`DecideOn`] selects from a context that may
//! be wider - a counter the caller keeps, a sentence the ingestion derived -
//! while the request still sends only the state. This crate keeps those two
//! apart on purpose ([`crate::Struct::Context`]): collapsing them would either
//! decide a lane by a reading the caller cannot see, or pay tokens for a field
//! no question reads. The wider form exists so neither has to be chosen by
//! accident.
//!
//! # Which model, when the corpus names none
//!
//! The model is the lane's rather than an argument, because a lane is a price
//! as well as a shape and an entry point that took a model would let a caller
//! overrule the corpus on the way past it. A lane that names none asks the
//! crate's default - [`crate::Fn::Model::Bare`] of an empty name - which is the
//! same silence [`Compose`] resolves. A caller that wants a *different* default
//! stamps the corpus first: [`crate::Struct::Decider::Create`] is the
//! constructor that does exactly that, and
//! [`crate::Struct::Decider::Decider::Corpus`] hands back the stamped corpus
//! this function takes.
//!
//! # One error, three origins
//!
//! A selection that names no lane, or names two, lowers to
//! [`variant@Error::NoMatch`] / [`variant@Error::CorpusInvalid`] through
//! [`crate::Struct::SelectError`]'s own `From`; a poster that does not answer is
//! [`variant@Error::PostFailed`]; a reply this crate cannot read is
//! [`variant@Error::Unparseable`] carrying the decoder's own reason in its
//! `Cause`, so a caller matches on [`crate::Struct::Decode::DecodeError`]
//! instead of parsing a sentence back out of a string. The order is fixed -
//! selection, then composition, then the post, then the read - so a failure
//! before the post leaves the poster with nothing recorded and nobody billed.
//!
//! # No I/O, and no clock of its own
//!
//! Nothing here opens a file or a socket: the corpus arrives loaded, the
//! transport arrives as a [`Poster`], and the caller's [`Call`] is the only
//! bound on the one await. [`Ask`] refuses a call the caller has already
//! cancelled, so an abandoned operation does not become a request somebody pays
//! for.

use AgenticRuntime::Call;

use crate::{
	Fn::{Ask::Ask, Compose::Compose, Select::Choose, SelectLane::SelectLane},
	Struct::{
		Answer::Answer,
		Context::Context,
		Corpus::Corpus,
		Error::Error,
		Lane::Selection,
		Port::Poster,
		Rule::Rules,
		State::State,
	},
};

/// One decision, selected by the state it sends.
///
/// The pipeline in four lines: the lane the corpus's conditions name, the
/// template that lane serves, the request those two compose, and the answer the
/// poster's reply decodes to.
///
/// The state is the context here, so a lane that selects on something the
/// request never carries - a counter, a derived sentence - cannot hold; use
/// [`DecideOn`] for that. Selection is a function of the corpus and the state
/// alone: no clock, no counter, no cache, so a recorded state replays to the
/// same lane and the same request.
pub async fn Decide<P:Poster + ?Sized>(Poster:&P, Corpus:&Corpus, State:&State, Call:Call) -> Result<Answer, Error> {
	DecideOn(Poster, Corpus, &Context::from(State), State, Call).await
}

/// One decision, selected by a context wider than the state it sends.
///
/// The context is what a lane's condition reads; the state is what the request
/// carries. They are two values because they are two questions - "does this lane
/// apply" and "what does the model get to see" - and a caller that keeps a
/// counter or a derived verdict can answer the first without paying for it in
/// the second.
///
/// The template inside the lane is chosen by the lane's own order
/// ([`Rules::default`] holds no rules), because a corpus lists its templates in
/// preference. A caller that wants to override that per call has
/// [`crate::Fn::Select::Select`], which takes the caller's rules.
pub async fn DecideOn<P:Poster + ?Sized>(
	Poster:&P,
	Corpus:&Corpus,
	Context:&Context,
	State:&State,
	Call:Call,
) -> Result<Answer, Error> {
	let Lane = SelectLane(Corpus, Context)?;
	let Template = Choose(Corpus, Lane, State, &Rules::default())?;
	let Selected = Selection { Lane:Lane.clone(), Template:Template.clone() };
	let Request = Compose(&Selected, State)?;
	Ask(&Request, Poster, Call).await
}

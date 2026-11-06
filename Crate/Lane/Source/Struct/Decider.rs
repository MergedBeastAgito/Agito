//! The port's value: a corpus, a poster, the rules and the meter.
//!
//! [`Create`] is the whole construction, and the three things it takes are the
//! three things a caller has to decide: which corpus (data), which poster (a
//! transport), and which rules (the caller's own knowledge of what its states
//! mean - or none, when the corpus already states each lane's condition).
//! Nothing else is configurable, because nothing else differs between two
//! callers.
//!
//! # The meter, and why it is written around the call
//!
//! [`crate::Struct::Budget::Budget`] sits behind a `Mutex` because the port's
//! methods take `&self` - a value held as `Arc<dyn Decide>` cannot offer
//! `&mut self`, and a caller that had to own the meter would be a second
//! authority over the same spend. The lock is taken, read, and dropped before
//! the request is awaited; it is taken again afterwards to record what the
//! reply reported. Holding it across the await would make the returned future
//! `!Send`, which is the one thing [`RunTime::FutureBox`] forbids.

use std::sync::{Arc, Mutex, MutexGuard};

use ::RunTime::{Call, FutureBox};

use crate::{
	Fn::{
		Ask::Ask,
		Compose::Compose,
		Model::{Bare, Models},
		Resolve::Resolve,
	},
	Struct::{
		Answer::Answer,
		Budget::Budget,
		Corpus::Corpus,
		Error::Error,
		Lane::Selection,
		Port::{Decide, Poster},
		Rule::Rules,
		State::State,
	},
};

/// What a caller configures once.
#[derive(Clone, Debug)]
pub struct Options {
	/// The model a lane that names none asks, as the caller names it. The body
	/// gets the bare name and the route gets the same one; see
	/// [`crate::Fn::Model::Bare`].
	///
	/// This is a default rather than an override: a corpus that names a model
	/// per lane has already decided, and a run that flattened those decisions
	/// into one would be paying the expensive model for the cheap lane.
	pub Model:String,
	/// The lane rules, in priority order.
	///
	/// Empty means "select by the corpus's own conditions": every lane states its
	/// `select`, so a caller with nothing to add passes `Rules::default()` rather
	/// than restating the corpus in Rust. A list that names anything wins outright.
	pub Lanes:Rules,
	/// The template rules, in priority order. A rule naming a template the
	/// selected lane does not serve is skipped.
	pub Templates:Rules,
	/// The run's ceiling, priced for the model.
	pub Budget:Budget,
}

impl Default for Options {
	fn default() -> Self {
		Self {
			Model:Models[0].0.to_string(),
			Lanes:Rules::default(),
			Templates:Rules::default(),
			Budget:Budget::New(0, 0),
		}
	}
}

/// One corpus, one poster, one policy.
pub struct Decider {
	/// The corpus the lanes and templates come from.
	Corpus:Corpus,
	/// The transport, as the port that hides it.
	Poster:Arc<dyn Poster>,
	/// The model a lane that names none asks.
	Model:String,
	/// The lane rules.
	Lanes:Rules,
	/// The template rules.
	Templates:Rules,
	/// The meter, behind a lock because the port takes `&self`.
	Budget:Mutex<Budget>,
}

/// Build the port's value.
///
/// The corpus is taken by value because it is written to once here: every lane
/// whose corpus named no model is stamped with [`Options::Model`], which is what
/// makes the caller's default reach [`Compose`] without a third argument. A lane
/// that did name a model keeps it - the corpus is data, and the caller's default
/// is only what silence falls back to.
pub fn Create(mut Corpus:Corpus, Poster:Arc<dyn Poster>, Options:Options) -> Decider {
	let Default = Bare(&Options.Model);
	for Lane in Corpus.Lanes.values_mut() {
		if Lane.Model.trim().is_empty() {
			Lane.Model = Default.clone();
		}
	}
	Decider {
		Corpus,
		Poster,
		Model:Options.Model,
		Lanes:Options.Lanes,
		Templates:Options.Templates,
		Budget:Mutex::new(Options.Budget),
	}
}

impl Decider {
	/// The corpus this value was built from, so a caller can print what it
	/// loaded rather than what it meant to load.
	pub fn Corpus(&self) -> &Corpus { &self.Corpus }

	/// The model a lane that names none asks.
	pub fn Model(&self) -> &str { &self.Model }
}

impl Decide for Decider {
	fn Select(&self, State:&State) -> Result<Selection, Error> {
		Resolve(&self.Corpus, State, &self.Lanes, &self.Templates)
	}

	fn Decide<'a>(
		&'a self,
		Selection:&'a Selection,
		State:&'a State,
		Call:Call,
	) -> FutureBox<'a, Result<Answer, Error>> {
		Box::pin(async move {
			// The ceiling is read and the lock dropped before anything is
			// awaited: see the module doc.
			if !Lock(&self.Budget).Allows() {
				let Line = self.Budget().Line();
				return Err(Error::BudgetExhausted("this run has asked enough", Line));
			}
			let Request = Compose(Selection, State)?;
			match Ask(&Request, &*self.Poster, Call).await {
				Ok(Answer) => {
					Lock(&self.Budget).Charge(Answer.Usage.Input, Answer.Usage.Output);
					Ok(Answer)
				},
				Err(Why) => {
					// A call that ended without an answer was still a request,
					// and the meter says so: a run whose failures are invisible
					// in its total is a run that looks cheaper than it was.
					Lock(&self.Budget).Miss(Why.Why());
					Err(Why)
				},
			}
		})
	}

	fn Budget(&self) -> Budget { Lock(&self.Budget).clone() }
}

/// A lock whose poisoning is not a failure.
///
/// A panicked call must not make the meter permanently unusable: the counts
/// behind the lock are still there, and a run that could not report its spend
/// because an earlier tick panicked would be a run that lost the invoice.
fn Lock<T>(Guard:&Mutex<T>) -> MutexGuard<'_, T> { Guard.lock().unwrap_or_else(|Poison| Poison.into_inner()) }

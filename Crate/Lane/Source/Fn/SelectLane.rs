//! Choosing the one lane a context belongs to, from the corpus's own conditions.
//!
//! [`crate::Fn::Select::Select`] takes the rules from the caller, which is what a
//! caller wants when the honest condition needs memory the corpus cannot see -
//! "not moving for three ticks" is a counter, not an observation. This verb is
//! the other half of that: a lane may declare its own condition in
//! `Questions.json`, and then selection is a function of the corpus and the
//! context alone, with no Rust written per lane.
//!
//! # Adding a lane is a JSON edit
//!
//! A corpus that wants a lane for a surface the caller never named - ice, a
//! tunnel, a menu - gains one by writing it, and nothing in this crate changes:
//!
//! ```json
//! "lanes": {
//! 	"ice": {
//! 		"fields": ["speed", "surface"],
//! 		"templates": ["surface.loss"],
//! 		"when": { "all": [
//! 			{ "is": { "field": "surface", "value": "ice" } },
//! 			{ "more": { "field": "speed", "than": 40 } }
//! 		] }
//! 	}
//! }
//! ```
//!
//! The next call to [`SelectLane`] sees it: no match arm, no registration, no
//! rebuild of this crate. The condition's vocabulary is the closed set in
//! [`crate::Fn::Condition::Condition`], so the cost of that freedom is bounded -
//! a corpus can only say things this crate already knows how to answer.
//!
//! # One lane, or a refusal
//!
//! Exactly one lane must hold. None is [`SelectError::NoMatchingLane`], naming
//! the context that matched nothing; more than one is
//! [`SelectError::AmbiguousMatch`], naming every lane that held. Picking the
//! first of several would be this crate guessing at a priority the corpus never
//! stated, and the guess would be invisible in the answer.
//!
//! Two tiers make a corpus able to say "anything else" without becoming
//! ambiguous:
//!
//! 1. **Conditional lanes** - every lane whose condition is not `always` - are
//!    evaluated first, and exactly one of them must hold;
//! 2. **A catch-all** - a lane whose condition is `always` - answers only when
//!    no conditional lane held, and at most one catch-all may exist.
//!
//! The tier is what keeps `always` meaning *fallback* rather than *competitor*.
//! A lane with no condition at all is in neither tier: it is selectable by name
//! (see [`crate::Fn::Select::Select`]) and by nothing else, so a corpus that
//! forgot its predicates is told so instead of being handed whichever lane the
//! map happened to sort first.
//!
//! # Deterministic, and only over the data
//!
//! The lanes are visited in the corpus's own order (`BTreeMap`, sorted by name),
//! so two calls with one context return the same lane and the same ambiguity
//! list. Nothing here reads the clock, the filesystem or a global: a recorded
//! context replays to the same selection.

use crate::Struct::{Context::Context, Corpus::Corpus, Lane::Lane, SelectError::SelectError, When::When};

/// The one lane this context belongs to, or why there is not one.
pub fn SelectLane<'a>(Corpus:&'a Corpus, Context:&Context) -> Result<&'a Lane, SelectError> {
	let mut Holding:Vec<&'a Lane> = Vec::new();
	let mut Catch:Vec<&'a Lane> = Vec::new();
	for Lane in Corpus.Lanes() {
		match Lane.When.as_ref() {
			// A lane that declares no condition is not a candidate: the corpus
			// has not said when it applies, and "always" is a statement the
			// corpus can make explicitly.
			None => {},
			Some(When::Always) => Catch.push(Lane),
			Some(Condition) if Condition.Matches(Context) => Holding.push(Lane),
			Some(_) => {},
		}
	}

	match Holding.len() {
		1 => return Ok(Holding[0]),
		0 => {},
		_ => {
			let Lanes:Vec<String> = Holding.iter().map(|Lane| Lane.Name.clone()).collect();
			return Err(SelectError::AmbiguousMatch(
				format!("{} lanes hold for this context and the corpus names no priority", Lanes.len()),
				Lanes,
			));
		},
	}

	match Catch.len() {
		1 => Ok(Catch[0]),
		0 => {
			Err(SelectError::NoMatchingLane(
				format!(
					"no lane's condition holds, and no lane is declared `always` (the corpus carries {})",
					Conditional(Corpus)
				),
				Context.Summary(),
			))
		},
		_ => {
			let Lanes:Vec<String> = Catch.iter().map(|Lane| Lane.Name.clone()).collect();
			Err(SelectError::AmbiguousMatch(
				format!("{} lanes are declared `always`, so a catch-all is ambiguous", Lanes.len()),
				Lanes,
			))
		},
	}
}

/// The lanes that declare a condition, as one line, for a refusal that has to
/// say what was tried.
fn Conditional(Corpus:&Corpus) -> String {
	let Named:Vec<String> = Corpus
		.Lanes()
		.filter_map(|Lane| {
			Lane.When
				.as_ref()
				.map(|Condition| format!("{} when {}", Lane.Name, Condition.Line()))
		})
		.collect();
	if Named.is_empty() {
		return "no lane with a condition".to_string();
	}
	Named.join("; ")
}

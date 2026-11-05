//! Choosing the one lane a context belongs to, from the corpus's own conditions.
//!
//! [`crate::Fn::Select::Select`] takes its rules from the caller, which is what a
//! caller wants when the honest condition needs something a corpus cannot hold -
//! "not moving for three ticks" is a counter, not an observation. This verb is
//! the other half: a lane states its own `select` in the corpus, so selection is
//! a function of the corpus and the context alone, with no Rust written per lane.
//!
//! # Adding a lane is a JSON edit
//!
//! A corpus that wants a lane for a surface the caller never named - ice, a
//! tunnel, a menu - gains one by writing it, and nothing in this crate changes:
//!
//! ```json
//! "lanes": {
//!     "ice": {
//!         "fields": ["speed", "surface"],
//!         "templates": ["surface.loss"],
//!         "select": { "op": "and", "of": [
//!             { "op": "eq", "field": "surface", "value": "ice" },
//!             { "op": "gt", "field": "speed", "than": 40 }
//!         ] }
//!     }
//! }
//! ```
//!
//! The next call to [`SelectLane`] sees it: no match arm, no registration, no
//! rebuild of this crate. The condition's vocabulary is the closed set in
//! [`crate::Struct::Predicate::Predicate`], so the cost of that freedom is
//! bounded - a corpus can only say things this crate already knows how to answer.
//!
//! # One lane, or a refusal
//!
//! Exactly one lane must hold. None is [`SelectError::NoMatchingLane`], naming
//! the context that matched nothing; more than one is
//! [`SelectError::AmbiguousMatch`], naming every lane that held. Taking the first
//! of several would be this crate guessing at a priority the corpus never stated,
//! and the guess would be invisible in the answer.
//!
//! Two tiers make a corpus able to say "anything else" without becoming
//! ambiguous:
//!
//! 1. **Conditional lanes** - every lane whose `select` is not `always` - are
//!    evaluated first, and exactly one of them must hold;
//! 2. **A catch-all** - a lane whose `select` is `always`, which is also what a
//!    lane that states no `select` means - answers only when no conditional lane
//!    held, and at most one catch-all may exist.
//!
//! The tier is what keeps `always` meaning *fallback* rather than *competitor*.
//! A corpus in which every lane is a catch-all has no conditions at all, and it
//! is reported as ambiguous rather than resolved by taking whichever lane the map
//! happened to sort first.
//!
//! # Deterministic, and only over the data
//!
//! The lanes are visited in the corpus's own order (`BTreeMap`, sorted by name),
//! so two calls with one context return the same lane and the same ambiguity
//! list. Nothing here reads the clock, the filesystem or a global: a recorded
//! context replays to the same selection.

use crate::Struct::{Context::Context, Corpus::Corpus, Lane::Lane, Predicate::Predicate, SelectError::SelectError};

/// The one lane this context belongs to, or why there is not one.
pub fn SelectLane<'a>(Corpus:&'a Corpus, Context:&Context) -> Result<&'a Lane, SelectError> {
	let mut Holding:Vec<&'a Lane> = Vec::new();
	let mut Catch:Vec<&'a Lane> = Vec::new();
	for Lane in Corpus.Lanes() {
		if matches!(&Lane.Select, Predicate::Always) {
			Catch.push(Lane);
		} else if Lane.Select.Matches(Context) {
			Holding.push(Lane);
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
					"no lane's condition holds, and no lane is a catch-all (the corpus carries {})",
					Conditional(Corpus)
				),
				Describe(Context),
			))
		},
		_ => {
			let Lanes:Vec<String> = Catch.iter().map(|Lane| Lane.Name.clone()).collect();
			Err(SelectError::AmbiguousMatch(
				format!("{} lanes are catch-alls, so \"anything else\" names no lane", Lanes.len()),
				Lanes,
			))
		},
	}
}

/// The lanes that state a condition, as one line, for a refusal that has to say
/// what was tried.
fn Conditional(Corpus:&Corpus) -> String {
	let Named:Vec<String> = Corpus
		.Lanes()
		.filter(|Lane| !matches!(&Lane.Select, Predicate::Always))
		.map(|Lane| format!("{} when {}", Lane.Name, Lane.Select.Line()))
		.collect();
	if Named.is_empty() {
		return "no lane with a condition".to_string();
	}
	Named.join("; ")
}

/// The context as one line, for the same reason: `speed=60, yaw=0.5`.
///
/// A path that holds `null` is printed as `null` rather than omitted, because
/// "the caller sent nothing here" is exactly what a reader of a no-match needs to
/// see - and it is the same reading the conditions saw.
fn Describe(Context:&Context) -> String {
	if Context.IsEmpty() {
		return "an empty context".to_string();
	}
	Context
		.Fields()
		.map(|Field| {
			match Context.Get(Field) {
				Some(Value) => format!("{Field}={Value}"),
				None => format!("{Field}=null"),
			}
		})
		.collect::<Vec<String>>()
		.join(", ")
}

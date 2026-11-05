//! Choosing the lane and the template, by a predicate over the state.
//!
//! Two levels, and both are needed: the lane decides which fields travel, the
//! template decides which questions are asked. Selecting only the lane would
//! leave a caller to pick a template by name, which is the guess the corpus's
//! `when` sentences exist to avoid.
//!
//! # What is a rule, and what is a fallback
//!
//! A rule is a name plus a [`crate::Struct::When::When`]. First match wins, in
//! the order the caller wrote. The fallback is a *name*, not an always-true
//! rule, because a trace should be able to say whether a lane was chosen by its
//! condition or by exhaustion - and a caller that names no fallback gets
//! `no-match` rather than a lane nobody asked for.
//!
//! At the template level the fallback is the lane's own first template: the
//! corpus lists them in preference order, so "no template rule held" means "the
//! lane's ordinary question", and a rule naming a template this lane does not
//! serve is skipped rather than refused - one rule list for every lane is the
//! ordinary case.
//!
//! # No lane rules means the corpus's own conditions
//!
//! A caller with nothing to add passes [`Rules::default`], and selection then
//! reads each lane's `select` ([`crate::Fn::SelectLane::SelectLane`]) instead of
//! answering `no-match`. The two are one function because they answer one
//! question - "which lane is this reading" - and a caller that had to know which
//! of them its empty list would reach would be reading this crate's internals to
//! call it. A rule list that names anything at all still wins outright: a caller
//! that states a policy is not asking for the corpus's.
//!
//! The template rules are honoured in both cases. A lane's conditions and its
//! templates are different questions, and a corpus that states one does not
//! thereby state the other.

use crate::{
	Fn::SelectLane::SelectLane,
	Struct::{
		Context::Context,
		Corpus::Corpus,
		Error::Error,
		Lane::{Lane, Selection},
		Rule::Rules,
		State::State,
		Template::Template,
	},
};

/// The lane and the template this state selects.
pub fn Select(Corpus:&Corpus, State:&State, Lanes:&Rules, Templates:&Rules) -> Result<Selection, Error> {
	// Nothing to add: the corpus states each lane's condition, so this is the
	// whole policy. `SelectLane` refuses an ambiguous or empty corpus rather
	// than taking the first lane the map sorted, which is the same refusal a
	// rule list with no match gives.
	if Lanes.IsEmpty() {
		let Lane = SelectLane(Corpus, &Context::from(State))?;
		let Template = Choose(Corpus, Lane, State, Templates)?;
		return Ok(Selection { Lane:Lane.clone(), Template:Template.clone() });
	}
	for Rule in &Lanes.Ordered {
		if Rule.When.Holds(State) {
			let Lane = Corpus.Lane(&Rule.Name).ok_or_else(|| Unknown(&Rule.Name, Corpus))?;
			let Template = Choose(Corpus, Lane, State, Templates)?;
			return Ok(Selection { Lane:Lane.clone(), Template:Template.clone() });
		}
	}
	let Some(Name) = &Lanes.Default else {
		return Err(Error::NoMatch(
			"no lane rule holds and the rules name no fallback",
			format!("{} over {}", DescribeRules(Lanes), DescribeState(State)),
		));
	};
	let Lane = Corpus.Lane(Name).ok_or_else(|| Unknown(Name, Corpus))?;
	let Template = Choose(Corpus, Lane, State, Templates)?;
	Ok(Selection { Lane:Lane.clone(), Template:Template.clone() })
}

/// The template one lane serves for this state.
pub fn Choose<'a>(Corpus:&'a Corpus, Lane:&Lane, State:&State, Rules:&Rules) -> Result<&'a Template, Error> {
	for Rule in &Rules.Ordered {
		// A rule about another lane's template is not a rule about this one.
		if !Lane.Templates.contains(&Rule.Name) || !Rule.When.Holds(State) {
			continue;
		}
		return Corpus.Template(&Rule.Name).ok_or_else(|| {
			Error::CorpusInvalid(
				format!("lane {} names a template the corpus does not carry", Lane.Name),
				format!("{} is not one of the templates", Rule.Name),
			)
		});
	}
	// The lane's own order is the preference: the first template it names is
	// the one it asks when nothing more specific applies.
	let Some(Name) = Lane.Templates.first() else {
		return Err(Error::NoMatch(
			format!("lane {} serves no template", Lane.Name),
			"a lane with no template has no question to ask".to_string(),
		));
	};
	Corpus.Template(Name).ok_or_else(|| {
		Error::CorpusInvalid(
			format!("lane {} names a template the corpus does not carry", Lane.Name),
			format!("{Name} is not one of the templates"),
		)
	})
}

/// A rule naming a lane the corpus does not have.
fn Unknown(Name:&str, Corpus:&Corpus) -> Error {
	Error::UnknownLane(
		format!("a rule names the lane {Name}, which the corpus does not carry"),
		format!(
			"the corpus carries: {}",
			Corpus.Lanes().map(|Lane| Lane.Name.as_str()).collect::<Vec<&str>>().join(", ")
		),
	)
}

/// A rule list as words, for a refusal that has to say what was tried.
fn DescribeRules(Rules:&Rules) -> String {
	let Mutters:Vec<String> = Rules
		.Ordered
		.iter()
		.map(|Rule| format!("{} when {}", Rule.Name, Rule.When.Line()))
		.collect();
	match &Rules.Default {
		Some(Default) => format!("{}, else {Default}", Mutters.join("; ")),
		None => Mutters.join("; "),
	}
}

/// A state as words, for the same reason: `depth=38, heading=90`.
fn DescribeState(State:&State) -> String {
	if State.IsEmpty() {
		return "an empty state".to_string();
	}
	State
		.Fields()
		.map(|Field| {
			match State.Get(Field) {
				Some(Value) => format!("{Field}={Value}"),
				None => format!("{Field}=null"),
			}
		})
		.collect::<Vec<String>>()
		.join(", ")
}

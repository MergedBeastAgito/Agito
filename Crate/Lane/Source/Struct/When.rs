//! A condition over the state, as a value a caller can hand over and a test can
//! argue with.
//!
//! This is the predicate a lane and a template are selected by. It is a small
//! closed set rather than an expression language, and that is a decision: a
//! string evaluated at run time is a parse failure mid-run, a second syntax
//! nobody's editor checks, and a way for a corpus file to run arbitrary logic.
//! A closed set can only be wrong in ways the type system shows.
//!
//! It is also why the corpus does not carry its predicates. The honest
//! condition often reads something a single observation cannot hold - "not
//! moving for three ticks" is a counter the caller keeps - so the mechanism is
//! here and the conditions belong to the caller that can actually evaluate
//! them.
//!
//! # Missing is not a value
//!
//! Every comparison answers `false` when the field is absent or null: an
//! unobserved field is not a field of zero, and a predicate that treated it as
//! one would select a lane for an observation nobody made. `Absent` is the
//! predicate for "we could not read it", and it is the only one that is true in
//! that case.

use serde_json::Value;

use crate::Struct::State::State;

/// One condition over the state.
#[derive(Clone, Debug, PartialEq)]
pub enum When {
	/// Always holds. A rule with no condition, which is a lane with no
	/// alternative.
	Always,
	/// The field is a number greater than `Than`.
	More {
		/// The field the corpus names.
		Field:String,
		/// The bound, exclusive.
		Than:f64,
	},
	/// The field is a number less than `Than`.
	Less {
		/// The field the corpus names.
		Field:String,
		/// The bound, exclusive.
		Than:f64,
	},
	/// The field is a number at least `Than`.
	AtLeast {
		/// The field the corpus names.
		Field:String,
		/// The bound, inclusive.
		Than:f64,
	},
	/// The field is a number at most `Than`.
	AtMost {
		/// The field the corpus names.
		Field:String,
		/// The bound, inclusive.
		Than:f64,
	},
	/// The field equals a JSON value.
	Is {
		/// The field the corpus names.
		Field:String,
		/// The value it must hold.
		Value:Value,
	},
	/// The field is present and not null.
	Has {
		/// The field the corpus names.
		Field:String,
	},
	/// The field is absent or null.
	Lacks {
		/// The field the corpus names.
		Field:String,
	},
	/// At least one case holds.
	AnyOf(Vec<When>),
	/// Every case holds. An empty list is true, which is what makes `AllOf`
	/// with no cases the same statement as `Always`.
	AllOf(Vec<When>),
	/// No case holds.
	NoneOf(Vec<When>),
}

impl When {
	/// The field is a number greater than `Than`.
	pub fn Above(Field:&str, Than:f64) -> Self { When::More { Field:Field.to_string(), Than } }

	/// The field is a number less than `Than`.
	pub fn Below(Field:&str, Than:f64) -> Self { When::Less { Field:Field.to_string(), Than } }

	/// The field is a number at least `Than`.
	pub fn Least(Field:&str, Than:f64) -> Self { When::AtLeast { Field:Field.to_string(), Than } }

	/// The field is a number at most `Than`.
	pub fn Most(Field:&str, Than:f64) -> Self { When::AtMost { Field:Field.to_string(), Than } }

	/// The field equals a JSON value.
	pub fn Equal(Field:&str, Value:impl Into<Value>) -> Self {
		When::Is { Field:Field.to_string(), Value:Value.into() }
	}

	/// The field is present and not null.
	pub fn Present(Field:&str) -> Self { When::Has { Field:Field.to_string() } }

	/// The field is absent or null.
	pub fn Absent(Field:&str) -> Self { When::Lacks { Field:Field.to_string() } }

	/// At least one case holds.
	pub fn Any(Cases:Vec<When>) -> Self { When::AnyOf(Cases) }

	/// Every case holds.
	pub fn All(Cases:Vec<When>) -> Self { When::AllOf(Cases) }

	/// No case holds.
	pub fn Not(Cases:Vec<When>) -> Self { When::NoneOf(Cases) }

	/// Whether the state satisfies this condition.
	pub fn Holds(&self, State:&State) -> bool {
		match self {
			When::Always => true,
			When::More { Field, Than } => State.Number(Field).is_some_and(|Value| Value > *Than),
			When::Less { Field, Than } => State.Number(Field).is_some_and(|Value| Value < *Than),
			When::AtLeast { Field, Than } => State.Number(Field).is_some_and(|Value| Value >= *Than),
			When::AtMost { Field, Than } => State.Number(Field).is_some_and(|Value| Value <= *Than),
			When::Is { Field, Value } => State.Get(Field) == Some(Value),
			When::Has { Field } => State.Has(Field),
			When::Lacks { Field } => !State.Has(Field),
			When::AnyOf(Cases) => Cases.iter().any(|Case| Case.Holds(State)),
			When::AllOf(Cases) => Cases.iter().all(|Case| Case.Holds(State)),
			When::NoneOf(Cases) => !Cases.iter().any(|Case| Case.Holds(State)),
		}
	}

	/// The condition as words, for a trace that has to say why a lane was
	/// chosen.
	pub fn Line(&self) -> String {
		match self {
			When::Always => "always".to_string(),
			When::More { Field, Than } => format!("{Field} > {Than}"),
			When::Less { Field, Than } => format!("{Field} < {Than}"),
			When::AtLeast { Field, Than } => format!("{Field} >= {Than}"),
			When::AtMost { Field, Than } => format!("{Field} <= {Than}"),
			When::Is { Field, Value } => format!("{Field} == {Value}"),
			When::Has { Field } => format!("{Field} present"),
			When::Lacks { Field } => format!("{Field} absent"),
			When::AnyOf(Cases) => {
				let Words:Vec<String> = Cases.iter().map(When::Line).collect();
				format!("any of ({})", Words.join(" or "))
			},
			When::AllOf(Cases) => {
				let Words:Vec<String> = Cases.iter().map(When::Line).collect();
				format!("all of ({})", Words.join(" and "))
			},
			When::NoneOf(Cases) => {
				let Words:Vec<String> = Cases.iter().map(When::Line).collect();
				format!("none of ({})", Words.join(" or "))
			},
		}
	}
}

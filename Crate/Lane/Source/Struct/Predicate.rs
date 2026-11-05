//! A condition as corpus data: what a lane states about the context, and the
//! JSON it is written as.
//!
//! This is the half of selection an adapter owns. A lane's `select` is a value
//! in the corpus file, so adding a lane, or teaching an existing lane a new
//! condition, is an edit to JSON and never a change to this crate. That is the
//! whole point: a corpus is the vocabulary of one game or one page, and a crate
//! that had to be rebuilt to learn a word would be a crate per adapter.
//!
//! # The JSON schema
//!
//! One predicate is one object with an `op` tag. Nine operators, and no
//! expression language: a string evaluated at run time is a parse failure
//! mid-run and a way for a data file to run logic.
//!
//! ```json
//! { "op": "always" }
//! { "op": "eq", "field": "verdict", "value": "blocked" }
//! { "op": "ne", "field": "gear", "value": -1 }
//! { "op": "in", "field": "verdict", "values": ["blocked", "stalled"] }
//! { "op": "gt", "field": "speed_kmh", "than": 60 }
//! { "op": "lt", "field": "moved_m", "than": 1.5 }
//! { "op": "and", "of": [ { "op": "gt", "field": "speed_kmh", "than": 60 },
//!                        { "op": "lt", "field": "route_cue_x", "than": 0.5 } ] }
//! { "op": "or", "of": [ { "op": "eq", "field": "verdict", "value": "blocked" },
//!                       { "op": "eq", "field": "verdict", "value": "stalled" } ] }
//! { "op": "not", "of": { "op": "eq", "field": "gear", "value": 0 } }
//! ```
//!
//! Three rules the schema keeps, and each is a decision rather than an
//! omission:
//!
//! - **`and` and `or` take a list, `not` takes one case.** A negation of a list
//!   is `not` around `or`, which reads the way it evaluates; an `and` of one
//!   case is that case, and an `and` of no cases is `always` - the same
//!   statement, spelled twice, is a corpus two authors can disagree in.
//! - **`gt` and `lt` are numeric and exclusive.** A range is two of them, and
//!   the inclusive spellings are not here because a threshold that includes its
//!   own bound is a rule that fires on the boundary reading and not the tick
//!   after - the kind of off-by-one nobody notices until a car is against a
//!   wall. A corpus that needs `>=` writes `not (lt ...)`.
//! - **A field path is a string, and a dot walks into an object.** `speed` reads
//!   the top level; `car.speed` reads the object under `car`. One flat name and
//!   a nested one are the same lookup, so an adapter whose context is flat and
//!   one whose context is grouped are both expressible without a second
//!   operator.
//!
//! # Missing is not a value
//!
//! Every comparison answers `false` when its field is absent or `null`. An
//! unobserved field is not a field of zero, and a predicate that treated it as
//! one would select a lane for a reading nobody took. `not` is the complement
//! of its inner case, so `not (eq gear 0)` **is** true for a missing `gear` -
//! that follows from `not` meaning "the inner case does not hold", and a corpus
//! that must not fire on a missing field writes the positive form
//! (`in`, `eq`, `gt`, `lt`) instead of negating it.
//!
//! # Numbers compare by value
//!
//! `eq` and `in` compare two numbers as numbers, so `1` and `1.0` are the same
//! reading - the ingestion's integer and a computed float are the same fact, and
//! a corpus author should not have to know which one the caller built. Every
//! other pair of values compares as JSON: a string is not a number, and `true`
//! is not `1`.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

use crate::Struct::{Error::Error, State::State};

/// One condition over a context, as the corpus writes it.
///
/// `Default` is [`Predicate::Always`], which is what a lane that states no
/// `select` means: it is not a lane that never applies, it is a lane the corpus
/// has not taught a condition to - and a corpus whose lanes all say that is
/// ambiguous by construction, which the selector reports rather than resolving
/// by taking the first.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(tag = "op", rename_all = "lowercase")]
pub enum Predicate {
	/// Holds for every context. A lane with no alternative.
	Always,
	/// The field equals the value.
	Eq {
		/// The field path the corpus names.
		#[serde(rename = "field")]
		Field:String,
		/// The value it must hold.
		#[serde(rename = "value")]
		Value:Value,
	},
	/// The field is present and holds something other than the value.
	Ne {
		/// The field path the corpus names.
		#[serde(rename = "field")]
		Field:String,
		/// The value it must not hold.
		#[serde(rename = "value")]
		Value:Value,
	},
	/// The field holds one of the values.
	In {
		/// The field path the corpus names.
		#[serde(rename = "field")]
		Field:String,
		/// Every value that satisfies the case.
		#[serde(rename = "values")]
		Values:Vec<Value>,
	},
	/// The field is a number greater than `Than`.
	Gt {
		/// The field path the corpus names.
		#[serde(rename = "field")]
		Field:String,
		/// The bound, exclusive.
		#[serde(rename = "than")]
		Than:f64,
	},
	/// The field is a number less than `Than`.
	Lt {
		/// The field path the corpus names.
		#[serde(rename = "field")]
		Field:String,
		/// The bound, exclusive.
		#[serde(rename = "than")]
		Than:f64,
	},
	/// Every case holds. No cases is `always`.
	And {
		/// The cases, all of which must hold.
		#[serde(rename = "of")]
		Of:Vec<Predicate>,
	},
	/// At least one case holds. No cases is `false`.
	Or {
		/// The cases, one of which must hold.
		#[serde(rename = "of")]
		Of:Vec<Predicate>,
	},
	/// The case does not hold.
	Not {
		/// The case being negated.
		#[serde(rename = "of")]
		Of:Box<Predicate>,
	},
}

impl Predicate {
	/// The field equals the value.
	pub fn Equal(Field:&str, Value:impl Into<Value>) -> Self {
		Predicate::Eq { Field:Field.to_string(), Value:Value.into() }
	}

	/// The field holds one of the values.
	pub fn Among(Field:&str, Values:Vec<Value>) -> Self { Predicate::In { Field:Field.to_string(), Values } }

	/// The field is a number greater than `Than`.
	pub fn Above(Field:&str, Than:f64) -> Self { Predicate::Gt { Field:Field.to_string(), Than } }

	/// The field is a number less than `Than`.
	pub fn Below(Field:&str, Than:f64) -> Self { Predicate::Lt { Field:Field.to_string(), Than } }

	/// Every case holds.
	pub fn All(Cases:Vec<Predicate>) -> Self { Predicate::And { Of:Cases } }

	/// At least one case holds.
	pub fn Any(Cases:Vec<Predicate>) -> Self { Predicate::Or { Of:Cases } }

	/// The case does not hold.
	///
	/// Named `Negate` rather than `Not` because the variant of that name is
	/// already reachable as `Predicate::Not`, and one name cannot be two
	/// functions in the same namespace.
	pub fn Negate(Case:Predicate) -> Self { Predicate::Not { Of:Box::new(Case) } }

	/// Whether this context satisfies the condition.
	///
	/// This is the whole of the semantics and the only place a corpus's
	/// condition is interpreted. It answers `false` rather than refusing for
	/// anything it cannot read - a missing field, a string where a number was
	/// compared, a path that walks into an array - because a context is built by
	/// an adapter under load and a predicate is not a place to fail a run. The
	/// caller learns what happened from which lane matched, and a corpus whose
	/// conditions can never hold is visible as `no-match` rather than as a
	/// panic at the tick that mattered.
	pub fn Matches(&self, Context:&Context) -> bool {
		match self {
			Predicate::Always => true,
			Predicate::Eq { Field, Value } => Context.Get(Field).is_some_and(|Held| Same(Held, Value)),
			Predicate::Ne { Field, Value } => Context.Get(Field).is_some_and(|Held| !Same(Held, Value)),
			Predicate::In { Field, Values } => {
				Context.Get(Field).is_some_and(|Held| Values.iter().any(|One| Same(Held, One)))
			},
			Predicate::Gt { Field, Than } => Context.Number(Field).is_some_and(|Held| Held > *Than),
			Predicate::Lt { Field, Than } => Context.Number(Field).is_some_and(|Held| Held < *Than),
			Predicate::And { Of } => Of.iter().all(|Case| Case.Matches(Context)),
			Predicate::Or { Of } => Of.iter().any(|Case| Case.Matches(Context)),
			Predicate::Not { Of } => !Of.Matches(Context),
		}
	}

	/// Every field path the condition reads, in the order it reads them.
	///
	/// For a loader that wants to say what a lane looks at, and for a trace that
	/// has to explain a selection without re-running it. Duplicates are kept:
	/// a condition that reads one field twice is a condition someone wrote
	/// deliberately.
	pub fn Fields(&self) -> Vec<&str> {
		match self {
			Predicate::Always => Vec::new(),
			Predicate::Eq { Field, .. }
			| Predicate::Ne { Field, .. }
			| Predicate::In { Field, .. }
			| Predicate::Gt { Field, .. }
			| Predicate::Lt { Field, .. } => vec![Field.as_str()],
			Predicate::And { Of } | Predicate::Or { Of } => Of.iter().flat_map(Predicate::Fields).collect(),
			Predicate::Not { Of } => Of.Fields(),
		}
	}

	/// The condition as words, for a trace that has to say why a lane was
	/// chosen.
	pub fn Line(&self) -> String {
		match self {
			Predicate::Always => "always".to_string(),
			Predicate::Eq { Field, Value } => format!("{Field} == {Value}"),
			Predicate::Ne { Field, Value } => format!("{Field} != {Value}"),
			Predicate::In { Field, Values } => {
				let Listed:Vec<String> = Values.iter().map(Value::to_string).collect();
				format!("{Field} in [{}]", Listed.join(", "))
			},
			Predicate::Gt { Field, Than } => format!("{Field} > {Than}"),
			Predicate::Lt { Field, Than } => format!("{Field} < {Than}"),
			Predicate::And { Of } => {
				let Words:Vec<String> = Of.iter().map(Predicate::Line).collect();
				format!("all of ({})", Words.join(" and "))
			},
			Predicate::Or { Of } => {
				let Words:Vec<String> = Of.iter().map(Predicate::Line).collect();
				format!("any of ({})", Words.join(" or "))
			},
			Predicate::Not { Of } => format!("not ({})", Of.Line()),
		}
	}
}

/// What a predicate reads: field path to JSON value.
///
/// A context is not a [`State`], although the two hold the same kind of thing.
/// A state is what a request *sends* - it is kept flat and every field in it is
/// a field the model will be shown. A context is what a condition *reads*, which
/// may be larger: a counter the caller keeps, a knob the corpus never sends, a
/// grouped reading under one key. Keeping them apart is what lets a lane select
/// on a field it would never spend tokens on.
///
/// [`Context::Of`] converts a state, so an adapter that already built one does
/// not build its context twice.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Context(BTreeMap<String, Value>);

impl Context {
	/// An empty context.
	pub fn New() -> Self { Self(BTreeMap::new()) }

	/// The context a JSON object names.
	///
	/// Anything that is not an object is refused: an array of readings has no
	/// field names, and a predicate over it could not name what it read.
	pub fn Of(Value:Value) -> Result<Self, Error> {
		match Value {
			Value::Object(Fields) => Ok(Self(Fields.into_iter().collect())),
			Other => Err(Error::CorpusInvalid("the context is not a JSON object", Other.to_string())),
		}
	}

	/// Record one field.
	pub fn Set(&mut self, Field:&str, Value:impl Into<Value>) { self.0.insert(Field.to_string(), Value.into()); }

	/// One field's value, or `None` when it is absent or null.
	///
	/// The exact key is tried first, so a field whose own name contains a dot is
	/// reachable by writing that name; only when no such key exists is the name
	/// read as a path. A path that walks into a scalar, an array or a null
	/// answers `None` rather than failing.
	pub fn Get(&self, Field:&str) -> Option<&Value> {
		if let Some(Direct) = self.0.get(Field).filter(|Value| !Value.is_null()) {
			return Some(Direct);
		}
		if !Field.contains('.') {
			return None;
		}
		let mut Steps = Field.split('.');
		let mut Cursor = self.0.get(Steps.next()?)?;
		for Step in Steps {
			Cursor = Cursor.get(Step)?;
		}
		(!Cursor.is_null()).then_some(Cursor)
	}

	/// One field as a number, when it is one.
	pub fn Number(&self, Field:&str) -> Option<f64> { self.Get(Field)?.as_f64() }

	/// Whether the context carries this field with a value.
	pub fn Has(&self, Field:&str) -> bool { self.Get(Field).is_some() }

	/// Every field name at the top level, sorted.
	pub fn Fields(&self) -> impl Iterator<Item = &String> { self.0.keys() }

	/// The context as a JSON object.
	pub fn Body(&self) -> Value { Value::Object(self.0.clone().into_iter().collect::<Map<String, Value>>()) }

	/// Whether anything was recorded at all.
	pub fn IsEmpty(&self) -> bool { self.0.is_empty() }
}

impl From<State> for Context {
	fn from(State:State) -> Self { Self(State.Body().as_object().cloned().unwrap_or_default().into_iter().collect()) }
}

impl From<&State> for Context {
	fn from(State:&State) -> Self { Self::from(State.clone()) }
}

/// Whether two JSON values are the same reading.
///
/// Two numbers compare as numbers - `1` and `1.0` are one reading written two
/// ways - and every other pair compares as JSON. `true` is not `1`, and a string
/// is never a number, because a corpus that matched across types would be a
/// corpus whose condition fired on a field it misunderstood.
fn Same(Left:&Value, Right:&Value) -> bool {
	match (Left.as_f64(), Right.as_f64()) {
		(Some(Left), Some(Right)) => Left == Right,
		_ => Left == Right,
	}
}

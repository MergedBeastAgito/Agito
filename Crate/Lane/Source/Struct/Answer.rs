//! What came back, typed.
//!
//! The endpoint answers per question: a `choice` yields the chosen option and a
//! probability for every option, a `score` yields a value and its legend. Both
//! are kept as they arrived, because the probability *distribution* is the
//! interesting part - a model that spreads its mass across five options has not
//! committed to any of them, and a caller that only kept the winner could not
//! tell that apart from certainty.
//!
//! The chosen option's probability is never re-normalised. `0.2975` against four
//! rivals is a different fact from `0.9` against one, and a crate that scaled
//! either into "confidence" would be inventing a number the model did not give.

use std::collections::BTreeMap;

use serde_json::Value;

use crate::Struct::Request::Request;

/// What a reply reported spending.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Usage {
	/// Input tokens, as the reply counts them.
	pub Input:u64,
	/// Output tokens, as the reply counts them.
	pub Output:u64,
}

/// One question's answer.
#[derive(Clone, Debug, PartialEq)]
pub enum Answered {
	/// A choice, with the distribution it was chosen from.
	Choice {
		/// The option the model chose.
		Choice:String,
		/// Every allowed option's probability, as the reply gave them.
		Probabilities:BTreeMap<String, f64>,
	},
	/// A position on a scale, with the legend it sits on.
	Score {
		/// The raw score, on the legend's own scale.
		Score:f64,
		/// The scale's words, lowest first. Empty when the reply named none and
		/// the request had none to fill in.
		Legend:Vec<String>,
	},
	/// A question this crate does not interpret, as the reply gave it.
	Other(Value),
}

impl Answered {
	/// The type the reply declared.
	pub fn Type(&self) -> &str {
		match self {
			Answered::Choice { .. } => "choice",
			Answered::Score { .. } => "score",
			Answered::Other(Value) => Value.get("type").and_then(Value::as_str).unwrap_or("other"),
		}
	}
}

/// One reply, typed.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Answer {
	/// The model that answered, as the reply names it.
	pub Model:String,
	/// Every question's answer, by name.
	pub Questions:BTreeMap<String, Answered>,
	/// What the reply reported spending.
	pub Usage:Usage,
}

impl Answer {
	/// One question's answer.
	pub fn Answer(&self, Question:&str) -> Option<&Answered> { self.Questions.get(Question) }

	/// The option a choice question chose, and its own probability.
	pub fn Choice(&self, Question:&str) -> Option<(&str, f64)> {
		let Answered::Choice { Choice, Probabilities } = self.Questions.get(Question)? else {
			return None;
		};
		Some((Choice.as_str(), Probabilities.get(Choice).copied().unwrap_or(0.0)))
	}

	/// One option's probability, chosen or not.
	pub fn Probability(&self, Question:&str, Option:&str) -> Option<f64> {
		let Answered::Choice { Probabilities, .. } = self.Questions.get(Question)? else {
			return None;
		};
		Probabilities.get(Option).copied()
	}

	/// A score question's raw value.
	pub fn Score(&self, Question:&str) -> Option<f64> {
		let Answered::Score { Score, .. } = self.Questions.get(Question)? else {
			return None;
		};
		Some(*Score)
	}

	/// A score question's value as a position on its own legend, 0..1.
	///
	/// The legend comes from the reply, or from the request when the reply named
	/// none ([`Answer::Filled`]). A scale of one word has no position and
	/// answers `None` rather than dividing by zero.
	pub fn Position(&self, Question:&str) -> Option<f64> {
		let Answered::Score { Score, Legend } = self.Questions.get(Question)? else {
			return None;
		};
		let Steps = Legend.len().checked_sub(1)?;
		if Steps == 0 {
			return None;
		}
		Some((Score / Steps as f64).clamp(0.0, 1.0))
	}

	/// The same answer, with any legend the reply omitted filled in from the
	/// question the request asked.
	///
	/// A reply is allowed to answer a score without repeating its scale, and a
	/// caller holding only the answer could then not normalise it. The request
	/// is in hand at parse time, so this is the moment to join them - and it is
	/// a separate step rather than an argument to the parser so that a captured
	/// body can still be parsed on its own.
	pub fn Filled(&self, Request:&Request) -> Answer {
		let Questions = self
			.Questions
			.iter()
			.map(|(Name, Answered)| {
				let Answered = match Answered {
					Answered::Score { Score, Legend } if Legend.is_empty() => {
						let Legend = Request.Question(Name).map(|Question| Question.Criteria()).unwrap_or_default();
						Answered::Score { Score:*Score, Legend }
					},
					Other => Other.clone(),
				};
				(Name.clone(), Answered)
			})
			.collect();
		Answer { Model:self.Model.clone(), Questions, Usage:self.Usage }
	}
}

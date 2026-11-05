//! One typed question, which is the difference between a decision and a parse.
//!
//! A model that answers in free text has to be parsed, and a parse failure
//! mid-run is a crash. A typed question cannot answer outside its options: it
//! returns a probability per option, so "left at 0.11, right at 0.53" is a
//! decision a caller thresholds rather than interprets.
//!
//! Three shapes, and the third is the one that keeps the corpus honest:
//!
//! - `choice` names its options as criteria, so every answer is one of them;
//! - `score` names an ordered legend, so its answer is a position on it;
//! - anything else is carried verbatim. A question type this crate has not been
//!   taught is not a corpus that must be refused - the endpoint may well
//!   understand it - but it is also not something this crate can turn into a
//!   probability, and pretending otherwise would be a typed answer that is not
//!   one.

use std::collections::BTreeMap;

use serde_json::{Value, json};

/// One question, as the corpus declares it.
#[derive(Clone, Debug, PartialEq)]
pub enum Question {
	/// A choice between named options.
	Choice {
		/// What the model is asked.
		Instructions:String,
		/// Every option, with the sentence that describes it.
		Criteria:BTreeMap<String, String>,
	},
	/// A position on an ordered scale.
	Score {
		/// What the model is asked.
		Instructions:String,
		/// The scale, from its lowest word to its highest.
		Criteria:Vec<String>,
	},
	/// A question type this crate does not interpret, carried as it arrived.
	Other {
		/// The type the corpus declares.
		Type:String,
		/// The question body, exactly as the corpus wrote it.
		Body:Value,
	},
}

impl Question {
	/// The type the corpus declares.
	pub fn Type(&self) -> &str {
		match self {
			Question::Choice { .. } => "choice",
			Question::Score { .. } => "score",
			Question::Other { Type, .. } => Type,
		}
	}

	/// What the model is asked.
	pub fn Instructions(&self) -> &str {
		match self {
			Question::Choice { Instructions, .. } | Question::Score { Instructions, .. } => Instructions,
			Question::Other { .. } => "",
		}
	}

	/// The options or the legend, in the order the corpus wrote them. Empty for
	/// a type this crate does not interpret.
	pub fn Criteria(&self) -> Vec<String> {
		match self {
			Question::Choice { Criteria, .. } => Criteria.keys().cloned().collect(),
			Question::Score { Criteria, .. } => Criteria.clone(),
			Question::Other { .. } => Vec::new(),
		}
	}

	/// A raw score as a position on this question's own scale, 0..1.
	///
	/// Normalised by the legend the corpus wrote rather than by a number in the
	/// caller: a four-word scale is 0..3, a seven-word one is 0..6, and a
	/// confidence threshold that assumed 3 would silently mean something else
	/// on the second. `None` when this is not a score, or when the legend is too
	/// short to be a scale.
	pub fn Position(&self, Score:f64) -> Option<f64> {
		let Question::Score { Criteria, .. } = self else {
			return None;
		};
		let Steps = Criteria.len().checked_sub(1)?;
		if Steps == 0 {
			return None;
		}
		Some((Score / Steps as f64).clamp(0.0, 1.0))
	}

	/// The question as the request carries it.
	pub fn Body(&self) -> Value {
		match self {
			Question::Choice { Instructions, Criteria } => {
				json!({
					"type": "choice",
					"instructions": Instructions,
					"criteria": Criteria,
				})
			},
			Question::Score { Instructions, Criteria } => {
				json!({
					"type": "score",
					"instructions": Instructions,
					"criteria": Criteria,
				})
			},
			Question::Other { Body, .. } => Body.clone(),
		}
	}
}

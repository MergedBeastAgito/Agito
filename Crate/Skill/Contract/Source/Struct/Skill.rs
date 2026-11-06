//! One skill's record, and one rule it breaks.
//!
//! The record is what a sweep carries: the directory name is the skill's name,
//! because that is the address a reader types and an index cites, and
//! `Directory` is excluded from the digest so the same corpus digests the same
//! in two checkouts.

use serde::{Deserialize, Serialize};

use crate::Struct::Error::Rule;

/// One skill directory, as `discover` reports it.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Skill {
	/// The directory name, which is the skill's name.
	pub Name:String,
	/// Where the skill lives. Excluded from the digest on purpose.
	pub Directory:String,
	/// The frontmatter's `description`, trimmed; empty when there is none.
	pub Description:String,
	/// The file names directly under `Reference/`, excluding its index, sorted.
	pub References:Vec<String>,
}

/// One rule a skill breaks, as `validate` reports it.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Finding {
	/// The skill the finding is about.
	pub Skill:String,
	/// Which of the three rules it breaks.
	pub Rule:String,
	/// What was observed, kept apart from the sentence a human reads.
	pub Detail:String,
}

impl Finding {
	/// A finding for one skill, from a rule and what was observed.
	pub fn Of(Skill:&Skill, Rule:Rule, Detail:impl Into<String>) -> Self {
		Self { Skill:Skill.Name.clone(), Rule:Rule.Name().to_string(), Detail:Detail.into() }
	}

	/// The rule this finding reports, if it is one of the three.
	pub fn Code(&self) -> Option<Rule> { Rule::Parse(&self.Rule) }
}

//! Every rule a skill breaks.
//!
//! Three checks, in the order the manifest names them, and each one reports rather
//! than returns early: a skill can be unparsable and uncited at once, and a sweep
//! that stopped at the first finding would hide half the corpus's problems.

use std::path::Path;

use crate::Fn::Discover::{CorpusIndex, References};
use crate::Fn::Entries::{BodyFile, CorpusIndexes, IndexFile, ReferenceDirectory, Text};
use crate::Fn::Frontmatter::Split;
use crate::Fn::Names::Names;
use crate::Struct::Error::Rule;
use crate::Struct::Skill::{Finding, Skill};

/// Every rule `Skill` breaks. An empty vector is a valid skill.
pub fn Validate(Skill: &Skill) -> Vec<Finding> {
	let mut Findings = Vec::new();
	let Directory = Path::new(&Skill.Directory);

	match Text(&Directory.join(BodyFile)) {
		None => Findings.push(Finding::Of(
			Skill,
			Rule::Unparsable,
			format!("{} carries no {BodyFile}", Skill.Directory),
		)),
		Some(Body) => match Split(&Body) {
			Err(Reason) => Findings.push(Finding::Of(
				Skill,
				Rule::Unparsable,
				format!("the frontmatter does not parse: {Reason}"),
			)),
			Ok(Data) => {
				let Description = Data
					.get("description")
					.map(|Value| Value.trim())
					.unwrap_or_default();
				if Description.is_empty() {
					Findings.push(Finding::Of(
						Skill,
						Rule::Unparsable,
						"the frontmatter states no description",
					));
				}
				let Name = Data.get("name").map(|Value| Value.trim()).unwrap_or_default();
				if !Name.is_empty() && Name.to_lowercase() != Skill.Name.to_lowercase() {
					Findings.push(Finding::Of(
						Skill,
						Rule::Unparsable,
						format!(
							"the frontmatter names {} but the directory is {}",
							Quote(Name),
							Quote(&Skill.Name)
						),
					));
				}
			}
		},
	}

	let Index = Text(&Directory.join(ReferenceDirectory).join(IndexFile));
	for Reference in References(Directory) {
		match &Index {
			None => Findings.push(Finding::Of(
				Skill,
				Rule::UnlistedReference,
				format!(
					"{ReferenceDirectory}/{Reference} is not named in {ReferenceDirectory}/{IndexFile}, because there is no {ReferenceDirectory}/{IndexFile}"
				),
			)),
			Some(Index) if !Names(Index, &Reference) => Findings.push(Finding::Of(
				Skill,
				Rule::UnlistedReference,
				format!("{ReferenceDirectory}/{Reference} is not named in {ReferenceDirectory}/{IndexFile}"),
			)),
			Some(_) => {}
		}
	}

	let Corpus = Directory.parent().and_then(CorpusIndex);
	match Corpus {
		None => Findings.push(Finding::Of(
			Skill,
			Rule::Uncited,
			format!(
				"the corpus root carries no {}, so nothing points at {}",
				CorpusIndexes.join(" or "),
				Skill.Name
			),
		)),
		Some((Name, Text)) if !Names(&Text, &Skill.Name) => Findings.push(Finding::Of(
			Skill,
			Rule::Uncited,
			format!("{Name} does not name {}/", Skill.Name),
		)),
		Some(_) => {}
	}

	Findings
}

/// A name as a quoted literal, so a finding cannot be misread.
fn Quote(Value: &str) -> String {
	serde_json::to_string(Value).unwrap_or_else(|_| format!("{Value:?}"))
}

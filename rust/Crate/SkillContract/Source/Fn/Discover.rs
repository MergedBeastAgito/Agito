//! Finding the skills under a corpus root, and reading one of them.
//!
//! Code-unit order, not a locale-aware collation: a sweep must not change meaning
//! with the machine's locale, and `String`'s `Ord` is exactly the byte order the
//! TypeScript implementation compares with.

use std::path::Path;

use crate::Fn::Entries::{BodyFile, CorpusIndexes, Entries, IndexFile, IsDirectory, IsFile, ReferenceDirectory, Text};
use crate::Fn::Frontmatter::Split;
use crate::Struct::Skill::Skill;

/// Every skill directory under `Root`, sorted by name.
pub fn Discover(Root: &Path) -> Vec<Skill> {
	let mut Skills = Vec::new();
	for Entry in Entries(Root) {
		if !IsDirectory(&Entry, &Root.join(&Entry.Name)) {
			continue;
		}
		Skills.push(Read(Root, &Entry.Name));
	}
	Skills.sort_by(|Left, Right| Left.Name.cmp(&Right.Name));
	Skills
}

/// One skill's record: the directory name is the skill's name, because that is
/// the address a reader types and an index cites.
pub fn Read(Root: &Path, Name: &str) -> Skill {
	let Directory = Root.join(Name);
	let Body = Text(&Directory.join(BodyFile)).unwrap_or_default();
	let Description = Split(&Body)
		.ok()
		.and_then(|Data| Data.get("description").map(|Value| Value.trim().to_string()))
		.unwrap_or_default();
	Skill {
		Name: Name.to_string(),
		Directory: Directory.to_string_lossy().into_owned(),
		Description,
		References: References(&Directory),
	}
}

/// The file names directly under `Reference/`, excluding the index itself.
pub fn References(Directory: &Path) -> Vec<String> {
	let mut Names: Vec<String> = Entries(&Directory.join(ReferenceDirectory))
		.iter()
		.filter(|Entry| IsFile(Entry))
		.map(|Entry| Entry.Name.clone())
		.filter(|Name| Name != IndexFile)
		.collect();
	Names.sort();
	Names
}

/// The first index a corpus root carries, and the name it was found under.
pub fn CorpusIndex(Directory: &Path) -> Option<(String, String)> {
	for Name in CorpusIndexes {
		if let Some(Text) = Text(&Directory.join(Name)) {
			return Some((Name.to_string(), Text));
		}
	}
	None
}

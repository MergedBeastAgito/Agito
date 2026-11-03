//! The three rules `validate` reports, and the only three it may report.
//!
//! They mirror `provides.errors` in `module.json`, which is what a harness
//! reads; the contract test compares the two so the manifest cannot drift from
//! the code.
//!
//! - `unparsable` - the frontmatter does not describe the skill that directory
//!   is: no `SKILL.md`, no frontmatter block, a block that is not a mapping, no
//!   `description`, or a `name` that disagrees with the directory. The
//!   directory is the skill's address, so a frontmatter name that contradicts
//!   it is a rename that stopped halfway.
//! - `unlisted-reference` - a file under `Reference/` that `Reference/Index.md`
//!   does not name, which is a reader's dead end.
//! - `uncited` - the corpus root's index - `Index.md`, or `AGENTS.md` - does
//!   not name this skill, which is a skill no reader is pointed at.
//!
//! Two rules the source implementation enforces are deliberately not here:
//! whether the body carries a claim-to-test table, and whether a second,
//! reader-facing index names the skill. Both are policies of one repository
//! rather than properties of a corpus, and a port that hard-coded them would
//! refuse every corpus that does not share that repository's house style.

use std::fmt;

/// The rules, in the order the manifest names them.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Rule {
	/// The frontmatter does not describe the skill that directory is.
	Unparsable,
	/// A file under `Reference/` that the index does not name.
	UnlistedReference,
	/// The corpus root's index does not name this skill.
	Uncited,
}

impl Rule {
	/// The string the manifest uses.
	pub const fn Name(self) -> &'static str {
		match self {
			Rule::Unparsable => "unparsable",
			Rule::UnlistedReference => "unlisted-reference",
			Rule::Uncited => "uncited",
		}
	}

	/// Every rule, so a harness can compare the manifest against the type.
	pub const fn All() -> [Rule; 3] { [Rule::Unparsable, Rule::UnlistedReference, Rule::Uncited] }

	/// The rule a manifest string names, if it names one of the three.
	pub fn Parse(Value:&str) -> Option<Rule> { Rule::All().into_iter().find(|Rule| Rule.Name() == Value) }
}

/// The three rules, in the order the manifest names them.
pub const Rules:[Rule; 3] = Rule::All();

/// The one failure this capability has, for the caller that must fail loudly.
///
/// `discover` and `validate` do not return it: a malformed skill is a finding.
/// A command that exits non-zero, though, needs one value to branch on, and
/// inventing a fourth vocabulary at the command layer is how a port stops being
/// the contract. It carries the same three codes as a finding.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Error {
	/// The frontmatter does not describe the skill that directory is.
	Unparsable {
		/// The sentence a human reads.
		Message:String,
		/// What was observed.
		Detail:String,
	},
	/// A file under `Reference/` that the index does not name.
	UnlistedReference {
		/// The sentence a human reads.
		Message:String,
		/// What was observed.
		Detail:String,
	},
	/// The corpus root's index does not name this skill.
	Uncited {
		/// The sentence a human reads.
		Message:String,
		/// What was observed.
		Detail:String,
	},
}

impl Error {
	/// The code this failure carries.
	pub const fn Rule(&self) -> Rule {
		match self {
			Error::Unparsable { .. } => Rule::Unparsable,
			Error::UnlistedReference { .. } => Rule::UnlistedReference,
			Error::Uncited { .. } => Rule::Uncited,
		}
	}

	/// The sentence a human reads.
	pub fn Message(&self) -> &str {
		match self {
			Error::Unparsable { Message, .. }
			| Error::UnlistedReference { Message, .. }
			| Error::Uncited { Message, .. } => Message,
		}
	}

	/// What was observed, which may be empty.
	pub fn Detail(&self) -> &str {
		match self {
			Error::Unparsable { Detail, .. }
			| Error::UnlistedReference { Detail, .. }
			| Error::Uncited { Detail, .. } => Detail,
		}
	}
}

impl fmt::Display for Error {
	fn fmt(&self, Formatter:&mut fmt::Formatter<'_>) -> fmt::Result {
		write!(Formatter, "{}: {}", self.Rule().Name(), self.Message())
	}
}

impl std::error::Error for Error {}

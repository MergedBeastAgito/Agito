//! The port `agito.skill.validate` promises.
//!
//! The port states more than data shapes, so the semantics two adapters would
//! otherwise disagree about are fixed here rather than left to an
//! implementation:
//!
//! - `discover` is deterministic. Two runs over the same tree return the same
//!   order, sorted by skill name rather than by whatever the filesystem
//!   answered, because directory order is not stable across platforms or file
//!   systems.
//! - `validate` reports, it does not fail. A malformed skill is a finding, so
//!   one bad directory cannot abort a sweep over a corpus of good ones.
//! - `digest` is a pure function of the records it is handed. It reads no file,
//!   so a modification time cannot enter it, and `directory` is excluded from
//!   it, so the same corpus digests the same in two checkouts.
//! - Stateless means stateless. Nothing is cached between calls, and `validate`
//!   re-reads the skill's directory instead of trusting the record it is given:
//!   a record is a claim about a directory, and a cache of that claim is how a
//!   sweep reports yesterday's corpus.
//!
//! The methods are boxed futures rather than plain functions because the port
//! shape is preserved from the TypeScript capability, where every verb answered
//! a promise. Nothing in this crate awaits, and the work is filesystem work: a
//! caller on an async runtime should drive these from a blocking context rather
//! than assume the box makes the read asynchronous.

use ::RunTime::FutureBox;

use crate::Struct::Skill::{Finding, Skill};

/// The one port this capability offers.
pub trait SkillContract: Send + Sync {
	/// Every skill directory under the corpus root.
	fn Discover<'a>(&'a self, Root:&'a str) -> FutureBox<'a, Vec<Skill>>;
	/// Every rule a skill breaks. An empty vector is a valid corpus.
	fn Validate<'a>(&'a self, Skill:&'a Skill) -> FutureBox<'a, Vec<Finding>>;
	/// A stable digest of the corpus: names, order and reference lists.
	fn Digest(&self, Skills:&[Skill]) -> String;
}

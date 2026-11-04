//! `agentic.skill.validate`, from `agentic-skill-contract`: discover a corpus,
//! report what a skill breaks, digest the result.
//!
//! [`Contract`] is the entry point, built by [`Create`]; [`Skill`] and
//! [`Finding`] are what it answers with, and [`SkillContract`] is the trait it
//! implements. [`Rule`] is the failure vocabulary - `unparsable`,
//! `unlisted-reference`, `uncited` - and it is a rule rather than an exception
//! on purpose: validation reports, so one malformed skill cannot abort a sweep
//! over a corpus of good ones.
//!
//! [`Rules`] is the collection those findings arrive in, and [`Error`] is what a
//! call that could not read the tree at all fails with.

pub use AgenticSkillContract::{Contract, Create, Error, Finding, Rule, Rules, Skill, SkillContract};

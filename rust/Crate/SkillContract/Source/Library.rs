//! `agentic.skill.validate`: discover a corpus, report what a skill breaks,
//! digest the result.
//!
//! A corpus is a directory of skill directories. A skill is a directory carrying
//! a `SKILL.md` and, when it has worked instances, a `Reference/` tree. A skill's
//! `Reference/` carries an `Index.md` and the corpus root carries an index, and
//! each index names its siblings: the root's names the skills, a skill's names its
//! files.
//!
//! Everything here reads the filesystem and nothing here caches it. There is no
//! module state, so two calls with the same arguments answer the same findings and
//! a call after the tree changed answers the new ones.
//!
//! The port's three codes are [`Struct::Error::Rule`]s rather than exceptions:
//! `validate` reports, it does not throw, so one malformed skill cannot abort a
//! sweep over a corpus of good ones. A rule carries `detail` for the same reason
//! an error does - the sentence is for a reader, the observation is for whoever
//! has to fix it.

#![allow(non_snake_case)]
#![allow(non_upper_case_globals)]
#![allow(clippy::upper_case_acronyms)]

pub mod Fn;
pub mod Struct;

pub use crate::Struct::Contract::{Create, Contract};
pub use crate::Struct::Error::{Error, Rule, Rules};
pub use crate::Struct::Port::SkillContract;
pub use crate::Struct::Skill::{Finding, Skill};

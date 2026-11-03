//! The implementation of the port in `Port.rs`: a stateless contract over a
//! directory of skill directories.
//!
//! There is no state beyond the constants in `Fn/`, so two calls with the same
//! arguments answer the same findings and a call after the tree changed answers
//! the new ones. `digest` is the one exception, and it is the opposite of a cache:
//! it is a pure function of the records it is handed, so it can be computed for a
//! corpus that is not on this machine at all.

use std::path::Path;

use AgenticRuntime::FutureBox;

use crate::Fn::Digest::Digest;
use crate::Fn::Discover::Discover;
use crate::Fn::Validate::Validate;
use crate::Struct::Port::SkillContract;
use crate::Struct::Skill::{Finding, Skill};

/// A stateless contract. It holds nothing, which is the point.
#[derive(Debug, Default)]
pub struct Contract;

/// Build one. A contract has no configuration, so this takes none.
pub fn Create() -> Contract {
	Contract
}

impl SkillContract for Contract {
	fn Discover<'a>(&'a self, Root: &'a str) -> FutureBox<'a, Vec<Skill>> {
		Box::pin(async move { Discover(Path::new(Root)) })
	}

	fn Validate<'a>(&'a self, Skill: &'a Skill) -> FutureBox<'a, Vec<Finding>> {
		Box::pin(async move { Validate(Skill) })
	}

	fn Digest(&self, Skills: &[Skill]) -> String {
		Digest(Skills)
	}
}

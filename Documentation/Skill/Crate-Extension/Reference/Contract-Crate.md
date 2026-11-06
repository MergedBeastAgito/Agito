# The smallest complete crate

`Crate/Skill/Contract` is the worked instance to copy: one port, three error codes, no dependency
beyond the runtime and a serialiser, no state, and no script. It carries every part a crate needs
and nothing a crate merely may have, so a new crate can be read against it line by line.

## The tree

```
Crate/Skill/Contract/
	Cargo.toml
	module.json
	Source/Library.rs
	Source/Fn.rs
	Source/Fn/Digest.rs
	Source/Fn/Discover.rs
	Source/Fn/Entry.rs
	Source/Fn/Frontmatter.rs
	Source/Fn/Name.rs
	Source/Fn/Validate.rs
	Source/Struct.rs
	Source/Struct/Contract.rs
	Source/Struct/Error.rs
	Source/Struct/Port.rs
	Source/Struct/Skill.rs
```

Six verbs, four nouns, no `Main.rs`, no `Script/`, no `Test/` - the suite lives in the crate's own
`#[cfg(test)]` modules because there is no process to drive.

## `Cargo.toml`

```toml
[package]
name = "agito-skill-contract"
version.workspace = true
edition.workspace = true
rust-version.workspace = true
license.workspace = true
publish.workspace = true
description = "Corpus discovery, validation and packaging: a stateless contract over a directory of skills."

[lib]
name = "Contract"
path = "Source/Library.rs"

[dependencies]
agito-runtime = { workspace = true }
serde = { workspace = true }
serde_json = { workspace = true }
sha2 = { workspace = true }

[lints.rust]
non_snake_case = "allow"
non_upper_case_globals = "allow"

[lints.clippy]
upper_case_acronyms = "allow"
```

The package id is `agito-skill-contract`; the library is `Contract`; the directory is
`Crate/Skill/Contract`. Three spellings of one name, each fixed by something outside the crate:
Cargo requires kebab for the package, Rust forbids a nested crate root, and the style rule makes the
leaf the library.

## `module.json`

```json
{
	"Name": "@agito/skill-contract",
	"Kind": "Capability",
	"Version": "0.1.3",
	"Provide": {
		"Port": "agito.skill.validate",
		"Error": ["unparsable", "unlisted-reference", "uncited"]
	},
	"Require": {},
	"Artefact": {
		"Entry": "Target/debug/libContract.rlib",
		"Hash": null,
		"RunTime": "rust>=1.88"
	},
	"State": "Stateless",
	"Cost": {
		"ColdMs": 30,
		"WarmMs": 5,
		"Network": false
	},
	"Verify": "agito verify --module @agito/skill-contract"
}
```

`Require` is an empty object rather than absent: a crate that needs nothing states that, and a
reader should not have to tell an empty requirement from a forgotten one. `Artefact.Entry` names the
debug `.rlib`, which is the profile the manifest records; the release gate writes `Target/release/`
and does not contradict it.

## `Source/Library.rs`

```rust
//! `agito.skill.validate`: discover a corpus, report what a skill breaks,
//! digest the result.

#![allow(non_snake_case)]
#![allow(non_upper_case_globals)]
#![allow(clippy::upper_case_acronyms)]

pub mod Fn;
pub mod Struct;

pub use crate::Struct::{
	Contract::{Contract, Create},
	Error::{Error, Rule, Rules},
	Port::SkillContract,
	Skill::{Finding, Skill},
};
```

The module doc states the port and what the crate refuses to do - here, that a malformed skill is a
finding rather than an exception, so one bad skill cannot abort a sweep. `Struct.rs` holds `pub mod`
lines only; the re-exports are here, which is what keeps `pub use ...::Decision::Decision` out of
the scope where `pub mod Decision;` already named it.

## The error type, and the readiness table

```rust
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Rule {
	Unparsable,
	UnlistedReference,
	Uncited,
}

impl Rule {
	pub const fn Name(self) -> &'static str {
		match self {
			Rule::Unparsable => "unparsable",
			Rule::UnlistedReference => "unlisted-reference",
			Rule::Uncited => "uncited",
		}
	}

	pub const fn All() -> [Rule; 3] { [Rule::Unparsable, Rule::UnlistedReference, Rule::Uncited] }
}
```

`All()` exists so a harness can compare the manifest against the type, and `Name()` is the string
the manifest uses - the two are the whole contract `verify` reads. The crate is then named once, in
`Crate/Agito/Source/Fn/Module/Verify.rs`:

```rust
"@agito/skill-contract" => SkillRule::All().into_iter().map(SkillRule::Name).collect(),
```

A crate missing from that match is not an error: `Declared` returns `None`, and the report becomes
the foreign one, with `code: null`. That is the right answer for a third party's adapter and the
wrong one for a crate in this tree, so the arm is part of adding the crate.

## What to read in the crate, in order

1. `Source/Struct/Port.rs` - the port as a type, before any function takes it.
2. `Source/Struct/Error.rs` - the codes, with the doc comment saying which rules were deliberately
   left out and why.
3. `Source/Fn/Validate.rs` - the one verb that reports rather than throws.
4. `module.json` - and check `Provide.Error` against `Rule::All()` by eye, then with `verify`.

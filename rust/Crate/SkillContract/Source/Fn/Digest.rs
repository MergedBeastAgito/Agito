//! The corpus digest.
//!
//! A pure function of the records it is handed: no file is read, so a modification
//! time cannot enter it, and `Directory` is excluded, so the same corpus digests
//! the same in two checkouts. The canonical form is the JSON the TypeScript
//! implementation hashes - `[[name, description, references], ...]` - which is why
//! the digest of a given corpus is the same value across the two trees and the
//! contract test can assert a recorded constant.

use sha2::{Digest as _, Sha256};

use crate::Struct::Skill::Skill;

/// The corpus digest, prefixed with the algorithm that produced it.
pub fn Digest(Skills: &[Skill]) -> String {
	let Canonical: Vec<serde_json::Value> = Skills
		.iter()
		.map(|Skill| serde_json::json!([Skill.Name, Skill.Description, Skill.References]))
		.collect();
	let Bytes = serde_json::to_string(&Canonical).unwrap_or_else(|_| "[]".to_string());
	let Sum = Sha256::digest(Bytes.as_bytes());
	// Hex by hand: the digest type's own `LowerHex` is behind a feature this crate
	// does not need, and a format string is not a reason to take one.
	let Hex: String = Sum.iter().map(|Byte| format!("{Byte:02x}")).collect();
	format!("sha256:{Hex}")
}

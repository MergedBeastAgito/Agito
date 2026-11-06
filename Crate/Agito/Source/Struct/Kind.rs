//! The kinds a manifest may declare.
//!
//! One vocabulary, read by the manifest reader and by the readiness report. A
//! kind outside it is not a module with an unusual role: it is a manifest this
//! binary cannot reason about, and refusing it is the difference between a check
//! and a shrug.
//!
//! The same list used to be stated on the other side of the same fact - which
//! arrow each kind may draw - by a boundary check that read every crate's
//! `module.json` and `Cargo.toml`. That check is deleted, so this file is the
//! only statement of the vocabulary: a kind added here is a kind the readiness
//! report accepts, and nothing else decides whether the arrows around it make
//! sense.

/// The kinds a manifest may declare, in the layer order the design uses.
///
/// `adapter` is the one a crate outside this workspace declares: it provides a
/// port against a substrate this workspace did not write, and the composition
/// root holds it as a value rather than naming it.
pub const Kinds:[&str; 6] = ["RunTime", "Capability", "Adapter", "Workflow", "Composition", "Facade"];

/// Whether a manifest may declare this kind.
pub fn Known(Kind:&str) -> bool { Kinds.contains(&Kind) }

/// The vocabulary, for a refusal that has to name it.
pub fn Names() -> String { Kinds.join(", ") }

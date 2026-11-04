//! The kinds a manifest may declare.
//!
//! One vocabulary, read by the manifest reader and by the readiness report. A
//! kind outside it is not a module with an unusual role: it is a manifest this
//! binary cannot reason about, and refusing it is the difference between a check
//! and a shrug.
//!
//! `Crate/Agentic/Test/Boundaries.rs` states the same list on the other side of
//! the same fact - which arrow each kind may draw - and the two are read
//! together: this one decides what a manifest may call itself, that one decides
//! what a crate may import once it has. A kind added to one and not the other is
//! a module that verifies and cannot be placed, or one that is placed and cannot
//! be verified.

/// The kinds a manifest may declare, in the layer order the design uses.
///
/// `adapter` is the one a crate outside this workspace declares: it provides a
/// port against a substrate this workspace did not write, and the composition
/// root holds it as a value rather than naming it.
pub const Kinds:[&str; 6] = ["runtime", "capability", "adapter", "workflow", "composition", "facade"];

/// Whether a manifest may declare this kind.
pub fn Known(Kind:&str) -> bool { Kinds.contains(&Kind) }

/// The vocabulary, for a refusal that has to name it.
pub fn Names() -> String { Kinds.join(", ") }

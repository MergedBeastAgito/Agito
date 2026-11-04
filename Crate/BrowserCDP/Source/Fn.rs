//! Functions: the error mapping, and nothing else yet.
//!
//! The session's own logic is in `Struct/Session.rs` because it is the
//! adapter's one stateful thing, and splitting it across a `Fn` tree would make
//! the lifetime it owns harder to see, not easier.

pub mod Classify;

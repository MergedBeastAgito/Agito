//! `agentic.adapter`, from `agentic-adapter`: the shape a substrate this
//! workspace did not write implements.
//!
//! [`Adapter`] is the port; [`Serves`] is the one claim check a registry-free
//! composition root needs, because it is the only place both the value and the
//! manifest that named it are in hand; [`StandIn`] is the scripted double a
//! consumer's own tests use instead of a substrate.
//!
//! This module exists because the facade is what a consumer outside this tree
//! depends on, and a third-party adapter is exactly such a consumer. It is
//! `agentic-sdk`'s one dependency that ships no port of its own: `agentic-adapter`
//! declares `kind: "capability"` and holds the shape a foreign crate implements,
//! which is why the edge is inward and why this crate is not re-exporting a
//! substrate.
//!
//! What is not here: nothing. The adapter port has no internal seam to hide - it
//! is a trait, a function and a double, and a consumer needs all three.

pub use AgenticAdapter::{Adapter, Error, Serves, StandIn};

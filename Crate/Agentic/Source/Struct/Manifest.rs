//! The workspace's manifest table, which now lives in `agentic-sdk`.
//!
//! The table is part of the facade's surface: a consumer asking "what does this
//! workspace ship" is asking about the items it just imported, and the CLI's
//! `manifests` verb is one consumer of that answer rather than the owner of it.
//! So the definition moved down to `Crate/Sdk/` and this module re-exports it.
//!
//! One definition, two paths: the binary keeps `Agentic::All` and every internal
//! `crate::Struct::Manifest::Find` keeps resolving, while a consumer that depends
//! only on the facade gets the same values. A copy would have been a second
//! answer to the same question - two `Manifest` types, and a `Find` that handed
//! back a value of a type the other path does not name.

pub use AgenticSdk::Struct::Manifest::{All, Find, Manifest};

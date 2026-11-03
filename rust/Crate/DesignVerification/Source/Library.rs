//! `agentic.verify.design`: measure a built page and judge the measurements
//! against invariants the caller supplies.
//!
//! This crate holds no design. No selector, no pixel value, no colour: a caller
//! supplies the selectors and the invariants, and a test supplies a fixture. A port
//! that knew one site's rail width could not measure another site at all.
//!
//! It declares the port it needs - [`Struct::Port::Browse`], whose TypeScript
//! counterpart is the `Transport` seam - so a measurement can come from a tab, a
//! saved document or a fixture without this crate knowing which. The two verbs
//! split cleanly: `Measure` asks a page and is bounded, `Check` is pure and cannot
//! fail.

#![allow(non_snake_case)]
#![allow(non_upper_case_globals)]
#![allow(clippy::upper_case_acronyms)]

pub mod Fn;
pub mod Struct;

pub use crate::Struct::Error::{Code, Error};
pub use crate::Struct::Port::{
	Browse, Evidence, Failure, Invariant, Measurement, Outcome, Verdict, Verify, Want,
};
pub use crate::Struct::Verify::{Create, Verifier};

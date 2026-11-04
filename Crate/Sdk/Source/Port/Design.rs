//! `agentic.verify.design`, from `agentic-design-verification`: measure a page
//! and judge the measurements against invariants the caller supplies.
//!
//! [`Browse`] here is this crate's own port, not [`crate::Browse`]: a
//! measurement may come from a tab, a saved document or a fixture, and the
//! capability declares the two verbs it needs rather than the browser's three.
//! [`Create`] builds a [`Verifier`] from a [`Browse`] and a [`Pace`] the caller
//! supplies; [`Invariant`], [`Want`] and [`Evidence`] are what it is asked with,
//! and [`Verdict`] and [`Outcome`] are what it answers.
//!
//! The crate holds no design of its own: no selector, no pixel value, no colour,
//! and so nothing here names a site. A verifier that knew one rail's width could
//! not measure another site at all.

pub use AgenticDesignVerification::{
	Browse,
	Code,
	Create,
	Error,
	Evidence,
	Failure,
	Invariant,
	Measurement,
	Outcome,
	Verdict,
	Verifier,
	Verify,
	Want,
};

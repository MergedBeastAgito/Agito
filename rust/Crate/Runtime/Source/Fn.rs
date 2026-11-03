//! Functions: the combinators, and the one implementation that needs a feature.
//!
//! `Bound` and `Race` are the whole of the bounding machinery: a deadline is
//! not enforced by a global timer, it is enforced by racing the work against a
//! wait the caller's `Pace` supplies. That is why a capability can be bounded
//! without knowing which executor is running it.

pub mod BlockOn;
pub mod Bound;
pub mod Race;
#[cfg(feature = "tokio")]
pub mod Tokio;

//! `agentic.runtime`: execution policy and resource ownership.
//!
//! The runtime is a policy engine, not a scheduler of capabilities. It is
//! handed one generation and the capabilities registered against it, and
//! everything it answers afterwards is derived from that binding. It imports no
//! capability, so the manifest it reports is the manifest it enforces.
//!
//! # Two shapes, one of which this crate is
//!
//! The lease, the fence and the generation are internalities: in production
//! they live in a table changed by a reducer, and a Rust crate that kept them
//! would be a second authority however correct its fencing. So the crate is
//! split:
//!
//! - [`Struct::Runtime::Runner`] is a **thin effect runner**. It holds one
//!   generation, one registration list and one
//!   [`Struct::Authority::Authority`], and it keeps no lease state at all.
//!   `WithLease` asks the authority to admit an effect, runs the work, and
//!   drops the accounting guard.
//! - [`Struct::StandIn::StandIn`] is the **local stand-in** for the reducer:
//!   the same checks over a local table, so the port can be exercised with no
//!   database, no socket and no clock it did not choose. It is a test double.
//!   It is named as one, and a composition root that has a database substitutes
//!   a reducer adapter instead of constructing it.
//!
//! The destination is the guarantee and the admission is the optimisation:
//! [`Struct::Port::Destination::Present`] re-reads the record when an effect
//! lands and refuses a stale ticket there. A refusal is not an undo - an effect
//! that ran before it presented may have changed the world - so the destination
//! records it as [`Struct::Port::Ambiguous`] and quarantines the resource until
//! a resolver states whether the external action landed.
//!
//! # What the crate also is
//!
//! The workspace's execution core. It holds the shapes every other crate needs
//! in order to be bounded without naming an executor:
//!
//! - [`Struct::Call::FutureBox`] - one boxed future, the return type of every
//!   port method, so a capability never mentions a runtime;
//! - [`Struct::Call::Cancellation`] - the caller's signal, read as a flag and
//!   awaited as a future;
//! - [`Struct::Call::Clock`] and [`Struct::Call::Pace`] - the time a deadline
//!   is compared against and the wait between two reads, both supplied rather
//!   than reached for;
//! - [`Fn::Bound`] and [`Fn::Race`] - the combinator that turns "one operation"
//!   plus "one deadline" into "an answer or an expiry", written against those
//!   traits and nothing else.
//!
//! Nothing here is `async`-runtime-specific. The `tokio` feature adds a
//! `Pace` implementation and is off by default, so a capability crate that
//! depends on this one does not inherit an executor.

#![allow(non_snake_case)]
#![allow(non_upper_case_globals)]
#![allow(clippy::upper_case_acronyms)]

pub mod Fn;
pub mod Struct;

pub use crate::{
	Fn::{
		BlockOn::BlockOn,
		Bound::Bound,
		Race::{Either, Race},
		Script::{AssertParameters, Escape, FillDeclared, Unfilled},
	},
	Struct::{
		Authority::{Admitted, Authority},
		Call::{Call, Cancellation, Clock, Expired, FutureBox, Never, Pace, Signal, Wall},
		Error::{Code, Error},
		Port::{
			Admission,
			Ambiguous,
			Binding,
			Budget,
			Destination,
			Lease,
			Recorded,
			Registration,
			Request,
			Resolution,
			Runtime,
			Seal,
			StateClass,
			States,
			Ticket,
		},
		Runtime::{Create, Options, Runner},
		StandIn::StandIn,
	},
};

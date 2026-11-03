//! `agentic.runtime`: execution policy and resource ownership.
//!
//! The runtime is a policy engine, not a scheduler of capabilities. It is
//! handed one generation and the capabilities registered against it, and
//! everything it answers afterwards is derived from that binding. It imports no
//! capability, so the manifest it reports is the manifest it enforces.
//!
//! This crate is also the workspace's execution core. It holds the shapes every
//! other crate needs in order to be bounded without naming an executor:
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
	},
	Struct::{
		Call::{Call, Cancellation, Clock, Expired, FutureBox, Never, Pace, Signal, Wall},
		Error::{Code, Error},
		Ledger::{Ledger, Recorded},
		Port::{
			Admission,
			Binding,
			Budget,
			Lease,
			Registration,
			Request,
			Runtime,
			StateClass,
			States,
			Ticket,
		},
		Runtime::{Create, Options, Policy},
	},
};

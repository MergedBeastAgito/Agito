//! The seam between the runner and whoever actually owns the lease.
//!
//! `docs/Observability.md` puts every decision, counter, lease and record in a
//! table changed by a reducer, and puts only the effect itself in Rust. This
//! trait is that sentence as a type: the runner holds an [`Authority`] and asks
//! it, and it holds no lease state of its own to disagree with.
//!
//! Two implementations exist in the shape of things, and only one of them is in
//! this crate:
//!
//! - **A reducer adapter**, which is the authority in production: `Claim` is the
//!   `claim` reducer, `Renew` is `renew`, `Admit` reads the lease row the
//!   reducer already wrote, and `Present` is the `record` reducer. It is not
//!   here because it needs a SpacetimeDB client, which belongs to an adapter
//!   crate.
//! - **[`crate::Struct::StandIn::StandIn`]**, which is the local stand-in: the
//!   same six checks the reducer makes, over a local table, so the port can be
//!   exercised with no database at all. It is a test double. It is named as one,
//!   it is not wired into a production composition root, and the mirror test
//!   pins it to the reducer so it cannot quietly become a second authority.
//!
//! That is the difference the review asked to have made explicit: this crate
//! *runs* an effect and *presents* a ticket. It decides nothing. The thing that
//! decides is a value substituted at construction, and substituting a database
//! for the stand-in changes no line of the runner.

use std::time::Duration;

use crate::Struct::{
	Call::FutureBox,
	Error::Error,
	Port::{Budget, Destination, Lease, Ticket},
};

/// One admitted effect: the ticket to present, and the accounting the admission
/// opened.
///
/// The guard is opaque on purpose. The runner holds it for exactly as long as
/// the work runs and never reads it, so the count of unsettled local futures is
/// the authority's business and cannot be inspected, cached or reconstructed by
/// the runner. Dropping it settles the future whether the work finished, failed
/// or was abandoned - which is the only way an abandoned future gets counted
/// correctly, since Rust has no `finally`.
pub struct Admitted {
	/// The authority the effect presents when it lands.
	pub Ticket:Ticket,
	/// Dropping this settles the local future this admission opened.
	///
	/// Never read: the runner holds it for exactly as long as the work runs, and
	/// the count of unsettled futures is the authority's business.
	#[expect(dead_code, reason = "held only for its Drop; the runner must not read it")]
	pub(crate) Settle:Box<dyn Send>,
}

/// Everything the runner needs from whoever owns the leases.
///
/// `Claim`, `Renew` and `Release` are the verbs a reducer already has; `Admit` is
/// the one that is not a reducer, because it is the optimisation rather than the
/// record - it refuses doomed work early, charges the run's allowance and mints
/// the ticket. A reducer adapter implements it by reading the lease row and
/// returning the ticket that row implies.
pub trait Authority: Destination {
	/// The generation this authority records under.
	fn Generation(&self) -> u64;
	/// Claim a resource for one run, with that run's budget, or fail because one
	/// is held.
	fn Claim<'a>(
		&'a self,
		Resource:&'a str,
		Ttl:Duration,
		Budget:Option<Budget>,
	) -> FutureBox<'a, Result<Lease, Error>>;
	/// Extend a lease, only for the holder of the current fence, and only
	/// forwards.
	fn Renew<'a>(&'a self, Lease:Lease, Ttl:Duration) -> FutureBox<'a, Result<Lease, Error>>;
	/// Give the authority back. Never fails, and is safe twice.
	fn Release<'a>(&'a self, Lease:Lease) -> FutureBox<'a, ()>;
	/// The admission step: refuse a lease whose authority ended, charge the run's
	/// allowance, count the local future, and mint the ticket it will present.
	fn Admit<'a>(&'a self, Lease:Lease) -> FutureBox<'a, Result<Admitted, Error>>;
}

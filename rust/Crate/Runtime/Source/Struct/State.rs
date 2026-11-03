//! The one table the local stand-in keeps.
//!
//! Two verbs mutate it: the authority issues, renews and releases leases, and
//! the destination accepts or refuses a record. Both take the same lock over the
//! same maps, which is what makes "the manifest it reports is the manifest it
//! enforces" a property of the bytes rather than a convention - there is no
//! second copy of a fence to drift, and no window in which the lease engine and
//! the recorder disagree about which fence is current.
//!
//! It is crate-private on purpose. The table is an implementation of one
//! authority, not a shape a consumer may reach into: a consumer holds a
//! [`crate::Struct::StandIn::StandIn`] and presents tickets to it, which is the
//! only way to change what is recorded. A reducer adapter keeps no such table
//! here at all - its table is the database's, and that is the whole difference
//! between the stand-in and the authority.

use std::{
	collections::HashMap,
	sync::{Mutex, MutexGuard},
	time::Instant,
};

use crate::Struct::{
	Port::{Ambiguous, Budget, Recorded, Seal},
};

/// What the authority knows about one live grant.
pub(crate) struct Run {
	/// The id the lease was issued with; carried in a ticket so a receipt names
	/// the grant it came from.
	pub(crate) Id:String,
	/// The holder's identity. This is the reducer's `lease.holder` column: the
	/// only caller a fenced write is accepted from, rendered as something a
	/// caller cannot guess instead of a name a caller could.
	pub(crate) Seal:Seal,
	/// The generation the lease was granted under. A lease never adopts a later
	/// one.
	pub(crate) Generation:u64,
	/// The fence the resource is at while this record is the live one.
	pub(crate) Fence:u64,
	/// The record's own expiry. A presented lease's copy is never trusted.
	pub(crate) ExpiresAt:Instant,
	/// This run's allowance, owned by the authority once the claim is admitted.
	pub(crate) Budget:Option<Budget>,
	/// Local futures admitted under this record that have not settled yet.
	///
	/// This is an accounting count, not a fence. It says a future this process
	/// admitted is still running. It does not say the worker behind that future
	/// still has authority, and it says nothing at all about work that has
	/// already left the process: dropping a future decrements this, and
	/// dropping a future does not stop a browser-side operation.
	pub(crate) Unsettled:u64,
	/// Set when the holder released. The row survives, because the row is the
	/// only place that still knows who held this epoch - and a late effect from
	/// a released epoch has to be attributable to somebody.
	pub(crate) Released:bool,
	/// Set when an effect from this epoch was refused after it ran. A
	/// quarantined resource is not reassigned until a resolver says whether the
	/// effect landed.
	pub(crate) Quarantined:bool,
}

/// The table every verb mutates, behind one lock.
#[derive(Default)]
pub(crate) struct Inner {
	/// What is live now, by resource name.
	pub(crate) Runs:HashMap<String, Run>,
	/// The highest fence ever issued, by resource name. Never reset, so a
	/// released row cannot make an old number current again.
	pub(crate) Fences:HashMap<String, u64>,
	/// Every effect the destination accepted, in the order it accepted them.
	///
	/// A refused effect leaves nothing here: the record is the effect's
	/// existence in the state, so an effect the destination refused did not
	/// happen as far as anything reading this table is concerned. What a
	/// refusal leaves is in `Ambiguities`, because the world is not the state
	/// and a write that landed is not retracted by a record that did not.
	pub(crate) Records:Vec<Recorded>,
	/// Every effect that was admitted and refused, in the order it was refused.
	pub(crate) Ambiguities:Vec<Ambiguous>,
}

/// A lock whose poisoning is not a failure: a panicked caller must not make the
/// whole runtime unusable.
pub(crate) fn Lock(Guard:&Mutex<Inner>) -> MutexGuard<'_, Inner> {
	Guard.lock().unwrap_or_else(|Poison| Poison.into_inner())
}

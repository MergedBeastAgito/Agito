//! The one table the engine writes and the destination reads.
//!
//! Two verbs mutate it: the engine issues, renews and releases leases, and the
//! destination accepts or refuses a record. Both take the same lock over the
//! same maps, which is what makes "the manifest it reports is the manifest it
//! enforces" a property of the bytes rather than a convention - there is no
//! second copy of a fence to drift, and no window in which the lease engine and
//! the recorder disagree about which fence is current.
//!
//! It is crate-private on purpose. The table is an implementation of one
//! authority, not a shape a consumer may reach into: a consumer holds a
//! [`crate::Struct::Ledger::Ledger`] and presents tickets to it, which is the
//! only way to change what is recorded.

use std::{
	collections::HashMap,
	sync::{Mutex, MutexGuard},
	time::Instant,
};

use crate::Struct::{
	Ledger::Recorded,
	Port::Budget,
};

/// What the runtime knows about one live grant.
pub(crate) struct Run {
	/// The id the lease was issued with; a presented lease must carry it.
	pub(crate) Id:String,
	/// The fence the resource is at while this record is the live one.
	pub(crate) Fence:u64,
	/// The record's own expiry. A presented lease's copy is never trusted.
	pub(crate) ExpiresAt:Instant,
	/// This run's allowance, owned by the runtime once the claim is admitted.
	pub(crate) Budget:Option<Budget>,
	/// Local futures admitted under this record that have not settled yet.
	///
	/// This is an accounting count, not a fence. It says a future this process
	/// admitted is still running. It does not say the worker behind that future
	/// still has authority, and it says nothing at all about work that has
	/// already left the process: dropping a future decrements this, and
	/// dropping a future does not stop a browser-side operation.
	pub(crate) Unsettled:u64,
	/// Set when the holder released while a future was still unsettled.
	pub(crate) Released:bool,
}

/// The table every verb mutates, behind one lock.
#[derive(Default)]
pub(crate) struct Inner {
	/// What is live now, by resource name.
	pub(crate) Runs:HashMap<String, Run>,
	/// The highest fence ever issued, by resource name. Never reset.
	pub(crate) Fences:HashMap<String, u64>,
	/// Every effect the destination accepted, in the order it accepted them.
	///
	/// A refused effect leaves nothing here: the record is the effect's
	/// existence in the state, so an effect the destination refused did not
	/// happen as far as anything reading this table is concerned.
	pub(crate) Records:Vec<Recorded>,
}

/// A lock whose poisoning is not a failure: a panicked caller must not make the
/// whole runtime unusable.
pub(crate) fn Lock(Guard:&Mutex<Inner>) -> MutexGuard<'_, Inner> {
	Guard.lock().unwrap_or_else(|Poison| Poison.into_inner())
}

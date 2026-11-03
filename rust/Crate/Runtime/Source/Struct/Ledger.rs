//! The destination: where an effect's record lands, and the only place that
//! decides whether it may.
//!
//! `docs/Observability.md` splits the world in two: effects are external and
//! live in Rust, internalities are decisions and records and live in a table
//! changed by a reducer. This module is the local stand-in for that reducer. It
//! is not a second engine - it reads the same table the lease engine writes, so
//! the fence it compares against is the fence the engine issued, and there is no
//! window in which two copies disagree.
//!
//! What it is not: a wrapper around [`crate::Struct::Runtime::Policy::WithLease`].
//! `WithLease` admits an effect when the work starts; this refuses a record when
//! the work ends. Between those two moments the lease can expire, be released,
//! or be taken over by a newer fence, and the admission that already happened
//! cannot say anything about any of it. Only the party the record is presented
//! to can. That is why the check lives here, and why moving it back into the
//! caller is a regression a test catches rather than a refactor.
//!
//! The honest limit: a local check cannot un-send bytes that already left the
//! process. What the destination guarantees is that a stale effect is never
//! recorded, so the state never claims an effect that had no authority behind
//! it. An effect that records before it writes therefore cannot write at all
//! after its authority ended; an effect that writes first is detectable as an
//! orphan rather than counted as a success.

use std::{
	sync::{
		Arc,
		Mutex,
	},
	time::Instant,
};

use crate::Struct::{
	Call::Clock,
	Error::Error,
	Port::Ticket,
	State::{self, Inner},
};

/// What the destination kept after accepting an effect.
///
/// The receipt is the effect's existence in the state: the fence and generation
/// it names are the authority it was recorded under, read from the table at the
/// instant it landed rather than copied from the ticket.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Recorded {
	/// The effect's own name for what it did.
	pub Name:String,
	/// The resource it happened under.
	pub Resource:String,
	/// The fence it was recorded at.
	pub Fence:u64,
	/// The generation it was recorded under.
	pub Generation:u64,
	/// The instant the destination accepted it, on the runtime's clock.
	pub At:Instant,
}

/// The place an effect is recorded: the local stand-in for a reducer.
///
/// Cloning one is cloning a handle to the same authority, so a destination
/// handed to another thread is still the destination the lease engine is
/// fencing against.
#[derive(Clone)]
pub struct Ledger {
	Generation:u64,
	Clock:Arc<dyn Clock>,
	Inner:Arc<Mutex<Inner>>,
}

impl Ledger {
	/// The destination for one runtime's table.
	///
	/// Crate-private: a destination is obtained from
	/// [`crate::Struct::Runtime::Policy::Ledger`], so there is exactly one
	/// authority per runtime and no way to build a ledger that reads a table
	/// nobody is fencing.
	pub(crate) fn At(Generation:u64, Clock:Arc<dyn Clock>, Inner:Arc<Mutex<Inner>>) -> Self {
		Self { Generation, Clock, Inner }
	}

	/// Record one effect at the place it lands.
	///
	/// Six checks, in the order that makes the reason the true one:
	///
	/// 1. **Generation** - the ticket was minted by a runtime bound to another
	///    generation, so its fence numbers are not this authority's numbers.
	/// 2. **No record** - nothing is held under that resource any more.
	/// 3. **Fence mismatch** - a newer claim superseded the authority the effect
	///    was admitted under; `lease-held`.
	/// 4. **Id mismatch** - the fence is current but the ticket was not the one
	///    issued for it, which happens when a ticket is hand-built or carried
	///    between runtimes.
	/// 5. **Released** - the holder gave the authority back.
	/// 6. **Expired** - the fence is current, the ticket is the issued one, and
	///    authority ended anyway. This is the counterexample in
	///    `docs/Review.md`: the effect was admitted while the lease was live,
	///    the lease expired under it, and the write arrives afterwards. Nothing
	///    in the caller can refuse it at that point; this can.
	///
	/// Every refusal is a value. A panic would turn a lost lease into a lost
	/// process, and a lost lease is the expected outcome of a slow worker.
	pub fn Record(&self, Ticket:Ticket, Name:impl Into<String>) -> Result<Recorded, Error> {
		let Now = self.Clock.Now();
		let mut Inner = State::Lock(&self.Inner);
		if Ticket.Generation != self.Generation {
			return Err(Error::Refused(format!(
				"the effect was admitted under generation {}, and this destination records generation {}",
				Ticket.Generation, self.Generation
			)));
		}
		let Some(Run) = Inner.Runs.get(&Ticket.Resource) else {
			return Err(Error::Refused(format!("no lease is live for {}", Ticket.Resource)));
		};
		if Run.Fence != Ticket.Fence {
			return Err(Error::Held(format!(
				"{} is held under fence {}, not fence {}",
				Ticket.Resource, Run.Fence, Ticket.Fence
			)));
		}
		if Run.Id != Ticket.LeaseId {
			return Err(Error::Refused(format!(
				"the ticket presented for fence {} of {} was not the one issued",
				Run.Fence, Ticket.Resource
			)));
		}
		if Run.Released {
			return Err(Error::Refused(format!("the lease on {} was released", Ticket.Resource)));
		}
		if Now >= Run.ExpiresAt {
			return Err(Error::Refused(format!(
				"the effect outlived its authority: the lease on {} ended at {:?}, the ticket was minted for {:?}, and the clock reads {:?}",
				Ticket.Resource, Run.ExpiresAt, Ticket.ExpiresAt, Now
			)));
		}
		let Fence = Run.Fence;
		let Receipt = Recorded {
			Name:Name.into(),
			Resource:Ticket.Resource.clone(),
			Fence,
			Generation:self.Generation,
			At:Now,
		};
		Inner.Records.push(Receipt.clone());
		Ok(Receipt)
	}

	/// Every effect this destination accepted, in the order it accepted them.
	///
	/// A refused effect is absent: it was never recorded, so as far as anything
	/// reading this table is concerned it did not happen.
	pub fn Records(&self) -> Vec<Recorded> { State::Lock(&self.Inner).Records.clone() }

	/// The generation this destination records under.
	pub const fn Generation(&self) -> u64 { self.Generation }
}

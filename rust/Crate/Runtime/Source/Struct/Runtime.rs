//! The implementation of the port in `Port.rs`: admit a run, lease a resource
//! with a fence, bound it with a budget, and clean up without throwing.
//!
//! Everything here is process-local policy. There is no transport, no
//! capability and no import from another crate: the runtime is handed a
//! generation and a registration list and answers from that binding alone,
//! which is what makes "the manifest it reports is the manifest it enforces"
//! checkable rather than asserted.
//!
//! Four mechanisms carry the rules the port states:
//!
//! 1. **A record per resource, and a fence counter that outlives it.** The
//!    record is what is live now; the counter is what was ever issued. Deleting
//!    the record on release therefore cannot make an old number current again.
//! 2. **One locked step per verb.** Claim, renew, release and the admission
//!    step of an effect all mutate the same table, and each does so under one
//!    lock. A check that is not atomic with the mutation it authorises is the
//!    race the fence exists to close: two claims that both read "free" both
//!    win. The effect itself is the caller's and is not under the lock - holding
//!    it across an effect would deadlock one that renews its own lease, and a
//!    slow effect would stall every other resource.
//! 3. **An effect carries the authority it was admitted under.** `WithLease`
//!    mints a `Ticket` - lease id, fence, generation - and hands it to the work.
//!    The admission is an optimisation: it refuses doomed work early and charges
//!    the budget. The guarantee is at the destination
//!    ([`crate::Struct::Ledger`]), which re-reads the table when the effect
//!    lands and refuses a ticket whose fence, generation, lease or expiry is no
//!    longer current. A check in this file cannot see the instant an effect
//!    writes; a check where it lands can.
//! 4. **Unsettled local futures are counted, and a takeover waits for them.**
//!    This is accounting, not fencing. The count says a future this process
//!    admitted has not settled; it does not say the worker behind it still has
//!    authority, and it says nothing about work that has already left the
//!    process. What it buys is that a local takeover does not begin while an
//!    older local future is still running - a scheduling choice, honestly
//!    labelled. The count is decremented by a guard's `Drop`, so an abandoned
//!    future settles exactly as a completed one does.

use std::{
	collections::BTreeMap,
	sync::{
		Arc,
		Mutex,
		atomic::{AtomicU64, Ordering},
	},
	time::{Duration, Instant},
};

use crate::Struct::{
	Call::{Clock, FutureBox, Wall},
	Error::Error,
	Ledger::Ledger,
	Port::{Admission, Binding, Budget, Lease, Registration, Request, Runtime, StateClass, Ticket},
	State::{self, Inner, Run},
};

/// What the composition root hands the engine.
pub struct Options {
	/// The generation the composition root bound this runtime to.
	pub Generation:u64,
	/// What it may admit, by name, with the state class each declares.
	pub Capabilities:Vec<Registration>,
	/// The clock leases and budgets are read on.
	pub Clock:Arc<dyn Clock>,
}

impl Options {
	/// A runtime with no capabilities and the wall clock.
	pub fn New(Generation:u64) -> Self { Self { Generation, Capabilities:Vec::new(), Clock:Arc::new(Wall) } }

	/// The same options, with one more registration.
	pub fn With(mut self, Name:impl Into<String>, State:StateClass) -> Self {
		self.Capabilities.push(Registration { Name:Name.into(), State });
		self
	}

	/// The same options, on another clock.
	pub fn On(mut self, Clock:Arc<dyn Clock>) -> Self {
		self.Clock = Clock;
		self
	}
}

/// Distinguishes one runtime's leases from another's in the same process.
///
/// Lease ids are built from this token, so a lease issued by one runtime is not
/// accepted by a second one that happens to have issued the same fence number
/// for the same resource name. Two runtimes are two authorities, and a fence
/// number is only meaningful inside the authority that counted it.
static Instances:AtomicU64 = AtomicU64::new(0);

/// Accounts for one admitted local future, however it ends.
///
/// A record released while it had a future unsettled survives until that future
/// settles, and is deleted then. `Drop` is what makes that true for an abandoned
/// future as well as a completed one: there is no `finally` in Rust, and a
/// cancellation that leaked the count would hold a resource until the process
/// ended.
///
/// It counts local futures and nothing else. A future that settles here may
/// still have left work running somewhere this process cannot see, which is why
/// the destination - not this count - is what decides whether an effect may be
/// recorded.
struct Settle {
	Inner:Arc<Mutex<Inner>>,
	Resource:String,
	Fence:u64,
}

impl Drop for Settle {
	fn drop(&mut self) {
		let mut Inner = State::Lock(&self.Inner);
		let Remove = match Inner.Runs.get_mut(&self.Resource) {
			Some(Run) if Run.Fence == self.Fence => {
				Run.Unsettled = Run.Unsettled.saturating_sub(1);
				Run.Released && Run.Unsettled == 0
			},
			_ => false,
		};
		if Remove {
			Inner.Runs.remove(&self.Resource);
		}
	}
}

/// The engine: one generation, one registration list, one clock, one authority.
pub struct Policy {
	Generation:u64,
	Capabilities:BTreeMap<String, Registration>,
	Clock:Arc<dyn Clock>,
	Token:String,
	Inner:Arc<Mutex<Inner>>,
}

/// Bind a runtime to one generation and one registration list.
///
/// A duplicate registration is refused here rather than resolved, because two
/// declarations of one name is a composition root disagreeing with itself and
/// picking one silently is how a pre-warm harness prepares the wrong thing.
pub fn Create(Options:Options) -> Result<Policy, String> {
	let mut Capabilities = BTreeMap::new();
	for Registration in Options.Capabilities {
		if Capabilities.contains_key(&Registration.Name) {
			return Err(format!("{} is registered twice", Registration.Name));
		}
		Capabilities.insert(Registration.Name.clone(), Registration);
	}
	let Instance = Instances.fetch_add(1, Ordering::SeqCst) + 1;
	Ok(Policy {
		Generation:Options.Generation,
		Capabilities,
		Clock:Options.Clock,
		Token:format!("r{Instance}"),
		Inner:Arc::new(Mutex::new(Inner::default())),
	})
}

impl Policy {
	/// The admission predicate, evaluated where an effect starts.
	///
	/// Five checks, ordered so the reason is the true one:
	///
	/// 1. **No record** - nothing is held under that name any more.
	/// 2. **Fence mismatch** - the caller's authority was superseded.
	///    `lease-held`: somebody else holds it, and the caller's copy is
	///    history.
	/// 3. **Id mismatch** - the fence is current but the lease was not the one
	///    issued for it, which happens when a lease is carried between two
	///    runtimes or hand-built.
	/// 4. **Released** - the holder gave the authority back.
	/// 5. **Expired** - the fence matches, the lease is the issued one, and
	///    authority ended anyway.
	///
	/// This is the caller's check, and it answers a question about *now*: may
	/// this work start? It is deliberately not the destination's check, which
	/// answers a different question - may this effect be recorded? - at a later
	/// moment. Sharing one function between the two would reintroduce exactly
	/// the defect the destination exists to close, so
	/// [`crate::Struct::Ledger::Ledger::Record`] has its own.
	fn Admit<'a>(Inner:&'a mut Inner, Lease:&Lease, Now:Instant) -> Result<&'a mut Run, Error> {
		let Some(Run) = Inner.Runs.get_mut(&Lease.Resource) else {
			return Err(Error::Refused(format!("no lease is live for {}", Lease.Resource)));
		};
		if Run.Fence != Lease.Fence {
			return Err(Error::Held(format!(
				"{} is held under fence {}, not fence {}",
				Lease.Resource, Run.Fence, Lease.Fence
			)));
		}
		if Run.Id != Lease.Id {
			return Err(Error::Refused(format!(
				"the lease presented for fence {} of {} was not the one issued",
				Run.Fence, Lease.Resource
			)));
		}
		if Run.Released {
			return Err(Error::Refused(format!("the lease on {} was released", Lease.Resource)));
		}
		if Now >= Run.ExpiresAt {
			return Err(Error::Refused(format!(
				"the lease on {} expired at {:?}, and the clock reads {:?}",
				Lease.Resource, Run.ExpiresAt, Now
			)));
		}
		Ok(Run)
	}

	/// Spend one operation of the run's allowance.
	///
	/// Charged before the effect, not after: an effect that fails was still
	/// attempted, and a count that only moved on success would let a run that
	/// fails in a loop spend nothing. The charge happens under the same lock as
	/// the fence check, so two effects cannot both read the last operation.
	fn Charge(Run:&mut Run, Now:Instant) -> Result<(), Error> {
		let Some(Budget) = Run.Budget.as_mut() else {
			return Ok(());
		};
		if let Some(Deadline) = Budget.Deadline
			&& Now >= Deadline
		{
			return Err(Error::Spent(format!(
				"the run's deadline passed at {Deadline:?}, and the clock reads {Now:?}"
			)));
		}
		if Budget.Operations == 0 {
			return Err(Error::Spent("the run has spent every operation it was granted"));
		}
		Budget.Operations -= 1;
		Ok(())
	}

	/// The destination this runtime records effects at.
	///
	/// One authority, two verbs: the engine issues, renews and releases leases,
	/// and the ledger accepts or refuses records. Both read the same table, so a
	/// record cannot be accepted under a fence the engine has already moved past
	/// - not because the two agree, but because there is only one number.
	///
	/// Hand this to whatever performs an effect. An effect that never presents
	/// its ticket leaves no record, and an unrecorded effect is invisible to the
	/// state: the runtime cannot make the presentation happen, but it can make
	/// the absence observable.
	pub fn Ledger(&self) -> Ledger { Ledger::At(self.Generation, self.Clock.clone(), self.Inner.clone()) }
}

impl Runtime for Policy {
	fn Admit<'a>(&'a self, Request:Request) -> FutureBox<'a, Admission> {
		Box::pin(async move {
			if Request.Generation != self.Generation {
				return Admission::Refused(format!(
					"admission-refused: generation {} is not bound; this runtime enforces generation {}",
					Request.Generation, self.Generation
				));
			}
			let Some(Registered) = self.Capabilities.get(&Request.Capability) else {
				return Admission::Refused(format!(
					"admission-refused: {} is not registered with this runtime",
					Request.Capability
				));
			};
			if Registered.State != Request.State {
				return Admission::Refused(format!(
					"admission-refused: {} declares {}, not {}",
					Registered.Name,
					Registered.State.Name(),
					Request.State.Name()
				));
			}
			Admission::Admitted
		})
	}

	fn Claim<'a>(
		&'a self,
		Resource:&'a str,
		Ttl:Duration,
		Budget:Option<Budget>,
	) -> FutureBox<'a, Result<Lease, Error>> {
		Box::pin(async move {
			if Resource.is_empty() {
				return Err(Error::Refused("a claim names the resource it wants"));
			}
			if Ttl.is_zero() {
				return Err(Error::Refused(format!(
					"a lease ttl must be a positive, finite number of milliseconds; got {}",
					Ttl.as_millis()
				)));
			}
			let Now = self.Clock.Now();
			let ExpiresAt = Now.checked_add(Ttl).ok_or_else(|| {
				Error::Refused(format!("a lease ttl of {}ms has no instant to expire at", Ttl.as_millis()))
			})?;
			let mut Inner = State::Lock(&self.Inner);
			if let Some(Current) = Inner.Runs.get(Resource) {
				if Current.Unsettled > 0 {
					return Err(Error::Held(format!(
						"{Resource} has an unsettled local future under fence {}; a takeover waits for it to settle",
						Current.Fence
					)));
				}
				if Now < Current.ExpiresAt {
					return Err(Error::Held(format!(
						"{Resource} is held under fence {} until {:?}",
						Current.Fence, Current.ExpiresAt
					)));
				}
			}
			let Fence = Inner.Fences.get(Resource).copied().unwrap_or(0) + 1;
			Inner.Fences.insert(Resource.to_string(), Fence);
			let Lease = Lease {
				Id:format!("{}#{}#{}", self.Token, Resource, Fence),
				Resource:Resource.to_string(),
				Fence,
				ExpiresAt,
			};
			Inner.Runs.insert(
				Resource.to_string(),
				Run {
					Id:Lease.Id.clone(),
					Fence,
					ExpiresAt,
					Budget,
					Unsettled:0,
					Released:false,
				},
			);
			Ok(Lease)
		})
	}

	fn Renew<'a>(&'a self, Lease:Lease, Ttl:Duration) -> FutureBox<'a, Result<Lease, Error>> {
		Box::pin(async move {
			if Ttl.is_zero() {
				return Err(Error::Refused(format!(
					"a renewal ttl must be a positive, finite number of milliseconds; got {}",
					Ttl.as_millis()
				)));
			}
			let Now = self.Clock.Now();
			let mut Inner = State::Lock(&self.Inner);
			let Run = Self::Admit(&mut Inner, &Lease, Now)?;
			let ExpiresAt = Now.checked_add(Ttl).ok_or_else(|| {
				Error::Refused(format!("a renewal ttl of {}ms has no instant to expire at", Ttl.as_millis()))
			})?;
			if ExpiresAt <= Run.ExpiresAt {
				return Err(Error::Refused(format!(
					"a renewal must extend the lease: {ExpiresAt:?} is not later than {:?}. Release it instead.",
					Run.ExpiresAt
				)));
			}
			Run.ExpiresAt = ExpiresAt;
			Ok(Lease { ExpiresAt, ..Lease })
		})
	}

	fn WithLease<'a, T, Write>(&'a self, Lease:Lease, Write:Write) -> FutureBox<'a, Result<T, Error>>
	where
		T: Send + 'a,
		Write: FnOnce(Ticket) -> FutureBox<'a, T> + Send + 'a, {
		Box::pin(async move {
			// The admission step: check, charge, count, and mint the authority
			// the effect will carry. All four happen under one lock, so the
			// fence in the ticket is the fence the charge was taken under.
			let Ticket = {
				let Now = self.Clock.Now();
				let mut Inner = State::Lock(&self.Inner);
				let Run = Self::Admit(&mut Inner, &Lease, Now)?;
				Self::Charge(Run, Now)?;
				Run.Unsettled += 1;
				Ticket {
					Generation:self.Generation,
					Resource:Lease.Resource.clone(),
					LeaseId:Lease.Id.clone(),
					Fence:Lease.Fence,
					// The record's expiry, not the copy the caller presented: a
					// lease renewed after it was handed out is still the same
					// grant, and the caller's copy is the older number.
					ExpiresAt:Run.ExpiresAt,
				}
			};
			let Guard = Settle { Inner:self.Inner.clone(), Resource:Lease.Resource.clone(), Fence:Lease.Fence };
			let Value = Write(Ticket).await;
			drop(Guard);
			Ok(Value)
		})
	}

	fn Release<'a>(&'a self, Lease:Lease) -> FutureBox<'a, ()> {
		Box::pin(async move {
			let mut Inner = State::Lock(&self.Inner);
			// A superseded lease is not this caller's to end: the resource now
			// belongs to the fence that took it over, and releasing here would
			// hand a stranger's authority back.
			let Remove = match Inner.Runs.get_mut(&Lease.Resource) {
				Some(Run) if Run.Fence == Lease.Fence => {
					Run.Released = true;
					Run.Unsettled == 0
				},
				_ => false,
			};
			if Remove {
				Inner.Runs.remove(&Lease.Resource);
			}
		})
	}

	fn Now(&self) -> Instant { self.Clock.Now() }

	fn Reflect(&self) -> Binding {
		Binding {
			Generation:self.Generation,
			Capabilities:self.Capabilities.values().cloned().collect(),
		}
	}

	fn Clock(&self) -> &dyn Clock { self.Clock.as_ref() }
}

//! The implementation of the port in `Port.rs`: admit a run, lease a resource
//! with a fence, bound it with a budget, and clean up without throwing.
//!
//! Everything here is process-local policy. There is no transport, no
//! capability and no import from another crate: the runtime is handed a
//! generation and a registration list and answers from that binding alone,
//! which is what makes "the manifest it reports is the manifest it enforces"
//! checkable rather than asserted.
//!
//! Three mechanisms carry the rules the port states:
//!
//! 1. **A record per resource, and a fence counter that outlives it.** The
//!    record is what is live now; the counter is what was ever issued. Deleting
//!    the record on release therefore cannot make an old number current again.
//! 2. **One locked step per verb.** Claim, renew, release and the admission
//!    step of a write all mutate the same two maps, and each does so under one
//!    lock. A check that is not atomic with the mutation it authorises is the
//!    race the fence exists to close: two claims that both read "free" both
//!    win. The write itself is the caller's and is not under the lock - holding
//!    it across the write would deadlock a write that renews its own lease, and
//!    a slow write would stall every other resource.
//! 3. **Effects in flight are counted, and a takeover waits for them.** A fence
//!    proves ordering, not liveness: an expired worker can arrive before any
//!    newer token reaches the resource. Counting admitted-but-unsettled effects
//!    means a claim cannot take a resource out from under an effect that is
//!    still running, and a release cannot free one either. The count is
//!    decremented by a guard's `Drop`, so an abandoned future settles exactly
//!    as a completed one does.

use std::{
	collections::{BTreeMap, HashMap},
	sync::{
		Arc,
		Mutex,
		MutexGuard,
		atomic::{AtomicU64, Ordering},
	},
	time::{Duration, Instant},
};

use crate::Struct::{
	Call::{Clock, FutureBox, Wall},
	Error::Error,
	Port::{Admission, Binding, Budget, Lease, Registration, Request, Runtime, StateClass},
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

/// What the runtime knows about one live grant.
struct Run {
	/// The id the lease was issued with; a presented lease must carry it.
	Id:String,
	/// The fence the resource is at while this record is the live one.
	Fence:u64,
	/// The record's own expiry. A presented lease's copy is never trusted.
	ExpiresAt:Instant,
	/// This run's allowance, owned by the runtime once the claim is admitted.
	Budget:Option<Budget>,
	/// Effects admitted under this record that have not settled yet.
	InFlight:u64,
	/// Set when the holder released while an effect was still in flight.
	Released:bool,
}

/// The two maps every verb mutates, behind one lock.
#[derive(Default)]
struct Inner {
	/// What is live now, by resource name.
	Runs:HashMap<String, Run>,
	/// The highest fence ever issued, by resource name. Never reset.
	Fences:HashMap<String, u64>,
}

/// Accounts for one admitted effect, however it ends.
///
/// A record released while it had an effect in flight survives until that
/// effect settles, and is deleted then. `Drop` is what makes that true for an
/// abandoned future as well as a completed one: there is no `finally` in Rust,
/// and a cancellation that leaked the count would hold a resource until the
/// process ended.
struct Settle<'a> {
	Inner:&'a Mutex<Inner>,
	Resource:String,
	Fence:u64,
}

impl Drop for Settle<'_> {
	fn drop(&mut self) {
		let mut Inner = Policy::Lock(self.Inner);
		let Remove = match Inner.Runs.get_mut(&self.Resource) {
			Some(Run) if Run.Fence == self.Fence => {
				Run.InFlight = Run.InFlight.saturating_sub(1);
				Run.Released && Run.InFlight == 0
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
	Inner:Mutex<Inner>,
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
		Inner:Mutex::new(Inner::default()),
	})
}

impl Policy {
	/// A lock whose poisoning is not a failure: a panicked caller must not make
	/// the whole runtime unusable.
	fn Lock(Guard:&Mutex<Inner>) -> MutexGuard<'_, Inner> { Guard.lock().unwrap_or_else(|Poison| Poison.into_inner()) }

	/// The admission predicate, evaluated at the effect.
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
	/// 4. **Released** - the holder gave the authority back while an effect was
	///    still running.
	/// 5. **Expired** - the fence matches, the lease is the issued one, and
	///    authority ended anyway. This is the check that makes a worker's late
	///    write fail rather than land.
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
			let mut Inner = Self::Lock(&self.Inner);
			if let Some(Current) = Inner.Runs.get(Resource) {
				if Current.InFlight > 0 {
					return Err(Error::Held(format!(
						"{Resource} has an effect in flight under fence {}; a takeover waits for it to settle",
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
				Run { Id:Lease.Id.clone(), Fence, ExpiresAt, Budget, InFlight:0, Released:false },
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
			let mut Inner = Self::Lock(&self.Inner);
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
		Write: FnOnce() -> FutureBox<'a, T> + Send + 'a, {
		Box::pin(async move {
			{
				let Now = self.Clock.Now();
				let mut Inner = Self::Lock(&self.Inner);
				let Run = Self::Admit(&mut Inner, &Lease, Now)?;
				Self::Charge(Run, Now)?;
				Run.InFlight += 1;
			}
			let Guard = Settle { Inner:&self.Inner, Resource:Lease.Resource.clone(), Fence:Lease.Fence };
			let Value = Write().await;
			drop(Guard);
			Ok(Value)
		})
	}

	fn Release<'a>(&'a self, Lease:Lease) -> FutureBox<'a, ()> {
		Box::pin(async move {
			let mut Inner = Self::Lock(&self.Inner);
			// A superseded lease is not this caller's to end: the resource now
			// belongs to the fence that took it over, and releasing here would
			// hand a stranger's authority back.
			let Remove = match Inner.Runs.get_mut(&Lease.Resource) {
				Some(Run) if Run.Fence == Lease.Fence => {
					Run.Released = true;
					Run.InFlight == 0
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

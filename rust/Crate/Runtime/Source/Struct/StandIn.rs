//! The local stand-in for the reducer: the same checks, over a local table.
//!
//! This module is a **test double**. It exists so the port can be exercised with
//! no database, no socket and no clock it did not choose; it is not the authority
//! in production, and the shape of the crate says so: the runner holds an
//! `Authority` and knows nothing else, so a composition root that wants the real
//! authority substitutes a reducer adapter and this type is never constructed.
//!
//! What it is not: a wrapper around [`crate::Runtime::WithLease`]. `WithLease`
//! admits an effect when the work starts; this refuses a record when the work
//! ends. Between those two moments the lease can expire, be released, or be taken
//! over by a newer fence, and the admission that already happened cannot say
//! anything about any of it. Only the party the record is presented to can. That
//! is why the check lives here, and why moving it back into the caller is a
//! regression a test catches rather than a refactor.
//!
//! # The decision, and its mirror
//!
//! `packages/spacetime-catalog/src/Reducers.ts` makes the same decision in
//! `record`, over the same columns, and `Test/Destination.test.ts` there drives
//! the same transcript to the same refusal. The correspondence is exact, column
//! for column:
//!
//! | reducer                                   | stand-in                                  |
//! | ----------------------------------------- | ----------------------------------------- |
//! | `Latest(ctx, generation) !== null`        | `Ticket.Generation == self.Generation`     |
//! | `ctx.find("lease", resource) !== null`    | `Inner.Runs.get(resource)`                 |
//! | `lease["fence"] === args.fence`           | `Run.Fence == Ticket.Fence`                |
//! | `lease["holder"] === ctx.sender`          | `Run.Seal == Ticket.Seal`                  |
//! | `lease["state"] !== "released"`           | `!Run.Released`                            |
//! | `lease["state"] !== "quarantined"`        | `!Run.Quarantined`                         |
//! | `IsLive(lease, ctx.now)`                  | `Now < Run.ExpiresAt`                      |
//! | refusal after identity → ambiguity + quarantine | the same, in `Quarantine`            |
//!
//! # The handover rule
//!
//! An effect that was admitted under an epoch and is refused when it lands is
//! recorded as [`Ambiguous`] against the resource, and the resource is
//! quarantined: the write is not counted as a success, and no new holder is
//! assigned the resource until a resolver states whether the external action
//! landed or was absent. Rejecting an effect's result does not undo its effect,
//! so the refusal is never allowed to be the only trace of it.
//!
//! # The honest limit, and where the rule puts the rest
//!
//! A refusal is *ambiguous* only when the ticket's identity matches the current
//! epoch - the authority can then say "this effect was admitted and its authority
//! has ended". A ticket whose fence has already been superseded names an epoch
//! the row no longer describes, so it is refused without an ambiguity entry. That
//! is not a gap in the rule, it is where the rule puts the decision: the claim
//! that superseded the epoch is the accounting point for it, and the reconciler
//! that granted the new fence is the party that knew the old one had ended. This
//! stand-in refuses such a takeover while a local future is unsettled, which is
//! the same accounting done eagerly instead of on inspection.
//!
//! A local check also cannot un-send bytes that already left the process. What
//! the destination guarantees is that a stale effect is never *recorded* as a
//! success, and that one which may have landed is recorded as unknown rather than
//! as nothing.

use std::{
	sync::{
		Arc,
		Mutex,
		atomic::{AtomicU64, Ordering},
	},
	time::{Duration, Instant},
};

use crate::Struct::{
	Authority::{Admitted, Authority},
	Call::{Clock, FutureBox},
	Error::Error,
	Port::{Ambiguous, Budget, Destination, Lease, Recorded, Resolution, Seal, Ticket},
	State::{self, Inner, Run},
};

/// Distinguishes one stand-in's leases from another's in the same process.
///
/// Lease ids are built from this token, so a lease issued by one stand-in reads
/// differently from one issued by a second that happens to have counted the same
/// fence for the same resource name. The id is for reading, not for deciding: the
/// decision is the seal, because an id is a string a caller can construct and a
/// seal is not.
static Instances:AtomicU64 = AtomicU64::new(0);

/// Accounts for one admitted local future, however it ends.
///
/// The row survives a release, so settling only decrements: the epoch stays
/// described, which is what lets a late effect from a released epoch be
/// attributed to the holder that had it. `Drop` is what makes the count true for
/// an abandoned future as well as a completed one - there is no `finally` in
/// Rust, and a cancellation that leaked the count would hold a resource until the
/// process ended.
struct Settling {
	Inner:Arc<Mutex<Inner>>,
	Resource:String,
	Fence:u64,
}

impl Drop for Settling {
	fn drop(&mut self) {
		let mut Inner = State::Lock(&self.Inner);
		if let Some(Run) = Inner.Runs.get_mut(&self.Resource)
			&& Run.Fence == self.Fence
		{
			Run.Unsettled = Run.Unsettled.saturating_sub(1);
		}
	}
}

/// The local stand-in: one generation, one clock, one table.
///
/// Cloning one is cloning a handle to the same table, so a destination handed to
/// another thread is still the table the lease verbs are fencing against.
#[derive(Clone)]
pub struct StandIn {
	Generation:u64,
	Clock:Arc<dyn Clock>,
	Token:String,
	Inner:Arc<Mutex<Inner>>,
}

impl StandIn {
	/// A stand-in recording one generation, on the clock it is given.
	pub fn New(Generation:u64, Clock:Arc<dyn Clock>) -> StandIn {
		let Instance = Instances.fetch_add(1, Ordering::SeqCst) + 1;
		StandIn {
			Generation,
			Clock,
			Token:format!("r{Instance}"),
			Inner:Arc::new(Mutex::new(Inner::default())),
		}
	}

	/// Every effect this destination accepted, in the order it accepted them.
	///
	/// A refused effect is absent: it was never recorded, so as far as anything
	/// reading the state is concerned it did not happen. What the world saw is in
	/// [`StandIn::Ambiguities`].
	pub fn Records(&self) -> Vec<Recorded> { State::Lock(&self.Inner).Records.clone() }

	/// Every effect that was admitted and refused, and whose external action is
	/// therefore unknown.
	pub fn Ambiguities(&self) -> Vec<Ambiguous> { State::Lock(&self.Inner).Ambiguities.clone() }

	/// Every resource that is not to be reassigned until its ambiguity is
	/// resolved, sorted so a caller need not know the table's order.
	pub fn Quarantined(&self) -> Vec<String> {
		let Inner = State::Lock(&self.Inner);
		let mut Names:Vec<String> = Inner
			.Runs
			.iter()
			.filter(|(_, Run)| Run.Quarantined)
			.map(|(Name, _)| Name.clone())
			.collect();
		Names.sort();
		Names
	}

	/// State what happened to the effects that made a resource ambiguous, and lift
	/// the quarantine.
	///
	/// The epoch is marked released rather than deleted, so the fence survives and
	/// the next claim advances past it. The ambiguity entries keep the answer, and
	/// stay: the record of an effect that landed under an authority that had ended
	/// is the evidence a later reader needs, and a resolution that erased it would
	/// be the retraction the rule forbids.
	pub fn Resolve(&self, Resource:&str, Resolution:Resolution) -> Result<(), Error> {
		let mut Inner = State::Lock(&self.Inner);
		let Some(Run) = Inner.Runs.get_mut(Resource) else {
			return Err(Error::Refused(format!("no lease is live for {Resource}")));
		};
		if !Run.Quarantined {
			return Err(Error::Refused(format!("{Resource} is not quarantined")));
		}
		Run.Quarantined = false;
		Run.Released = true;
		for Ambiguity in Inner.Ambiguities.iter_mut().filter(|Ambiguity| Ambiguity.Resource == Resource) {
			Ambiguity.Resolution = Some(Resolution);
		}
		Ok(())
	}

	/// The caller's check, evaluated where an effect starts.
	///
	/// Five checks, ordered so the reason is the true one:
	///
	/// 1. **No record** - nothing is held under that name any more.
	/// 2. **Fence mismatch** - the caller's authority was superseded.
	///    `lease-held`: somebody else holds it, and the caller's copy is history.
	///    Checked before the seal, because a superseded caller would fail both and
	///    "you were superseded" is the fact it needs.
	/// 3. **Seal mismatch** - the fence is current but the caller is not the
	///    holder that was admitted. This is the check a guessable identifier
	///    cannot pass.
	/// 4. **Released** - the holder gave the authority back.
	/// 5. **Expired** - the fence matches, the caller is the holder, and authority
	///    ended anyway.
	///
	/// This is the caller's check, and it answers a question about *now*: may this
	/// work start? It is deliberately not the destination's check, which answers a
	/// different question - may this effect be recorded? - at a later moment, and
	/// answers it with an ambiguity when the answer is no. Sharing one function
	/// between the two would reintroduce exactly the defect the destination exists
	/// to close, so [`Destination::Present`] has its own.
	fn Held(Inner:&mut Inner, Resource:&str, Seal:Seal, Fence:u64, Now:Instant) -> Result<&mut Run, Error> {
		let Some(Run) = Inner.Runs.get_mut(Resource) else {
			return Err(Error::Refused(format!("no lease is live for {Resource}")));
		};
		if Run.Fence != Fence {
			return Err(Error::Held(format!("{Resource} is held under fence {}, not fence {Fence}", Run.Fence)));
		}
		if Run.Seal != Seal {
			return Err(Error::Refused(format!(
				"the lease presented for {Resource} was not the one issued: the caller is not the holder that was admitted"
			)));
		}
		if Run.Released {
			return Err(Error::Refused(format!("the lease on {Resource} was released")));
		}
		if Now >= Run.ExpiresAt {
			return Err(Error::Refused(format!(
				"the lease on {Resource} expired at {:?}, and the clock reads {Now:?}",
				Run.ExpiresAt
			)));
		}
		Ok(Run)
	}

	/// Spend one operation of the run's allowance.
	///
	/// Charged before the effect, not after: an effect that fails was still
	/// attempted, and a count that only moved on success would let a run that fails
	/// in a loop spend nothing. The charge happens under the same lock as the fence
	/// check, so two effects cannot both read the last operation.
	fn Charge(Run:&mut Run, Now:Instant) -> Result<(), Error> {
		let Some(Budget) = Run.Budget.as_mut() else {
			return Ok(());
		};
		if let Some(Deadline) = Budget.Deadline
			&& Now >= Deadline
		{
			return Err(Error::Spent(format!("the run's deadline passed at {Deadline:?}, and the clock reads {Now:?}")));
		}
		if Budget.Operations == 0 {
			return Err(Error::Spent("the run has spent every operation it was granted"));
		}
		Budget.Operations -= 1;
		Ok(())
	}

	/// Record an effect as ambiguous and quarantine its resource.
	///
	/// Called only after the ticket's identity matched the current epoch, because
	/// that match is what makes the refusal mean "this effect was admitted": a
	/// guessed ticket is refused without an ambiguity, since nothing was ever
	/// authorised for it to have landed.
	fn Quarantine(Inner:&mut Inner, Ticket:&Ticket, Name:&str, Now:Instant, Reason:String) -> Error {
		Inner.Ambiguities.push(Ambiguous {
			Name:Name.to_string(),
			Resource:Ticket.Resource().to_string(),
			Generation:Ticket.Generation(),
			Fence:Ticket.Fence(),
			LeaseId:Ticket.LeaseId().to_string(),
			At:Now,
			Reason:Reason.clone(),
			Resolution:None,
		});
		if let Some(Run) = Inner.Runs.get_mut(Ticket.Resource()) {
			Run.Quarantined = true;
		}
		Error::Refused(format!(
			"{Reason}. The effect may have landed, so it is recorded as ambiguous and {} is quarantined until a resolver states whether it did.",
			Ticket.Resource()
		))
	}
}

impl Authority for StandIn {
	fn Generation(&self) -> u64 { self.Generation }

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
				if Current.Quarantined {
					return Err(Error::Held(format!(
						"{Resource} is quarantined: an effect admitted under fence {} was refused when it landed, and no holder is assigned until a resolver states whether it landed",
						Current.Fence
					)));
				}
				if !Current.Released && Now < Current.ExpiresAt {
					return Err(Error::Held(format!(
						"{Resource} is held under fence {} until {:?}",
						Current.Fence, Current.ExpiresAt
					)));
				}
			}
			let Fence = Inner.Fences.get(Resource).copied().unwrap_or(0) + 1;
			Inner.Fences.insert(Resource.to_string(), Fence);
			let Lease = Lease {
				Id:format!("{}#{Resource}#{Fence}", self.Token),
				Resource:Resource.to_string(),
				Fence,
				ExpiresAt,
				Seal:Seal::Mint(),
			};
			Inner.Runs.insert(
				Resource.to_string(),
				Run {
					Id:Lease.Id.clone(),
					Seal:Lease.Seal,
					Generation:self.Generation,
					Fence,
					ExpiresAt,
					Budget,
					Unsettled:0,
					Released:false,
					Quarantined:false,
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
			let Run = Self::Held(&mut Inner, &Lease.Resource, Lease.Seal, Lease.Fence, Now)?;
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

	fn Release<'a>(&'a self, Lease:Lease) -> FutureBox<'a, ()> {
		Box::pin(async move {
			let mut Inner = State::Lock(&self.Inner);
			// A superseded lease is not this caller's to end: the resource now
			// belongs to the fence that took it over, and releasing here would
			// hand a stranger's authority back. The row survives either way,
			// because it is the only thing that still knows who held the epoch.
			if let Some(Run) = Inner.Runs.get_mut(&Lease.Resource)
				&& Run.Fence == Lease.Fence
				&& Run.Seal == Lease.Seal
			{
				Run.Released = true;
			}
		})
	}

	fn Admit<'a>(&'a self, Lease:Lease) -> FutureBox<'a, Result<Admitted, Error>> {
		Box::pin(async move {
			// The admission step: check, charge, count, and mint the authority
			// the effect will carry. All four happen under one lock, so the
			// fence in the ticket is the fence the charge was taken under.
			let Now = self.Clock.Now();
			let mut Inner = State::Lock(&self.Inner);
			let Ticket = {
				let Run = Self::Held(&mut Inner, &Lease.Resource, Lease.Seal, Lease.Fence, Now)?;
				Self::Charge(Run, Now)?;
				Run.Unsettled += 1;
				// The record's expiry, not the copy the caller presented: a lease
				// renewed after it was handed out is still the same grant, and the
				// caller's copy is the older number.
				Ticket::Mint(self.Generation, &Lease, Run.ExpiresAt)
			};
			let Settle:Box<dyn Send> = Box::new(Settling {
				Inner:self.Inner.clone(),
				Resource:Lease.Resource.clone(),
				Fence:Lease.Fence,
			});
			Ok(Admitted { Ticket, Settle })
		})
	}
}

impl Destination for StandIn {
	fn Present<'a>(&'a self, Ticket:Ticket, Name:&'a str) -> FutureBox<'a, Result<Recorded, Error>> {
		Box::pin(async move {
			let Now = self.Clock.Now();
			let mut Inner = State::Lock(&self.Inner);
			// The generation first: a ticket minted under another generation
			// carries numbers this authority never counted, so nothing about it
			// can be attributed here.
			if Ticket.Generation() != self.Generation {
				return Err(Error::Refused(format!(
					"the effect was admitted under generation {}, and this destination records generation {}",
					Ticket.Generation(),
					self.Generation
				)));
			}
			let Some((Seal, Fence, Released, Quarantined, ExpiresAt)) = Inner
				.Runs
				.get(Ticket.Resource())
				.map(|Run| (Run.Seal, Run.Fence, Run.Released, Run.Quarantined, Run.ExpiresAt))
			else {
				return Err(Error::Refused(format!("no lease is live for {}", Ticket.Resource())));
			};
			// The fence before the seal, for the same reason the caller's check
			// does: a superseded ticket fails both, and "the epoch you were in has
			// ended" is the fact it needs. A superseded epoch is where the
			// takeover, not this destination, does the accounting.
			if Fence != Ticket.Fence() {
				return Err(Error::Held(format!(
					"{} is held under fence {Fence}, not fence {}",
					Ticket.Resource(),
					Ticket.Fence()
				)));
			}
			// The identity: the numbers matching is not the caller matching.
			if Seal != Ticket.Seal() {
				return Err(Error::Refused(format!(
					"the ticket presented for fence {Fence} of {} was not the one issued: the numbers match and the identity does not",
					Ticket.Resource()
				)));
			}
			// From here the ticket names the current epoch, so it is one this
			// authority issued and the effect it carries was authorised. Any
			// refusal now means an authorised effect may have landed, and that is
			// what makes it ambiguous rather than merely refused.
			if Quarantined {
				return Err(Self::Quarantine(
					&mut Inner,
					&Ticket,
					Name,
					Now,
					format!("{} is already quarantined by an earlier effect", Ticket.Resource()),
				));
			}
			if Released {
				return Err(Self::Quarantine(
					&mut Inner,
					&Ticket,
					Name,
					Now,
					format!("the lease on {} was released while the effect was in flight", Ticket.Resource()),
				));
			}
			if Now >= ExpiresAt {
				return Err(Self::Quarantine(
					&mut Inner,
					&Ticket,
					Name,
					Now,
					format!(
						"the effect outlived its authority: the lease on {} ended at {ExpiresAt:?}, the ticket was minted for {:?}, and the clock reads {Now:?}",
						Ticket.Resource(),
						Ticket.ExpiresAt()
					),
				));
			}
			let Receipt = Recorded {
				Name:Name.to_string(),
				Resource:Ticket.Resource().to_string(),
				Fence,
				Generation:self.Generation,
				At:Now,
			};
			Inner.Records.push(Receipt.clone());
			Ok(Receipt)
		})
	}
}

//! `agentic.runtime`, exercised against a clock the test owns.
//!
//! No tokio, no browser, no network and no sleeping: a lease expires because the
//! test advances a clock, which is the whole reason the clock is a parameter of
//! the port rather than a call to the system's.
//!
//! Three requirements meet in this file and they are not the same requirement:
//!
//! - **Authority** is decided where an effect lands. `Present` re-reads the table
//!   at the instant the effect arrives and refuses a stale fence, a stale
//!   generation, a released lease or an expired record. The test that proves it is
//!   `an_effect_that_outlives_its_lease_is_refused_where_it_lands_and_recorded_ambiguous`,
//!   which is the counterexample from `docs/Review.md` executed with the write
//!   before the record rather than after it.
//! - **Identity** is not a number. `a_ticket_whose_numbers_match_but_whose_seal_was_never_issued_is_refused_and_records_no_ambiguity`
//!   presents a ticket with the right generation, resource, lease id, fence and
//!   expiry, and is refused, because the destination compares the holder it
//!   recorded and not the numbers it can be told.
//! - **Accounting** is decided by the unsettled-future count, which says a local
//!   future has not settled. `a_takeover_waits_for_an_unsettled_local_future`
//!   encodes that and only that. It does not say authority survives a settled
//!   future, and it is not the guarantee.
//!
//! The three run against `StandIn`, which is the local stand-in for the reducer.
//! That is the point of the split: the runner under test holds no lease state, so
//! every assertion about a fence is an assertion about the double, and the double
//! is pinned to the reducer by the mirror test in `packages/spacetime-catalog`.

use std::{
	sync::{
		Arc,
		Mutex,
		atomic::{AtomicBool, Ordering},
	},
	time::{Duration, Instant},
};

use AgenticRuntime::{
	Admission,
	BlockOn,
	Budget,
	Call,
	Cancellation,
	Clock,
	Code,
	Create,
	Destination,
	Error,
	Lease,
	Options,
	Recorded,
	Request,
	Resolution,
	Runner,
	Runtime,
	Seal,
	StandIn,
	StateClass,
	States,
	Ticket,
};

/// A clock that only moves when the test moves it.
#[derive(Debug)]
struct TestClock {
	Now:Mutex<Instant>,
}

impl TestClock {
	fn New() -> Arc<Self> { Arc::new(Self { Now:Mutex::new(Instant::now()) }) }

	fn Advance(&self, Span:Duration) {
		let mut Now = self.Now.lock().unwrap();
		*Now += Span;
	}
}

impl Clock for TestClock {
	fn Now(&self) -> Instant { *self.Now.lock().unwrap() }
}

/// A runner bound to generation 7, over a stand-in on the test's clock.
///
/// The generation is passed twice on purpose: once to the options the runner
/// enforces and once to the stand-in that records. They are two objects, and a
/// test that could not tell them apart could not tell a second authority from
/// the one it is supposed to be.
fn Engine(Clock:Arc<TestClock>) -> Runner<StandIn> {
	let Options = Options::New(7)
		.With("chat", StateClass::HoldsTransport)
		.With("verify", StateClass::Stateless)
		.On(Clock.clone());
	let Authority = StandIn::New(Options.Generation, Clock);
	Create(Options, Authority).expect("the runtime binds")
}

fn Claim(System:&Runner<StandIn>, Resource:&str, Ttl:Duration) -> Result<Lease, Error> {
	BlockOn(System.Claim(Resource, Ttl, None))
}

fn Claimed(System:&Runner<StandIn>, Resource:&str, Ttl:Duration) -> Lease {
	Claim(System, Resource, Ttl).expect("the claim is admitted")
}

/// Present a ticket to the destination, which is the only place it can be
/// refused for being late.
fn Present(Place:&StandIn, Ticket:Ticket, Name:&str) -> Result<Recorded, Error> {
	BlockOn(Place.Present(Ticket, Name))
}

fn Span(Milliseconds:u64) -> Duration { Duration::from_millis(Milliseconds) }

/// Poll a future once, without an executor, so a test can observe the state
/// between "admitted" and "settled" instead of only after it finishes.
fn PollOnce<Work:std::future::Future + ?Sized>(Work:std::pin::Pin<&mut Work>) -> bool {
	let Waker = std::task::Waker::noop();
	let mut Context = std::task::Context::from_waker(Waker);
	matches!(Work.poll(&mut Context), std::task::Poll::Ready(_))
}

/// A latch an effect waits on, so a test can hold it between "admitted" and
/// "the write" without an executor.
///
/// `BlockOn` re-polls in a loop, so a latch opened without waking anyone is
/// still observed on the next poll.
#[derive(Clone)]
struct Gate(Arc<AtomicBool>);

impl Gate {
	fn New() -> Self { Self(Arc::new(AtomicBool::new(false))) }

	fn Open(&self) { self.0.store(true, Ordering::SeqCst); }

	fn Wait(&self) -> Waiting { Waiting(self.0.clone()) }
}

/// The future half of a [`Gate`].
struct Waiting(Arc<AtomicBool>);

impl std::future::Future for Waiting {
	type Output = ();

	fn poll(
		self:std::pin::Pin<&mut Self>,
		_Context:&mut std::task::Context<'_>,
	) -> std::task::Poll<()> {
		if self.0.load(Ordering::SeqCst) {
			std::task::Poll::Ready(())
		} else {
			std::task::Poll::Pending
		}
	}
}

#[test]
fn the_port_declares_exactly_the_errors_the_manifest_promises() {
	let Declared:Vec<&str> = Code::All().into_iter().map(Code::Name).collect();
	assert_eq!(Declared, vec!["admission-refused", "lease-held", "budget-exhausted"]);
	let Manifest = include_str!("../module.json");
	for Name in Declared {
		assert!(Manifest.contains(&format!("\"{Name}\"")), "{Name} is not in module.json");
	}
}

#[test]
fn the_port_declares_exactly_the_state_classes_the_manifest_promises() {
	let Declared:Vec<&str> = States.into_iter().map(StateClass::Name).collect();
	assert_eq!(
		Declared,
		vec!["stateless", "holds-transport", "holds-identity", "holds-workspace"]
	);
	assert_eq!(StateClass::Parse("holds-identity"), Some(StateClass::HoldsIdentity));
	assert_eq!(StateClass::Parse("resident"), None);
}

#[test]
fn reads_the_clock_it_was_given_not_the_wall_clock() {
	let Clock = TestClock::New();
	let Engine = Engine(Clock.clone());
	let Before = Engine.Now();
	Clock.Advance(Span(1_000));
	assert_eq!(Engine.Now(), Before + Span(1_000));
}

#[test]
fn admits_a_registered_capability_at_the_generation_it_is_bound_to() {
	let Engine = Engine(TestClock::New());
	let Answer = BlockOn(Engine.Admit(Request {
		Capability:"chat".to_string(),
		State:StateClass::HoldsTransport,
		Generation:7,
	}));
	assert_eq!(Answer, Admission::Admitted);
	assert!(Answer.Ok());
}

#[test]
fn refuses_a_generation_it_was_not_bound_to() {
	let Engine = Engine(TestClock::New());
	let Answer = BlockOn(Engine.Admit(Request {
		Capability:"chat".to_string(),
		State:StateClass::HoldsTransport,
		Generation:8,
	}));
	assert!(!Answer.Ok());
	assert!(Answer.Reason().starts_with("admission-refused: generation 8"));
}

#[test]
fn refuses_a_capability_it_does_not_register() {
	let Engine = Engine(TestClock::New());
	let Answer = BlockOn(Engine.Admit(Request {
		Capability:"browse".to_string(),
		State:StateClass::HoldsTransport,
		Generation:7,
	}));
	assert!(!Answer.Ok());
	assert!(Answer.Reason().contains("browse is not registered"));
}

#[test]
fn refuses_a_state_class_the_registration_does_not_declare() {
	let Engine = Engine(TestClock::New());
	let Answer = BlockOn(Engine.Admit(Request {
		Capability:"verify".to_string(),
		State:StateClass::HoldsWorkspace,
		Generation:7,
	}));
	assert!(!Answer.Ok());
	assert!(Answer.Reason().contains("verify declares stateless"));
}

#[test]
fn the_manifest_it_reports_is_the_manifest_it_enforces() {
	let Engine = Engine(TestClock::New());
	let Binding = Engine.Reflect();
	assert_eq!(Binding.Generation, 7);
	let Names:Vec<&str> = Binding
		.Capabilities
		.iter()
		.map(|Registration| Registration.Name.as_str())
		.collect();
	assert_eq!(Names, vec!["chat", "verify"], "sorted by name");
}

#[test]
fn a_second_claim_is_refused_while_the_first_is_live() {
	let Engine = Engine(TestClock::New());
	let First = Claimed(&Engine, "page", Span(1_000));
	let Second = Claim(&Engine, "page", Span(1_000));
	assert_eq!(Second.unwrap_err().Code(), Code::LeaseHeld);
	assert_eq!(First.Fence, 1);
}

#[test]
fn a_claim_takes_over_an_expired_lease_and_the_fence_advances() {
	let Clock = TestClock::New();
	let Engine = Engine(Clock.clone());
	let First = Claimed(&Engine, "page", Span(100));
	Clock.Advance(Span(101));
	let Second = Claimed(&Engine, "page", Span(100));
	assert_eq!(Second.Fence, First.Fence + 1);
	assert_ne!(Second.Id, First.Id);
	assert_ne!(Second.Seal, First.Seal, "a new holder is a new identity");
}

#[test]
fn a_stale_fence_is_refused_and_the_write_never_lands() {
	let Clock = TestClock::New();
	let Engine = Engine(Clock.clone());
	let Stale = Claimed(&Engine, "page", Span(100));
	Clock.Advance(Span(200));
	let _Current = Claimed(&Engine, "page", Span(100));
	let Landed = AtomicBool::new(false);
	let Outcome = BlockOn(Engine.WithLease(Stale, |_Ticket| {
		Box::pin(async {
			Landed.store(true, Ordering::SeqCst);
			1
		})
	}));
	assert_eq!(Outcome.unwrap_err().Code(), Code::LeaseHeld);
	assert!(!Landed.load(Ordering::SeqCst), "the late write landed");
}

#[test]
fn a_lease_that_expires_mid_run_refuses_the_next_write() {
	let Clock = TestClock::New();
	let Engine = Engine(Clock.clone());
	let Lease = Claimed(&Engine, "page", Span(100));
	let First = BlockOn(Engine.WithLease(Lease.clone(), |_Ticket| Box::pin(async { 1 })));
	assert_eq!(First.unwrap(), 1);
	Clock.Advance(Span(101));
	let Second = BlockOn(Engine.WithLease(Lease, |_Ticket| Box::pin(async { 2 })));
	let Error = Second.unwrap_err();
	assert_eq!(Error.Code(), Code::AdmissionRefused);
	assert!(Error.Message().contains("expired"));
}

#[test]
fn an_effect_that_outlives_its_lease_is_refused_where_it_lands_and_recorded_ambiguous() {
	// `docs/Review.md`, executed step for step, with the write where the review
	// put it - before the record, not after it:
	//
	//   lease is live
	//   WithLease admits the callback
	//   callback waits
	//   lease expires
	//   callback performs the external write      <- the bytes are in the page now
	//   the ticket is presented                   <- only now can anything refuse
	//   WithLease returns success                 <- what must no longer happen
	//
	// The admission happened while the lease was live and cannot be un-made, and
	// the write cannot be un-sent. The only party that can notice is the one the
	// record is presented to, and the only honest answer it can give is "this
	// effect was authorised, it ran, and I will not count it" - an ambiguity, not
	// a silence.
	let Clock = TestClock::New();
	let Engine = Engine(Clock.clone());
	let Place = Engine.Destination();
	let Lease = Claimed(&Engine, "page", Span(100));
	let Latch = Gate::New();
	let Wrote = Arc::new(AtomicBool::new(false));
	let mut Effect = Engine.WithLease(Lease.clone(), |Ticket| {
		let Wait = Latch.Wait();
		let Wrote = Wrote.clone();
		let Place = Place.clone();
		Box::pin(async move {
			Wait.await;
			// The external action, first and unconditional. Nothing here consults
			// the lease, because a real effect does not: by the time it writes, the
			// only copy of its authority is the ticket it is about to present.
			Wrote.store(true, Ordering::SeqCst);
			Place.Present(Ticket, "write").await
		})
	});
	assert!(!PollOnce(Effect.as_mut()), "the effect is admitted and waiting");
	Clock.Advance(Span(101));
	Latch.Open();
	let Outcome = BlockOn(Effect);
	assert!(Wrote.load(Ordering::SeqCst), "the effect must have written before it presented");
	// The destination's verdict is checked first, because it is the one that
	// matters: whatever the caller reports afterwards, an effect that left a
	// record behind landed.
	assert!(Place.Records().is_empty(), "a refused effect left a record behind");
	let Error = Outcome
		.expect("the effect was admitted")
		.expect_err("the destination recorded a write it should have refused");
	assert_eq!(Error.Code(), Code::AdmissionRefused);
	assert!(Error.Message().contains("outlived"), "{}", Error.Message());
	// The state did not claim the effect, and the world is not the state: the
	// write is recorded as unknown rather than as nothing.
	let Ambiguous = Place.Ambiguities();
	assert_eq!(Ambiguous.len(), 1, "the external action was not recorded as ambiguous");
	assert_eq!(Ambiguous[0].Name, "write");
	assert_eq!(Ambiguous[0].Resource, "page");
	assert_eq!(Ambiguous[0].Fence, Lease.Fence);
	assert_eq!(Ambiguous[0].Generation, 7);
	assert_eq!(Ambiguous[0].Resolution, None, "the outcome is unknown, not guessed");
	// The resource is not handed to anybody else while the outcome is unknown.
	assert_eq!(Place.Quarantined(), vec!["page".to_string()]);
	let Takeover = Claim(&Engine, "page", Span(1_000)).unwrap_err();
	assert_eq!(Takeover.Code(), Code::LeaseHeld);
	assert!(Takeover.Message().contains("quarantined"), "{}", Takeover.Message());
	// A resolver states what happened, and only then is reassignment possible.
	Place.Resolve("page", Resolution::Landed).expect("the resource is quarantined");
	assert!(Place.Quarantined().is_empty());
	assert_eq!(Place.Ambiguities()[0].Resolution, Some(Resolution::Landed));
	assert!(Place.Records().is_empty(), "a resolution is not a record of authority");
	assert_eq!(Claimed(&Engine, "page", Span(1_000)).Fence, Lease.Fence + 1);
}

#[test]
fn an_effect_that_presents_before_it_writes_never_writes_and_is_still_unknown() {
	// The other ordering, and the reason the port says "present first if you can":
	// an effect that records before it acts cannot act once its authority ended,
	// because the record is refused before the external call is made.
	//
	// It is still recorded as ambiguous, and that is not a defect in the rule: the
	// destination cannot know whether the caller wrote before or after presenting,
	// so the honest record is "unknown". A resolver turns it into `Absent`, and
	// then nothing claims the effect at all.
	let Clock = TestClock::New();
	let Engine = Engine(Clock.clone());
	let Place = Engine.Destination();
	let Lease = Claimed(&Engine, "page", Span(100));
	let Latch = Gate::New();
	let Wrote = Arc::new(AtomicBool::new(false));
	let mut Effect = Engine.WithLease(Lease.clone(), |Ticket| {
		let Wait = Latch.Wait();
		let Wrote = Wrote.clone();
		let Place = Place.clone();
		Box::pin(async move {
			Wait.await;
			Place.Present(Ticket, "write").await?;
			Wrote.store(true, Ordering::SeqCst);
			Ok::<(), Error>(())
		})
	});
	assert!(!PollOnce(Effect.as_mut()), "the effect is admitted and waiting");
	Clock.Advance(Span(101));
	Latch.Open();
	let Outcome = BlockOn(Effect).expect("the effect was admitted");
	assert!(Outcome.is_err(), "the record was accepted after the lease ended");
	assert!(!Wrote.load(Ordering::SeqCst), "the write happened after the record was refused");
	assert!(Place.Records().is_empty());
	assert_eq!(Place.Ambiguities().len(), 1, "an effect that may have acted is unknown, not absent");
	Place.Resolve("page", Resolution::Absent).expect("the resource is quarantined");
	assert_eq!(Place.Ambiguities()[0].Resolution, Some(Resolution::Absent));
	assert!(Place.Records().is_empty(), "an absent effect is never recorded");
	assert_eq!(Claimed(&Engine, "page", Span(1_000)).Fence, Lease.Fence + 1);
}

#[test]
fn an_effect_whose_lease_was_released_while_it_ran_is_refused_and_recorded_ambiguous() {
	// Release is the other way authority ends, and it has the same consequence:
	// the effect was admitted, the holder gave the authority back, and the write
	// arrives afterwards. The row survives the release precisely so this case can
	// still be attributed to somebody.
	let Clock = TestClock::New();
	let Engine = Engine(Clock.clone());
	let Place = Engine.Destination();
	let Lease = Claimed(&Engine, "page", Span(1_000));
	let Kept:Arc<Mutex<Option<Ticket>>> = Arc::new(Mutex::new(None));
	BlockOn(Engine.WithLease(Lease.clone(), {
		let Kept = Kept.clone();
		move |Ticket| {
			Kept.lock().unwrap().replace(Ticket);
			Box::pin(async {})
		}
	}))
	.expect("the effect was admitted");
	BlockOn(Engine.Release(Lease.clone()));
	let Late = Kept.lock().unwrap().take().expect("the effect kept its ticket");
	let Error = Present(&Place, Late, "late").expect_err("the destination refused it");
	assert_eq!(Error.Code(), Code::AdmissionRefused);
	assert!(Error.Message().contains("released"), "{}", Error.Message());
	assert!(Place.Records().is_empty());
	assert_eq!(Place.Ambiguities().len(), 1);
	assert_eq!(Place.Quarantined(), vec!["page".to_string()]);
}

#[test]
fn a_ticket_from_a_superseded_fence_is_refused_where_it_lands() {
	let Clock = TestClock::New();
	let Engine = Engine(Clock.clone());
	let Place = Engine.Destination();
	let Lease = Claimed(&Engine, "page", Span(100));
	let Kept:Arc<Mutex<Option<Ticket>>> = Arc::new(Mutex::new(None));
	let First = {
		let Kept = Kept.clone();
		BlockOn(Engine.WithLease(Lease.clone(), move |Ticket| {
			let Kept = Kept.clone();
			Box::pin(async move {
				Kept.lock().unwrap().replace(Ticket);
			})
		}))
	};
	assert!(First.is_ok(), "the effect was admitted");
	Clock.Advance(Span(101));
	let Current = Claimed(&Engine, "page", Span(1_000));
	assert_eq!(Current.Fence, Lease.Fence + 1);
	let Stale = Kept.lock().unwrap().take().expect("the effect kept its ticket");
	let Error = Present(&Place, Stale, "late").expect_err("the destination refused it");
	assert_eq!(Error.Code(), Code::LeaseHeld);
	assert!(Error.Message().contains("fence 2, not fence 1"), "{}", Error.Message());
	assert!(Place.Records().is_empty());
	// A superseded epoch is refused without an ambiguity, and the doc says why:
	// the row no longer describes that epoch, so the takeover - not this
	// destination - is the accounting point for it.
	assert!(Place.Ambiguities().is_empty());
}

#[test]
fn a_ticket_from_another_generation_is_refused_where_it_lands() {
	let Engine = Engine(TestClock::New());
	let Place = Engine.Destination();
	let Lease = Claimed(&Engine, "page", Span(1_000));
	let Forged = Ticket::Restore(8, Lease.Resource.clone(), Lease.Id.clone(), Lease.Fence, Lease.ExpiresAt, Lease.Seal);
	let Error = Present(&Place, Forged, "forged").expect_err("the destination refused it");
	assert_eq!(Error.Code(), Code::AdmissionRefused);
	assert!(Error.Message().contains("generation 8"), "{}", Error.Message());
	assert!(Place.Records().is_empty());
}

#[test]
fn a_ticket_whose_numbers_match_but_whose_seal_was_never_issued_is_refused_and_records_no_ambiguity() {
	// The review's first gap, as a test. Generation, resource, lease id, fence and
	// expiry are all correct - a caller that observed the lease and reconstructed
	// every number it could see - and the ticket is still refused, because the
	// numbers are not the identity.
	let Engine = Engine(TestClock::New());
	let Place = Engine.Destination();
	let Lease = Claimed(&Engine, "page", Span(1_000));
	let Guessed = Ticket::Restore(
		7,
		Lease.Resource.clone(),
		Lease.Id.clone(),
		Lease.Fence,
		Lease.ExpiresAt,
		Seal::Empty,
	);
	let Error = Present(&Place, Guessed, "guessed").expect_err("the destination refused it");
	assert_eq!(Error.Code(), Code::AdmissionRefused);
	assert!(Error.Message().contains("was not the one issued"), "{}", Error.Message());
	assert!(Place.Records().is_empty());
	// Nothing was admitted for this caller, so there is no external action to be
	// ambiguous about. Recording one would let a stranger quarantine a resource.
	assert!(Place.Ambiguities().is_empty(), "a caller that was never admitted has no effect to be ambiguous about");
	assert!(Place.Quarantined().is_empty());
}

#[test]
fn a_ticket_restored_from_the_row_the_authority_issued_is_accepted() {
	// `Restore` is the adapter's door, and it works: a ticket rebuilt from the row
	// the authority wrote, carrying the seal that row holds, is the same authority
	// as the one the runner handed out. That is what makes the production path - a
	// ticket read back out of the database - the same path this test exercises.
	let Engine = Engine(TestClock::New());
	let Place = Engine.Destination();
	let Lease = Claimed(&Engine, "page", Span(1_000));
	let Restored = Ticket::Restore(7, Lease.Resource.clone(), Lease.Id.clone(), Lease.Fence, Lease.ExpiresAt, Lease.Seal);
	let Receipt = Present(&Place, Restored, "restored").expect("the row the authority issued");
	assert_eq!(Receipt.Name, "restored");
	assert_eq!(Receipt.Fence, Lease.Fence);
	assert_eq!(Receipt.Generation, 7);
}

#[test]
fn a_ticket_for_a_resource_this_authority_never_claimed_is_refused_without_an_ambiguity() {
	let Engine = Engine(TestClock::New());
	let Place = Engine.Destination();
	let Forged = Ticket::Restore(7, "page", "r999#page#1", 1, Engine.Now(), Seal::Empty);
	let Error = Present(&Place, Forged, "forged").expect_err("the destination refused it");
	assert_eq!(Error.Code(), Code::AdmissionRefused);
	assert!(Error.Message().contains("no lease is live"), "{}", Error.Message());
	assert!(Place.Ambiguities().is_empty());
}

#[test]
fn a_ticket_never_prints_the_seal_it_carries() {
	// A ticket is a bearer token. A debug line that prints it is a debug line that
	// hands it to whoever reads the log.
	let Engine = Engine(TestClock::New());
	let Lease = Claimed(&Engine, "page", Span(1_000));
	let Ticket = BlockOn(Engine.WithLease(Lease.clone(), |Ticket| Box::pin(async move { Ticket })))
		.expect("the effect was admitted");
	let Printed = format!("{Ticket:?}");
	assert!(Printed.contains("Seal(<redacted>)"), "{Printed}");
	assert!(!Printed.contains(&Ticket.Seal().Hex()), "a logged ticket is a leaked ticket");
	assert!(!format!("{Lease:?}").contains(&Lease.Seal.Hex()));
}

#[test]
fn a_recorded_effect_keeps_the_fence_and_generation_it_was_admitted_under() {
	let Engine = Engine(TestClock::New());
	let Place = Engine.Destination();
	let Lease = Claimed(&Engine, "page", Span(1_000));
	let Receipt = BlockOn(Engine.WithLease(Lease.clone(), |Ticket| {
		let Place = Place.clone();
		Box::pin(async move { Place.Present(Ticket, "write").await })
	}))
	.expect("the effect was admitted")
	.expect("the destination recorded it");
	assert_eq!(Receipt.Name, "write");
	assert_eq!(Receipt.Resource, "page");
	assert_eq!(Receipt.Fence, Lease.Fence);
	assert_eq!(Receipt.Generation, 7);
	assert_eq!(Place.Records(), vec![Receipt], "the receipt is the record");
	assert!(Place.Ambiguities().is_empty());
}

#[test]
fn a_renewal_extends_the_lease_for_the_current_holder() {
	let Clock = TestClock::New();
	let Engine = Engine(Clock.clone());
	let Lease = Claimed(&Engine, "page", Span(100));
	Clock.Advance(Span(50));
	let Renewed = BlockOn(Engine.Renew(Lease.clone(), Span(500))).expect("the renewal holds");
	assert_eq!(Renewed.Fence, Lease.Fence);
	assert_eq!(Renewed.Id, Lease.Id);
	assert_eq!(Renewed.Seal, Lease.Seal, "a renewal is the same grant, not a new one");
	assert_eq!(Renewed.ExpiresAt, Lease.ExpiresAt + Span(450));
	Clock.Advance(Span(400));
	let Write = BlockOn(Engine.WithLease(Renewed, |_Ticket| Box::pin(async { 1 })));
	assert_eq!(Write.unwrap(), 1, "the extension is what kept it live");
}

#[test]
fn a_renewal_that_does_not_extend_the_lease_is_refused() {
	let Clock = TestClock::New();
	let Engine = Engine(Clock.clone());
	let Lease = Claimed(&Engine, "page", Span(1_000));
	Clock.Advance(Span(500));
	let Outcome = BlockOn(Engine.Renew(Lease, Span(100)));
	let Error = Outcome.unwrap_err();
	assert_eq!(Error.Code(), Code::AdmissionRefused);
	assert!(Error.Message().contains("must extend"));
}

#[test]
fn a_renewal_after_expiry_is_refused_rather_than_resurrected() {
	let Clock = TestClock::New();
	let Engine = Engine(Clock.clone());
	let Lease = Claimed(&Engine, "page", Span(100));
	Clock.Advance(Span(101));
	let Outcome = BlockOn(Engine.Renew(Lease, Span(1_000)));
	assert_eq!(Outcome.unwrap_err().Code(), Code::AdmissionRefused);
}

#[test]
fn a_renewal_from_a_superseded_lease_is_refused() {
	let Clock = TestClock::New();
	let Engine = Engine(Clock.clone());
	let First = Claimed(&Engine, "page", Span(100));
	Clock.Advance(Span(101));
	let Second = Claimed(&Engine, "page", Span(1_000));
	let Outcome = BlockOn(Engine.Renew(First, Span(2_000)));
	assert_eq!(Outcome.unwrap_err().Code(), Code::LeaseHeld);
	assert_eq!(Second.Fence, 2);
}

#[test]
fn a_lease_from_another_runtime_is_not_this_runtimes_lease() {
	let Clock = TestClock::New();
	let One = Engine(Clock.clone());
	let Two = Engine(Clock.clone());
	let Foreign = Claimed(&One, "page", Span(1_000));
	// The second runtime issues its own first fence for the same resource name.
	let Own = Claimed(&Two, "page", Span(1_000));
	assert_eq!(Foreign.Fence, Own.Fence, "both are fence 1");
	let Outcome = BlockOn(Two.WithLease(Foreign, |_Ticket| Box::pin(async { 1 })));
	let Error = Outcome.unwrap_err();
	assert_eq!(Error.Code(), Code::AdmissionRefused);
	assert!(Error.Message().contains("was not the one issued"));
}

#[test]
fn a_takeover_waits_for_an_unsettled_local_future() {
	// This encodes the accounting rule and nothing more: a future this process
	// admitted has not settled, so a local takeover does not begin under it.
	// It is not the authority rule - that one is
	// `an_effect_that_outlives_its_lease_is_refused_where_it_lands_and_recorded_ambiguous`,
	// which lets the future settle and refuses the write anyway.
	let Clock = TestClock::New();
	let Engine = Engine(Clock.clone());
	let Lease = Claimed(&Engine, "page", Span(100));
	// The effect is admitted, and the lease expires while it is still running.
	let mut Write = Engine.WithLease(Lease.clone(), |_Ticket| Box::pin(std::future::pending::<i32>()));
	assert!(!PollOnce(Write.as_mut()), "the effect is admitted and waiting");
	Clock.Advance(Span(200));
	let Running = Engine.Claim("page", Span(1_000), None);
	let Error = BlockOn(Running).unwrap_err();
	assert_eq!(Error.Code(), Code::LeaseHeld);
	assert!(Error.Message().contains("unsettled local future"), "{}", Error.Message());
	// Dropping the abandoned effect settles it, which is what frees the
	// resource. Dropping a future is not proof that the work behind it stopped;
	// it is proof that this process is no longer counting it.
	drop(Write);
	assert_eq!(Claimed(&Engine, "page", Span(1_000)).Fence, 2);
}

#[test]
fn a_malformed_claim_is_refused_not_panicked() {
	let Engine = Engine(TestClock::New());
	let Empty = Claim(&Engine, "", Span(1_000)).unwrap_err();
	assert_eq!(Empty.Code(), Code::AdmissionRefused);
	let Zero = Claim(&Engine, "page", Duration::ZERO).unwrap_err();
	assert_eq!(Zero.Code(), Code::AdmissionRefused);
	let Huge = Claim(&Engine, "page", Duration::MAX).unwrap_err();
	assert_eq!(Huge.Code(), Code::AdmissionRefused);
}

#[test]
fn release_is_safe_twice_and_the_next_claim_takes_a_new_fence() {
	let Engine = Engine(TestClock::New());
	let Lease = Claimed(&Engine, "page", Span(1_000));
	BlockOn(Engine.Release(Lease.clone()));
	BlockOn(Engine.Release(Lease.clone()));
	let Next = Claimed(&Engine, "page", Span(1_000));
	assert_eq!(Next.Fence, 2);
	assert_ne!(Next.Seal, Lease.Seal);
}

#[test]
fn releasing_a_superseded_lease_does_not_free_the_new_holders_resource() {
	let Clock = TestClock::New();
	let Engine = Engine(Clock.clone());
	let First = Claimed(&Engine, "page", Span(100));
	Clock.Advance(Span(101));
	let Second = Claimed(&Engine, "page", Span(1_000));
	BlockOn(Engine.Release(First));
	let Write = BlockOn(Engine.WithLease(Second, |_Ticket| Box::pin(async { 1 })));
	assert_eq!(Write.unwrap(), 1, "the new holder still holds it");
}

#[test]
fn release_during_an_effect_frees_the_resource_once_it_settles() {
	let Clock = TestClock::New();
	let Engine = Engine(Clock.clone());
	let Lease = Claimed(&Engine, "page", Span(1_000));
	let mut Write = Engine.WithLease(Lease.clone(), |_Ticket| Box::pin(std::future::pending::<i32>()));
	assert!(!PollOnce(Write.as_mut()), "the effect is admitted and waiting");
	// Released while the write is admitted but not yet settled.
	BlockOn(Engine.Release(Lease));
	// The record survives only to account for the effect, so nothing may take
	// it.
	assert_eq!(Claim(&Engine, "page", Span(1_000)).unwrap_err().Code(), Code::LeaseHeld);
	drop(Write);
	let Next = Claimed(&Engine, "page", Span(1_000));
	assert_eq!(Next.Fence, 2, "the resource came back");
}

#[test]
fn release_never_fails_whatever_it_is_handed() {
	let Engine = Engine(TestClock::New());
	let Foreign = Lease {
		Id:"r999#page#1".to_string(),
		Resource:"page".to_string(),
		Fence:1,
		ExpiresAt:Instant::now(),
		Seal:Seal::Empty,
	};
	BlockOn(Engine.Release(Foreign.clone()));
	BlockOn(Engine.Release(Foreign));
}

#[test]
fn a_budget_is_not_inherited_a_new_run_starts_from_the_callers_count() {
	let Engine = Engine(TestClock::New());
	let First = BlockOn(Engine.Claim("page", Span(1_000), Some(Budget { Deadline:None, Operations:1 })))
		.expect("the first claim is admitted");
	assert!(BlockOn(Engine.WithLease(First.clone(), |_Ticket| Box::pin(async { 1 }))).is_ok());
	assert_eq!(
		BlockOn(Engine.WithLease(First.clone(), |_Ticket| Box::pin(async { 2 })))
			.unwrap_err()
			.Code(),
		Code::BudgetExhausted
	);
	BlockOn(Engine.Release(First));
	let Second = BlockOn(Engine.Claim("page", Span(1_000), Some(Budget { Deadline:None, Operations:1 })))
		.expect("the second claim is admitted");
	assert!(
		BlockOn(Engine.WithLease(Second, |_Ticket| Box::pin(async { 3 }))).is_ok(),
		"a retry never inherits a spent count"
	);
}

#[test]
fn a_budgets_deadline_refuses_effects_past_it() {
	let Clock = TestClock::New();
	let Engine = Engine(Clock.clone());
	let Deadline = Clock.Now() + Span(100);
	let Lease = BlockOn(Engine.Claim("page", Span(1_000), Some(Budget { Deadline:Some(Deadline), Operations:10 })))
		.expect("the claim is admitted");
	Clock.Advance(Span(101));
	let Error = BlockOn(Engine.WithLease(Lease, |_Ticket| Box::pin(async { 1 }))).unwrap_err();
	assert_eq!(Error.Code(), Code::BudgetExhausted);
	assert!(Error.Message().contains("deadline passed"));
}

#[test]
fn an_effect_that_fails_still_spends_an_operation() {
	let Engine = Engine(TestClock::New());
	let Lease = BlockOn(Engine.Claim("page", Span(1_000), Some(Budget { Deadline:None, Operations:1 })))
		.expect("the claim is admitted");
	// The runtime never interprets the write: a write that failed is a value,
	// and the charge happened before it ran.
	let Failed = BlockOn(Engine.WithLease(Lease.clone(), |_Ticket| Box::pin(async { Err::<(), ()>(()) })));
	assert_eq!(Failed.expect("the effect was admitted"), Err(()));
	let Spent = BlockOn(Engine.WithLease(Lease, |_Ticket| Box::pin(async { Ok::<(), ()>(()) })));
	assert_eq!(Spent.unwrap_err().Code(), Code::BudgetExhausted);
}

#[test]
fn a_run_with_no_budget_is_unbounded() {
	let Engine = Engine(TestClock::New());
	let Lease = Claimed(&Engine, "page", Span(1_000));
	for _ in 0..25 {
		assert!(BlockOn(Engine.WithLease(Lease.clone(), |_Ticket| Box::pin(async { 1 }))).is_ok());
	}
}

#[test]
fn a_budget_of_no_operations_refuses_every_effect() {
	let Engine = Engine(TestClock::New());
	let Lease = BlockOn(Engine.Claim("page", Span(1_000), Some(Budget { Deadline:None, Operations:0 })))
		.expect("a probe is admitted");
	let Error = BlockOn(Engine.WithLease(Lease, |_Ticket| Box::pin(async { 1 }))).unwrap_err();
	assert_eq!(Error.Code(), Code::BudgetExhausted);
}

#[test]
fn one_runs_budget_is_not_another_resources() {
	let Engine = Engine(TestClock::New());
	let Page = BlockOn(Engine.Claim("page", Span(1_000), Some(Budget { Deadline:None, Operations:1 })))
		.expect("the page claim is admitted");
	let Tab = BlockOn(Engine.Claim("tab", Span(1_000), Some(Budget { Deadline:None, Operations:1 })))
		.expect("the tab claim is admitted");
	assert!(BlockOn(Engine.WithLease(Page.clone(), |_Ticket| Box::pin(async { 1 }))).is_ok());
	assert!(
		BlockOn(Engine.WithLease(Tab, |_Ticket| Box::pin(async { 1 }))).is_ok(),
		"the tab has its own count"
	);
	assert!(BlockOn(Engine.WithLease(Page, |_Ticket| Box::pin(async { 1 }))).is_err());
}

#[test]
fn a_bound_call_stops_its_wait_when_the_caller_cancels() {
	// The one place the combinator itself is exercised: a page that never
	// answers is cut short by the caller, and the answer is a cancellation
	// rather than a timeout, so a consumer can tell its own impatience from
	// a slow page.
	struct Fixed(Instant);
	impl Clock for Fixed {
		fn Now(&self) -> Instant { self.0 }
	}
	struct Waiting;
	impl AgenticRuntime::Pace for Waiting {
		fn Sleep(&self, Span:Duration) -> AgenticRuntime::FutureBox<'_, ()> {
			// A wait that never ends: only the signal can stop this call.
			let _ = Span;
			Box::pin(std::future::pending())
		}
	}
	impl Clock for Waiting {
		fn Now(&self) -> Instant { Instant::now() }
	}

	let Signal = AgenticRuntime::Signal::New();
	let Call = Call::Within(Span(60_000), &Fixed(Instant::now()), Signal.clone());
	let Pace = Waiting;
	let Silent = Box::pin(std::future::pending::<u8>());
	let Bounded = AgenticRuntime::Bound(Silent, &Call, &Pace);
	Signal.Fire();
	assert_eq!(BlockOn(Bounded), Err(AgenticRuntime::Expired::Cancelled));
	assert!(Call.Cancelled());
	assert!(Signal.IsCancelled());
}

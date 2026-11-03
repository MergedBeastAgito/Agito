//! `agentic.runtime`, exercised against a clock the test owns.
//!
//! No tokio, no browser, no network and no sleeping: a lease expires because the
//! test advances a clock, which is the whole reason the clock is a parameter of
//! the port rather than a call to the system's.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use AgenticRuntime::{
	Admission, BlockOn, Budget, Call, Clock, Code, Error, Lease, Options, Policy, Request, Runtime,
	Signal, StateClass, States, Create,
};

/// A clock that only moves when the test moves it.
#[derive(Debug)]
struct TestClock {
	Now: Mutex<Instant>,
}

impl TestClock {
	fn New() -> Arc<Self> {
		Arc::new(Self {
			Now: Mutex::new(Instant::now()),
		})
	}

	fn Advance(&self, Span: Duration) {
		let mut Now = self.Now.lock().unwrap();
		*Now += Span;
	}
}

impl Clock for TestClock {
	fn Now(&self) -> Instant {
		*self.Now.lock().unwrap()
	}
}

/// A runtime bound to generation 7 with two registrations.
fn Engine(Clock: Arc<TestClock>) -> Policy {
	Create(
		Options::New(7)
			.With("chat", StateClass::HoldsTransport)
			.With("verify", StateClass::Stateless)
			.On(Clock),
	)
	.expect("the runtime binds")
}

fn Claim(Policy: &Policy, Resource: &str, Ttl: Duration) -> Result<Lease, Error> {
	BlockOn(Policy.Claim(Resource, Ttl, None))
}

fn Claimed(Policy: &Policy, Resource: &str, Ttl: Duration) -> Lease {
	Claim(Policy, Resource, Ttl).expect("the claim is admitted")
}

fn Span(Milliseconds: u64) -> Duration {
	Duration::from_millis(Milliseconds)
}

/// Poll a future once, without an executor, so a test can observe the state
/// between "admitted" and "settled" instead of only after it finishes.
fn PollOnce<Work: std::future::Future + ?Sized>(Work: std::pin::Pin<&mut Work>) -> bool {
	let Waker = std::task::Waker::noop();
	let mut Context = std::task::Context::from_waker(Waker);
	matches!(
		Work.poll(&mut Context),
		std::task::Poll::Ready(_)
	)
}

#[test]
fn the_port_declares_exactly_the_errors_the_manifest_promises() {
	let Declared: Vec<&str> = Code::All().into_iter().map(Code::Name).collect();
	assert_eq!(
		Declared,
		vec!["admission-refused", "lease-held", "budget-exhausted"]
	);
	let Manifest = include_str!("../module.json");
	for Name in Declared {
		assert!(
			Manifest.contains(&format!("\"{Name}\"")),
			"{Name} is not in module.json"
		);
	}
}

#[test]
fn the_port_declares_exactly_the_state_classes_the_manifest_promises() {
	let Declared: Vec<&str> = States.into_iter().map(StateClass::Name).collect();
	assert_eq!(
		Declared,
		vec![
			"stateless",
			"holds-transport",
			"holds-identity",
			"holds-workspace"
		]
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
		Capability: "chat".to_string(),
		State: StateClass::HoldsTransport,
		Generation: 7,
	}));
	assert_eq!(Answer, Admission::Admitted);
	assert!(Answer.Ok());
}

#[test]
fn refuses_a_generation_it_was_not_bound_to() {
	let Engine = Engine(TestClock::New());
	let Answer = BlockOn(Engine.Admit(Request {
		Capability: "chat".to_string(),
		State: StateClass::HoldsTransport,
		Generation: 8,
	}));
	assert!(!Answer.Ok());
	assert!(Answer.Reason().starts_with("admission-refused: generation 8"));
}

#[test]
fn refuses_a_capability_it_does_not_register() {
	let Engine = Engine(TestClock::New());
	let Answer = BlockOn(Engine.Admit(Request {
		Capability: "browse".to_string(),
		State: StateClass::HoldsTransport,
		Generation: 7,
	}));
	assert!(!Answer.Ok());
	assert!(Answer.Reason().contains("browse is not registered"));
}

#[test]
fn refuses_a_state_class_the_registration_does_not_declare() {
	let Engine = Engine(TestClock::New());
	let Answer = BlockOn(Engine.Admit(Request {
		Capability: "verify".to_string(),
		State: StateClass::HoldsWorkspace,
		Generation: 7,
	}));
	assert!(!Answer.Ok());
	assert!(Answer.Reason().contains("verify declares stateless"));
}

#[test]
fn the_manifest_it_reports_is_the_manifest_it_enforces() {
	let Engine = Engine(TestClock::New());
	let Binding = Engine.Reflect();
	assert_eq!(Binding.Generation, 7);
	let Names: Vec<&str> = Binding
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
}

#[test]
fn a_stale_fence_is_refused_and_the_write_never_lands() {
	let Clock = TestClock::New();
	let Engine = Engine(Clock.clone());
	let Stale = Claimed(&Engine, "page", Span(100));
	Clock.Advance(Span(200));
	let _Current = Claimed(&Engine, "page", Span(100));
	let Landed = AtomicBool::new(false);
	let Outcome = BlockOn(Engine.WithLease(Stale, || {
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
	let First = BlockOn(Engine.WithLease(Lease.clone(), || Box::pin(async { 1 })));
	assert_eq!(First.unwrap(), 1);
	Clock.Advance(Span(101));
	let Second = BlockOn(Engine.WithLease(Lease, || Box::pin(async { 2 })));
	let Error = Second.unwrap_err();
	assert_eq!(Error.Code(), Code::AdmissionRefused);
	assert!(Error.Message().contains("expired"));
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
	assert_eq!(Renewed.ExpiresAt, Lease.ExpiresAt + Span(450));
	Clock.Advance(Span(400));
	let Write = BlockOn(Engine.WithLease(Renewed, || Box::pin(async { 1 })));
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
	let Outcome = BlockOn(Two.WithLease(Foreign, || Box::pin(async { 1 })));
	let Error = Outcome.unwrap_err();
	assert_eq!(Error.Code(), Code::AdmissionRefused);
	assert!(Error.Message().contains("was not the one issued"));
}

#[test]
fn a_claim_during_an_effect_on_an_expired_lease_is_refused() {
	let Clock = TestClock::New();
	let Engine = Engine(Clock.clone());
	let Lease = Claimed(&Engine, "page", Span(100));
	// The effect is admitted, and the lease expires while it is still running.
	let mut Write = Engine.WithLease(Lease.clone(), || Box::pin(std::future::pending::<i32>()));
	assert!(!PollOnce(Write.as_mut()), "the effect is admitted and waiting");
	Clock.Advance(Span(200));
	let Running = Engine.Claim("page", Span(1_000), None);
	let Error = BlockOn(Running).unwrap_err();
	assert_eq!(Error.Code(), Code::LeaseHeld);
	assert!(Error.Message().contains("effect in flight"));
	// Dropping the abandoned effect settles it, which is what frees the resource.
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
}

#[test]
fn releasing_a_superseded_lease_does_not_free_the_new_holders_resource() {
	let Clock = TestClock::New();
	let Engine = Engine(Clock.clone());
	let First = Claimed(&Engine, "page", Span(100));
	Clock.Advance(Span(101));
	let Second = Claimed(&Engine, "page", Span(1_000));
	BlockOn(Engine.Release(First));
	let Write = BlockOn(Engine.WithLease(Second, || Box::pin(async { 1 })));
	assert_eq!(Write.unwrap(), 1, "the new holder still holds it");
}

#[test]
fn release_during_an_effect_frees_the_resource_once_it_settles() {
	let Clock = TestClock::New();
	let Engine = Engine(Clock.clone());
	let Lease = Claimed(&Engine, "page", Span(1_000));
	let mut Write = Engine.WithLease(Lease.clone(), || Box::pin(std::future::pending::<i32>()));
	assert!(!PollOnce(Write.as_mut()), "the effect is admitted and waiting");
	// Released while the write is admitted but not yet settled.
	BlockOn(Engine.Release(Lease));
	// The record survives only to account for the effect, so nothing may take it.
	assert_eq!(
		Claim(&Engine, "page", Span(1_000)).unwrap_err().Code(),
		Code::LeaseHeld
	);
	drop(Write);
	let Next = Claimed(&Engine, "page", Span(1_000));
	assert_eq!(Next.Fence, 2, "the resource came back");
}

#[test]
fn release_never_fails_whatever_it_is_handed() {
	let Engine = Engine(TestClock::New());
	let Foreign = Lease {
		Id: "r999#page#1".to_string(),
		Resource: "page".to_string(),
		Fence: 1,
		ExpiresAt: Instant::now(),
	};
	BlockOn(Engine.Release(Foreign.clone()));
	BlockOn(Engine.Release(Foreign));
}

#[test]
fn a_budget_is_not_inherited_a_new_run_starts_from_the_callers_count() {
	let Engine = Engine(TestClock::New());
	let First = BlockOn(Engine.Claim(
		"page",
		Span(1_000),
		Some(Budget {
			Deadline: None,
			Operations: 1,
		}),
	))
	.expect("the first claim is admitted");
	assert!(BlockOn(Engine.WithLease(First.clone(), || Box::pin(async { 1 }))).is_ok());
	assert_eq!(
		BlockOn(Engine.WithLease(First.clone(), || Box::pin(async { 2 })))
			.unwrap_err()
			.Code(),
		Code::BudgetExhausted
	);
	BlockOn(Engine.Release(First));
	let Second = BlockOn(Engine.Claim(
		"page",
		Span(1_000),
		Some(Budget {
			Deadline: None,
			Operations: 1,
		}),
	))
	.expect("the second claim is admitted");
	assert!(
		BlockOn(Engine.WithLease(Second, || Box::pin(async { 3 }))).is_ok(),
		"a retry never inherits a spent count"
	);
}

#[test]
fn a_budgets_deadline_refuses_effects_past_it() {
	let Clock = TestClock::New();
	let Engine = Engine(Clock.clone());
	let Deadline = Clock.Now() + Span(100);
	let Lease = BlockOn(Engine.Claim(
		"page",
		Span(1_000),
		Some(Budget {
			Deadline: Some(Deadline),
			Operations: 10,
		}),
	))
	.expect("the claim is admitted");
	Clock.Advance(Span(101));
	let Error = BlockOn(Engine.WithLease(Lease, || Box::pin(async { 1 }))).unwrap_err();
	assert_eq!(Error.Code(), Code::BudgetExhausted);
	assert!(Error.Message().contains("deadline passed"));
}

#[test]
fn an_effect_that_fails_still_spends_an_operation() {
	let Engine = Engine(TestClock::New());
	let Lease = BlockOn(Engine.Claim(
		"page",
		Span(1_000),
		Some(Budget {
			Deadline: None,
			Operations: 1,
		}),
	))
	.expect("the claim is admitted");
	// The runtime never interprets the write: a write that failed is a value, and
	// the charge happened before it ran.
	let Failed = BlockOn(Engine.WithLease(Lease.clone(), || Box::pin(async { Err::<(), ()>(()) })));
	assert_eq!(Failed.expect("the effect was admitted"), Err(()));
	let Spent = BlockOn(Engine.WithLease(Lease, || Box::pin(async { Ok::<(), ()>(()) })));
	assert_eq!(Spent.unwrap_err().Code(), Code::BudgetExhausted);
}

#[test]
fn a_run_with_no_budget_is_unbounded() {
	let Engine = Engine(TestClock::New());
	let Lease = Claimed(&Engine, "page", Span(1_000));
	for _ in 0..25 {
		assert!(BlockOn(Engine.WithLease(Lease.clone(), || Box::pin(async { 1 }))).is_ok());
	}
}

#[test]
fn a_budget_of_no_operations_refuses_every_effect() {
	let Engine = Engine(TestClock::New());
	let Lease = BlockOn(Engine.Claim(
		"page",
		Span(1_000),
		Some(Budget {
			Deadline: None,
			Operations: 0,
		}),
	))
	.expect("a probe is admitted");
	let Error = BlockOn(Engine.WithLease(Lease, || Box::pin(async { 1 }))).unwrap_err();
	assert_eq!(Error.Code(), Code::BudgetExhausted);
}

#[test]
fn one_runs_budget_is_not_another_resources() {
	let Engine = Engine(TestClock::New());
	let Page = BlockOn(Engine.Claim(
		"page",
		Span(1_000),
		Some(Budget {
			Deadline: None,
			Operations: 1,
		}),
	))
	.expect("the page claim is admitted");
	let Tab = BlockOn(Engine.Claim(
		"tab",
		Span(1_000),
		Some(Budget {
			Deadline: None,
			Operations: 1,
		}),
	))
	.expect("the tab claim is admitted");
	assert!(BlockOn(Engine.WithLease(Page.clone(), || Box::pin(async { 1 }))).is_ok());
	assert!(
		BlockOn(Engine.WithLease(Tab, || Box::pin(async { 1 }))).is_ok(),
		"the tab has its own count"
	);
	assert!(BlockOn(Engine.WithLease(Page, || Box::pin(async { 1 }))).is_err());
}

#[test]
fn a_bound_call_stops_its_wait_when_the_caller_cancels() {
	// The one place the combinator itself is exercised: a page that never answers
	// is cut short by the caller, and the answer is a cancellation rather than a
	// timeout, so a consumer can tell its own impatience from a slow page.
	struct Fixed(Instant);
	impl Clock for Fixed {
		fn Now(&self) -> Instant {
			self.0
		}
	}
	struct Waiting;
	impl AgenticRuntime::Pace for Waiting {
		fn Sleep(&self, Span: Duration) -> AgenticRuntime::FutureBox<'_, ()> {
			// A wait that never ends: only the signal can stop this call.
			let _ = Span;
			Box::pin(std::future::pending())
		}
	}
	impl Clock for Waiting {
		fn Now(&self) -> Instant {
			Instant::now()
		}
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

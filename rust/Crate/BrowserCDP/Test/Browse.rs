//! `agentic.browse`, exercised against a fake launcher and a clock the test owns.
//!
//! No browser, no network and no sleeping: a launcher that never answers is one
//! whose script says so, and a deadline passes because the test advanced its own
//! clock.

use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use AgenticBrowserCDP::{
	Browse, Code, Create, Error, Launcher, OpenOptions, Options, Ownership, Report, Session, Target,
	Transport,
};
use AgenticRuntime::{BlockOn, Call, Clock, FutureBox, Never, Pace, Signal};
use serde_json::{json, Value};

fn Span(Milliseconds: u64) -> Duration {
	Duration::from_millis(Milliseconds)
}

/// A clock the test owns, and a wait that advances it.
struct TestPace {
	Now: Mutex<Instant>,
	Fire: Mutex<Option<Arc<Signal>>>,
}

impl TestPace {
	fn New() -> Arc<Self> {
		Arc::new(Self {
			Now: Mutex::new(Instant::now()),
			Fire: Mutex::new(None),
		})
	}

	fn Firing(self: &Arc<Self>, Signal: Arc<Signal>) -> Arc<Self> {
		*self.Fire.lock().unwrap() = Some(Signal);
		self.clone()
	}
}

impl Clock for TestPace {
	fn Now(&self) -> Instant {
		*self.Now.lock().unwrap()
	}
}

impl Pace for TestPace {
	fn Sleep(&self, Span: Duration) -> FutureBox<'_, ()> {
		// A pace that is about to fire models a caller giving up, not time passing:
		// it never finishes its wait, so the cancellation is what ends the call.
		if let Some(Signal) = self.Fire.lock().unwrap().take() {
			*self.Now.lock().unwrap() += Duration::from_millis(1);
			Signal.Fire();
			return Box::pin(std::future::pending());
		}
		*self.Now.lock().unwrap() += Span.max(Duration::from_millis(1));
		Box::pin(std::future::ready(()))
	}
}

/// One live page, scripted.
struct FakeTransport {
	Endpoint: String,
	Closed: AtomicBool,
	Evaluations: AtomicU64,
	Closes: AtomicU64,
	Detaches: AtomicU64,
	Answer: Mutex<Result<Value, Error>>,
	Hang: bool,
	Dies: bool,
	Release: Release,
}

#[derive(PartialEq)]
enum Release {
	Quick,
	Hang,
}

impl FakeTransport {
	fn Base(Endpoint: &str) -> Self {
		Self {
			Endpoint: Endpoint.to_string(),
			Closed: AtomicBool::new(false),
			Evaluations: AtomicU64::new(0),
			Closes: AtomicU64::new(0),
			Detaches: AtomicU64::new(0),
			Answer: Mutex::new(Ok(json!(null))),
			Hang: false,
			Dies: false,
			Release: Release::Quick,
		}
	}

	fn New(Endpoint: &str) -> Arc<Self> {
		Arc::new(Self::Base(Endpoint))
	}

	fn Answering(Endpoint: &str, Value: Value) -> Arc<Self> {
		let Transport = Self::New(Endpoint);
		*Transport.Answer.lock().unwrap() = Ok(Value);
		Transport
	}

	fn Failing(Endpoint: &str, Why: Error) -> Arc<Self> {
		let Transport = Self::New(Endpoint);
		*Transport.Answer.lock().unwrap() = Err(Why);
		Transport
	}

	fn Hanging(Endpoint: &str) -> Arc<Self> {
		Arc::new(Self {
			Hang: true,
			..Self::Base(Endpoint)
		})
	}

	fn Dead(Endpoint: &str) -> Arc<Self> {
		let Transport = Self::New(Endpoint);
		Transport.Closed.store(true, Ordering::SeqCst);
		Transport
	}

	/// A transport whose release never finishes.
	fn Stuck(Endpoint: &str) -> Arc<Self> {
		Arc::new(Self {
			Release: Release::Hang,
			..Self::Base(Endpoint)
		})
	}
}

impl Transport for FakeTransport {
	fn Endpoint(&self) -> &str {
		&self.Endpoint
	}

	fn Closed(&self) -> bool {
		self.Closed.load(Ordering::SeqCst)
	}

	fn Evaluate<'a>(
		&'a self,
		_Expression: &'a str,
		_Call: Call,
	) -> FutureBox<'a, Result<Value, Error>> {
		self.Evaluations.fetch_add(1, Ordering::SeqCst);
		let Hang = self.Hang;
		if self.Dies {
			self.Closed.store(true, Ordering::SeqCst);
		}
		let Answer = self.Answer.lock().unwrap().clone();
		Box::pin(async move {
			if Hang {
				std::future::pending::<()>().await;
			}
			Answer
		})
	}

	fn Close(&self) -> FutureBox<'_, ()> {
		self.Closes.fetch_add(1, Ordering::SeqCst);
		let Hang = self.Release == Release::Hang;
		Box::pin(async move {
			if Hang {
				std::future::pending::<()>().await;
			}
			self.Closed.store(true, Ordering::SeqCst);
		})
	}

	fn Detach(&self) -> FutureBox<'_, ()> {
		self.Detaches.fetch_add(1, Ordering::SeqCst);
		let Hang = self.Release == Release::Hang;
		Box::pin(async move {
			if Hang {
				std::future::pending::<()>().await;
			}
			self.Closed.store(true, Ordering::SeqCst);
		})
	}
}

/// A launcher that hands out one scripted transport, or one scripted failure.
struct FakeLauncher {
	Hands: Mutex<Option<Arc<FakeTransport>>>,
	Failure: Mutex<Option<Error>>,
	Hang: bool,
	Launches: AtomicU64,
	Attaches: AtomicU64,
}

impl FakeLauncher {
	fn Base(Transport: Arc<FakeTransport>) -> Self {
		Self {
			Hands: Mutex::new(Some(Transport)),
			Failure: Mutex::new(None),
			Hang: false,
			Launches: AtomicU64::new(0),
			Attaches: AtomicU64::new(0),
		}
	}

	fn Handing(Transport: Arc<FakeTransport>) -> Arc<Self> {
		Arc::new(Self::Base(Transport))
	}

	fn Failing(Why: Error) -> Arc<Self> {
		let Launcher = Self::Handing(FakeTransport::New("ws://none"));
		*Launcher.Failure.lock().unwrap() = Some(Why);
		Launcher
	}

	/// A launcher that never answers.
	fn Hanging() -> Arc<Self> {
		Arc::new(Self {
			Hang: true,
			..Self::Base(FakeTransport::New("ws://none"))
		})
	}

	fn Answer(&self) -> FutureBox<'_, Result<Arc<dyn Transport>, Error>> {
		let Hand = self.Hands.lock().unwrap().clone();
		let Failure = self.Failure.lock().unwrap().clone();
		let Hang = self.Hang;
		Box::pin(async move {
			if Hang {
				std::future::pending::<()>().await;
			}
			if let Some(Why) = Failure {
				return Err(Why);
			}
			match Hand {
				Some(Transport) => Ok(Transport as Arc<dyn Transport>),
				None => Err(Error::Unavailable("nothing to hand out", "")),
			}
		})
	}
}

impl Launcher for FakeLauncher {
	fn Attach<'a>(
		&'a self,
		_Endpoint: &'a str,
		_Url: &'a str,
		_Call: Call,
	) -> FutureBox<'a, Result<Arc<dyn Transport>, Error>> {
		self.Attaches.fetch_add(1, Ordering::SeqCst);
		self.Answer()
	}

	fn Launch<'a>(
		&'a self,
		_Url: &'a str,
		_Call: Call,
	) -> FutureBox<'a, Result<Arc<dyn Transport>, Error>> {
		self.Launches.fetch_add(1, Ordering::SeqCst);
		self.Answer()
	}
}

fn Built(Launcher: Arc<dyn Launcher>, Pace: Arc<TestPace>) -> Session {
	Create(Launcher, Pace, Options::default())
}

fn Reported(Launcher: Arc<dyn Launcher>, Pace: Arc<TestPace>) -> (Session, Arc<Mutex<Vec<String>>>) {
	let Seen: Arc<Mutex<Vec<String>>> = Arc::new(Mutex::new(Vec::new()));
	let Collected = Seen.clone();
	let Report: Report = Arc::new(move |_Target, Why| {
		Collected.lock().unwrap().push(Why.Message().to_string());
	});
	(
		Create(
			Launcher,
			Pace,
			Options {
				Report,
				DisposeBudget: Duration::from_secs(5),
			},
		),
		Seen,
	)
}

fn Call(Pace: &TestPace, Budget: Duration) -> Call {
	Call::Within(Budget, Pace, Arc::new(Never))
}

fn Opened(Session: &Session, Pace: &TestPace, Budget: Duration) -> Result<Target, Error> {
	BlockOn(Session.Open("https://example.test/", OpenOptions::Launch(Call(Pace, Budget))))
}

#[test]
fn the_port_declares_exactly_the_errors_the_manifest_promises() {
	let Declared: Vec<&str> = Code::All().into_iter().map(Code::Name).collect();
	assert_eq!(Declared, vec!["browser-unavailable", "navigation-failed", "timeout"]);
	let Manifest = include_str!("../module.json");
	for Name in Declared {
		assert!(
			Manifest.contains(&format!("\"{Name}\"")),
			"{Name} is not in module.json"
		);
	}
}

#[test]
fn launches_a_browser_when_the_caller_names_no_endpoint() {
	let Pace = TestPace::New();
	let Launcher = FakeLauncher::Handing(FakeTransport::New("ws://launched"));
	let Session = Built(Launcher.clone(), Pace.clone());
	let Target = Opened(&Session, &Pace, Span(2_000)).expect("opened");
	assert_eq!(Target.Ownership, Ownership::Launched);
	assert_eq!(Target.Ws, "ws://launched");
	assert_eq!(Target.Url, "https://example.test/");
	assert_eq!(Launcher.Launches.load(Ordering::SeqCst), 1);
	assert_eq!(Launcher.Attaches.load(Ordering::SeqCst), 0);
}

#[test]
fn attaches_to_a_browser_the_caller_already_owns() {
	let Pace = TestPace::New();
	let Launcher = FakeLauncher::Handing(FakeTransport::New("ws://attached"));
	let Session = Built(Launcher.clone(), Pace.clone());
	let Target = BlockOn(Session.Open(
		"https://example.test/",
		OpenOptions::Attach("ws://attached", Call(&Pace, Span(2_000))),
	))
	.expect("opened");
	assert_eq!(Target.Ownership, Ownership::Attached);
	assert_eq!(Launcher.Attaches.load(Ordering::SeqCst), 1);
	assert_eq!(Launcher.Launches.load(Ordering::SeqCst), 0);
}

#[test]
fn fails_with_navigation_failed_when_the_page_does_not_load() {
	let Pace = TestPace::New();
	let Launcher = FakeLauncher::Failing(Error::Navigation("the page did not load", "404"));
	let Session = Built(Launcher, Pace.clone());
	let Error = Opened(&Session, &Pace, Span(2_000)).expect_err("failed");
	assert_eq!(Error.Code(), Code::Navigation);
	assert_eq!(Error.Detail(), "404");
}

#[test]
fn fails_with_browser_unavailable_when_the_browser_cannot_be_reached() {
	let Pace = TestPace::New();
	let Launcher = FakeLauncher::Failing(Error::Unavailable("connection refused", "ws://nothing"));
	let Session = Built(Launcher, Pace.clone());
	let Error = Opened(&Session, &Pace, Span(2_000)).expect_err("failed");
	assert_eq!(Error.Code(), Code::Unavailable);
}

#[test]
fn fails_with_timeout_when_the_launcher_never_answers() {
	let Pace = TestPace::New();
	let Launcher = FakeLauncher::Hanging();
	let Session = Built(Launcher, Pace.clone());
	let Error = Opened(&Session, &Pace, Span(2_000)).expect_err("failed");
	assert_eq!(Error.Code(), Code::Timeout);
	assert!(Error.Detail().contains("never answered"));
}

#[test]
fn fails_with_timeout_when_the_callers_signal_fires() {
	let Signal = Signal::New();
	let Pace = TestPace::New().Firing(Signal.clone());
	let Launcher = FakeLauncher::Hanging();
	let Session = Built(Launcher, Pace.clone());
	let Call = Call {
		Deadline: Pace.Now() + Span(30_000),
		Cancellation: Signal.clone(),
	};
	let Error = BlockOn(Session.Open("https://example.test/", OpenOptions::Launch(Call)))
		.expect_err("failed");
	assert_eq!(Error.Code(), Code::Timeout);
	assert!(Error.Detail().contains("signal"));
}

#[test]
fn releases_a_transport_that_arrives_already_dead() {
	let Pace = TestPace::New();
	let Transport = FakeTransport::Dead("ws://dead");
	let Launcher = FakeLauncher::Handing(Transport.clone());
	let (Session, Seen) = Reported(Launcher, Pace.clone());
	let Error = Opened(&Session, &Pace, Span(2_000)).expect_err("failed");
	assert_eq!(Error.Code(), Code::Unavailable);
	assert!(Error.Message().contains("already closed"));
	assert_eq!(Transport.Closes.load(Ordering::SeqCst), 1, "it was released");
	let _ = Seen;
}

#[test]
fn returns_the_value_the_page_produced() {
	let Pace = TestPace::New();
	let Transport = FakeTransport::Answering("ws://live", json!({ "count": 2 }));
	let Session = Built(FakeLauncher::Handing(Transport), Pace.clone());
	let Target = Opened(&Session, &Pace, Span(2_000)).expect("opened");
	let Value = BlockOn(Session.Evaluate(&Target, "1 + 1", Call(&Pace, Span(2_000))))
		.expect("evaluated");
	assert_eq!(Value, json!({ "count": 2 }));
}

#[test]
fn fails_with_timeout_when_the_page_never_answers() {
	let Pace = TestPace::New();
	let Transport = FakeTransport::Hanging("ws://live");
	let Session = Built(FakeLauncher::Handing(Transport), Pace.clone());
	let Target = Opened(&Session, &Pace, Span(2_000)).expect("opened");
	let Error = BlockOn(Session.Evaluate(&Target, "1 + 1", Call(&Pace, Span(2_000))))
		.expect_err("failed");
	assert_eq!(Error.Code(), Code::Timeout);
	assert!(Error.Detail().contains("never answered"));
}

#[test]
fn fails_with_timeout_when_the_signal_fires_mid_flight() {
	let Signal = Signal::New();
	// The launcher answers, so the open finishes without a wait; the page never
	// answers, so the first wait is the one that fires.
	let Pace = TestPace::New().Firing(Signal.clone());
	let Transport = FakeTransport::Hanging("ws://live");
	let Session = Built(FakeLauncher::Handing(Transport), Pace.clone());
	let Target = Opened(&Session, &Pace, Span(2_000)).expect("opened");
	let Call = Call {
		Deadline: Pace.Now() + Span(30_000),
		Cancellation: Signal.clone(),
	};
	let Error = BlockOn(Session.Evaluate(&Target, "1 + 1", Call)).expect_err("failed");
	assert_eq!(Error.Code(), Code::Timeout);
	assert!(Error.Detail().contains("signal"));
}

#[test]
fn fails_with_timeout_before_asking_when_the_signal_is_already_aborted() {
	let Pace = TestPace::New();
	let Transport = FakeTransport::Answering("ws://live", json!(1));
	let Session = Built(FakeLauncher::Handing(Transport.clone()), Pace.clone());
	let Target = Opened(&Session, &Pace, Span(2_000)).expect("opened");
	let Signal = Signal::New();
	Signal.Fire();
	let Call = Call {
		Deadline: Pace.Now() + Span(5_000),
		Cancellation: Signal.clone(),
	};
	let Error = BlockOn(Session.Evaluate(&Target, "1 + 1", Call)).expect_err("failed");
	assert_eq!(Error.Code(), Code::Timeout);
	assert!(Error.Detail().contains("signal"));
	assert_eq!(
		Transport.Evaluations.load(Ordering::SeqCst),
		0,
		"the page was never asked"
	);
}

#[test]
fn still_honours_the_deadline_with_a_cancellation_that_never_fires() {
	let Pace = TestPace::New();
	let Transport = FakeTransport::Hanging("ws://live");
	let Session = Built(FakeLauncher::Handing(Transport), Pace.clone());
	let Target = Opened(&Session, &Pace, Span(2_000)).expect("opened");
	let Error = BlockOn(Session.Evaluate(&Target, "1 + 1", Call(&Pace, Span(500))))
		.expect_err("failed");
	assert_eq!(Error.Code(), Code::Timeout);
}

#[test]
fn fails_with_browser_unavailable_when_the_transport_closed_during_the_call() {
	let Pace = TestPace::New();
	// A page error, but the transport died underneath it: the socket is gone, so
	// the page is not what went wrong.
	let Transport = Arc::new(FakeTransport {
		Dies: true,
		..FakeTransport::Base("ws://live")
	});
	*Transport.Answer.lock().unwrap() = Err(Error::Navigation("the page threw", "boom"));
	let Session = Built(FakeLauncher::Handing(Transport.clone()), Pace.clone());
	let Target = Opened(&Session, &Pace, Span(2_000)).expect("opened");
	let Error = BlockOn(Session.Evaluate(&Target, "1 + 1", Call(&Pace, Span(2_000))))
		.expect_err("failed");
	assert_eq!(Error.Code(), Code::Unavailable);
	assert_eq!(Error.Detail(), "boom", "the observation survives");
}

#[test]
fn never_asks_a_transport_it_already_knows_is_closed() {
	let Pace = TestPace::New();
	let Transport = FakeTransport::Answering("ws://live", json!(1));
	let Session = Built(FakeLauncher::Handing(Transport.clone()), Pace.clone());
	let Target = Opened(&Session, &Pace, Span(2_000)).expect("opened");
	Transport.Closed.store(true, Ordering::SeqCst);
	let Error = BlockOn(Session.Evaluate(&Target, "1 + 1", Call(&Pace, Span(2_000))))
		.expect_err("failed");
	assert_eq!(Error.Code(), Code::Unavailable);
	assert_eq!(Transport.Evaluations.load(Ordering::SeqCst), 0);
}

#[test]
fn fails_with_navigation_failed_when_the_page_throws() {
	let Pace = TestPace::New();
	let Transport = FakeTransport::Failing("ws://live", Error::Navigation("the page threw", "boom"));
	let Session = Built(FakeLauncher::Handing(Transport), Pace.clone());
	let Target = Opened(&Session, &Pace, Span(2_000)).expect("opened");
	let Error = BlockOn(Session.Evaluate(&Target, "1 + 1", Call(&Pace, Span(2_000))))
		.expect_err("failed");
	assert_eq!(Error.Code(), Code::Navigation);
	assert_eq!(Error.Detail(), "boom");
}

#[test]
fn fails_with_browser_unavailable_on_a_target_this_session_does_not_hold() {
	let Pace = TestPace::New();
	let Session = Built(FakeLauncher::Handing(FakeTransport::New("ws://live")), Pace.clone());
	let Target = Target {
		Url: "https://example.test/".to_string(),
		Ws: "ws://somebody-elses".to_string(),
		Ownership: Ownership::Attached,
	};
	let Error = BlockOn(Session.Evaluate(&Target, "1 + 1", Call(&Pace, Span(2_000))))
		.expect_err("failed");
	assert_eq!(Error.Code(), Code::Unavailable);
	assert!(Error.Detail().contains("never opened here"));
}

#[test]
fn closes_a_launched_target() {
	let Pace = TestPace::New();
	let Transport = FakeTransport::New("ws://launched");
	let Session = Built(FakeLauncher::Handing(Transport.clone()), Pace.clone());
	let Target = Opened(&Session, &Pace, Span(2_000)).expect("opened");
	BlockOn(Session.Dispose(Target));
	assert_eq!(Transport.Closes.load(Ordering::SeqCst), 1);
	assert_eq!(Transport.Detaches.load(Ordering::SeqCst), 0);
}

#[test]
fn detaches_an_attached_target_without_closing_it() {
	let Pace = TestPace::New();
	let Transport = FakeTransport::New("ws://attached");
	let Session = Built(FakeLauncher::Handing(Transport.clone()), Pace.clone());
	let Target = BlockOn(Session.Open(
		"https://example.test/",
		OpenOptions::Attach("ws://attached", Call(&Pace, Span(2_000))),
	))
	.expect("opened");
	BlockOn(Session.Dispose(Target));
	assert_eq!(Transport.Detaches.load(Ordering::SeqCst), 1);
	assert_eq!(Transport.Closes.load(Ordering::SeqCst), 0, "not ours to close");
}

#[test]
fn is_silent_when_called_twice() {
	let Pace = TestPace::New();
	let Transport = FakeTransport::New("ws://launched");
	let Session = Built(FakeLauncher::Handing(Transport.clone()), Pace.clone());
	let Target = Opened(&Session, &Pace, Span(2_000)).expect("opened");
	BlockOn(Session.Dispose(Target.clone()));
	BlockOn(Session.Dispose(Target));
	assert_eq!(Transport.Closes.load(Ordering::SeqCst), 1, "released once");
}

#[test]
fn reports_a_failing_release_instead_of_returning_it() {
	let Pace = TestPace::New();
	let Transport = FakeTransport::Stuck("ws://launched");
	let (Session, Seen) = Reported(FakeLauncher::Handing(Transport.clone()), Pace.clone());
	let Target = Opened(&Session, &Pace, Span(2_000)).expect("opened");
	BlockOn(Session.Dispose(Target));
	let Seen = Seen.lock().unwrap();
	assert_eq!(Seen.len(), 1);
	assert!(Seen[0].contains("dispose"));
}

#[test]
fn does_not_fail_when_its_own_reporter_panics() {
	let Pace = TestPace::New();
	let Transport = FakeTransport::Stuck("ws://launched");
	let Session = Create(
		FakeLauncher::Handing(Transport),
		Pace.clone(),
		Options {
			Report: Arc::new(|_Target, _Why| panic!("the reporter is broken")),
			DisposeBudget: Duration::from_secs(5),
		},
	);
	let Target = Opened(&Session, &Pace, Span(2_000)).expect("opened");
	BlockOn(Session.Dispose(Target));
}

#[test]
fn reports_a_release_that_hangs_instead_of_waiting_for_it() {
	let Pace = TestPace::New();
	let Transport = FakeTransport::Stuck("ws://launched");
	let (Session, Seen) = Reported(FakeLauncher::Handing(Transport), Pace.clone());
	let Target = Opened(&Session, &Pace, Span(2_000)).expect("opened");
	BlockOn(Session.Dispose(Target));
	assert_eq!(Seen.lock().unwrap().len(), 1);
}

#[test]
fn resolves_for_a_target_this_session_never_opened() {
	let Pace = TestPace::New();
	let Session = Built(FakeLauncher::Handing(FakeTransport::New("ws://live")), Pace.clone());
	BlockOn(Session.Dispose(Target {
		Url: "https://example.test/".to_string(),
		Ws: "ws://never-opened".to_string(),
		Ownership: Ownership::Launched,
	}));
}

#[test]
fn resolves_even_when_handed_a_target_with_no_endpoint() {
	let Pace = TestPace::New();
	let Session = Built(FakeLauncher::Handing(FakeTransport::New("ws://live")), Pace.clone());
	BlockOn(Session.Dispose(Target {
		Url: String::new(),
		Ws: String::new(),
		Ownership: Ownership::Attached,
	}));
}

#[test]
fn leaves_the_target_unusable_afterwards() {
	let Pace = TestPace::New();
	let Transport = FakeTransport::Answering("ws://live", json!(1));
	let Session = Built(FakeLauncher::Handing(Transport.clone()), Pace.clone());
	let Target = Opened(&Session, &Pace, Span(2_000)).expect("opened");
	BlockOn(Session.Dispose(Target.clone()));
	let Error = BlockOn(Session.Evaluate(&Target, "1 + 1", Call(&Pace, Span(2_000))))
		.expect_err("failed");
	assert_eq!(Error.Code(), Code::Unavailable);
	assert_eq!(Transport.Evaluations.load(Ordering::SeqCst), 0);
}

#[test]
fn the_page_error_rule_leaves_a_timeout_alone() {
	// The classification rule itself, without a session: a deadline is the caller's
	// own bound, not a fact about the socket, so a closed transport does not
	// reclassify it.
	let Why = Error::Timeout("the page did not answer", "the deadline passed");
	let Classified = AgenticBrowserCDP::Fn::Classify::ClassifyPage(Why.clone(), true);
	assert_eq!(Classified.Code(), Code::Timeout);
	assert_eq!(Classified, Why);
}

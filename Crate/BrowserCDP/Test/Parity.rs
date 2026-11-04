//! The Rust side of the differential run for `agentic.browse`.
//!
//! These cases are not written here: they are read from
//! `scenarios/browser-cdp.json` and `scenarios/generated/browser-cdp.timing.json`,
//! the same files `Package/browser-cdp/Test/Parity.test.ts` reads. What this
//! file adds is only the fake, the interpreter and the assertion - the fixture,
//! the steps, the expected result and the expected ordered effect list all come
//! from the scenario files, so the two implementations cannot drift by being
//! tested against two hand-written suites.
//!
//! `effects` is the protocol the implementation spoke: every launch, attach,
//! evaluation and release the fake observed, in order. Two ports that returned
//! the same value by different routes disagree here, which is the whole point.
//!
//! `Test/Browse.rs` is kept as it is: that suite is the contract, this one is
//! the comparison. `AGENTIC_PARITY_OUT=<path>` makes this test also write what
//! it observed as JSON, which is how `scenarios/run.mjs --compare` gets the Rust
//! half of the differential run.

#![allow(non_snake_case, non_upper_case_globals)]

use std::{
	collections::BTreeMap,
	sync::{
		Arc, Mutex,
		atomic::{AtomicBool, AtomicU64, Ordering},
	},
	time::{Duration, Instant},
};

use AgenticBrowserCDP::{
	Browse, Create, Error, Launcher, OpenOptions, Options, Ownership, PageWant, Report, Target, Transport,
};
use AgenticRuntime::{BlockOn, Call, Cancellation, Clock, FutureBox, Never, Pace, Signal};
use serde_json::{Map, Value, json};

/// Where every observation the fake made is written, in order.
type Effects = Arc<Mutex<Vec<String>>>;

/// One scripted page connection.
struct FakeTransport {
	Endpoint: String,
	/// Where the scripted page says it is.
	Url: String,
	Closed: AtomicBool,
	/// `answer`, `hang`, `throw` or `dead`.
	Behaviour: Mutex<String>,
	Answer: Mutex<Value>,
	HangOnClose: AtomicBool,
	Closes: AtomicU64,
	Detaches: AtomicU64,
	Asked: Mutex<Vec<String>>,
	Effects: Effects,
}

impl FakeTransport {
	fn New(Fixture: &Value, Effects: Effects) -> Self {
		let Endpoint = string_at(Fixture, &["endpoint"]).unwrap_or_else(|| "ws://none".to_string());
		let State = Fixture.get("transport").cloned().unwrap_or(Value::Null);
		Self {
			Endpoint,
			Url: "https://example.test/".to_string(),
			Closed: AtomicBool::new(bool_at(&State, &["closed"]).unwrap_or(false)),
			Behaviour: Mutex::new(string_at(&State, &["behaviour"]).unwrap_or_else(|| "answer".to_string())),
			Answer: Mutex::new(State.get("answer").cloned().unwrap_or(Value::Null)),
			HangOnClose: AtomicBool::new(bool_at(&State, &["hangOnClose"]).unwrap_or(false)),
			Closes: AtomicU64::new(0),
			Detaches: AtomicU64::new(0),
			Asked: Mutex::new(Vec::new()),
			Effects,
		}
	}

	/// Apply the `before` patch of one step.
	fn Patch(&self, Values: &Value) {
		if let Some(Behaviour) = Values.get("behaviour").and_then(Value::as_str) {
			*self.Behaviour.lock().unwrap() = Behaviour.to_string();
		}
		if let Some(Answer) = Values.get("answer") {
			*self.Answer.lock().unwrap() = Answer.clone();
		}
		if let Some(Closed) = Values.get("closed").and_then(Value::as_bool) {
			self.Closed.store(Closed, Ordering::SeqCst);
		}
		if let Some(Hang) = Values.get("hangOnClose").and_then(Value::as_bool) {
			self.HangOnClose.store(Hang, Ordering::SeqCst);
		}
	}

	fn Note(&self, Effect: String) { self.Effects.lock().unwrap().push(Effect); }
}

impl Transport for FakeTransport {
	fn Endpoint(&self) -> &str { &self.Endpoint }

	fn Url(&self) -> &str { &self.Url }

	fn Closed(&self) -> bool { self.Closed.load(Ordering::SeqCst) }

	fn Evaluate<'a>(&'a self, Expression: &'a str, _Call: Call) -> FutureBox<'a, Result<Value, Error>> {
		self.Note(format!("evaluate:{Expression}"));
		self.Asked.lock().unwrap().push(Expression.to_string());
		let Behaviour = self.Behaviour.lock().unwrap().clone();
		let Endpoint = self.Endpoint.clone();
		if Behaviour == "dead" || self.Closed() {
			// The socket died under the call: the fake reports the state as well
			// as the error, so the session can classify it as a dead transport.
			self.Closed.store(true, Ordering::SeqCst);
			self.Note(format!("refused:evaluate:{Endpoint}"));
			return Box::pin(async move {
				Err(Error::Navigation(
					"WebSocket is already in CLOSING or CLOSED state",
					"the socket died under the call",
				))
			});
		}
		if Behaviour == "throw" {
			return Box::pin(async move {
				Err(Error::Navigation("the page could not answer", "ReferenceError: nothing is defined"))
			});
		}
		if Behaviour == "hang" {
			return Box::pin(std::future::pending::<Result<Value, Error>>());
		}
		let Answer = self.Answer.lock().unwrap().clone();
		Box::pin(async move { Ok(Answer) })
	}

	// `Close` and `Detach` answer `()`, so a release cannot fail - only a hang is
	// reportable, which is why the scenario file marks the one case that needs a
	// *rejecting* release as unported on this side.
	fn Close(&self) -> FutureBox<'_, ()> {
		self.Note(format!("close:{}", self.Endpoint));
		self.Closes.fetch_add(1, Ordering::SeqCst);
		if self.HangOnClose.load(Ordering::SeqCst) {
			return Box::pin(std::future::pending());
		}
		self.Closed.store(true, Ordering::SeqCst);
		Box::pin(std::future::ready(()))
	}

	fn Detach(&self) -> FutureBox<'_, ()> {
		self.Note(format!("detach:{}", self.Endpoint));
		self.Detaches.fetch_add(1, Ordering::SeqCst);
		self.Closed.store(true, Ordering::SeqCst);
		Box::pin(std::future::ready(()))
	}
}

/// A launcher that hands out one scripted transport, or one scripted failure.
struct FakeLauncher {
	Transport: Arc<FakeTransport>,
	/// `answer`, `hang` or `throw`.
	Open: Mutex<String>,
	Failure: Mutex<Option<Value>>,
	DeadOnArrival: AtomicBool,
	Launches: AtomicU64,
	Attaches: AtomicU64,
	Effects: Effects,
}

impl FakeLauncher {
	fn New(Fixture: &Value, Transport: Arc<FakeTransport>, Effects: Effects) -> Self {
		let State = Fixture.get("launcher").cloned().unwrap_or(Value::Null);
		Self {
			Transport,
			Open: Mutex::new(string_at(&State, &["open"]).unwrap_or_else(|| "answer".to_string())),
			Failure: Mutex::new(State.get("failure").cloned().filter(|Value| !Value.is_null())),
			DeadOnArrival: AtomicBool::new(bool_at(&State, &["deadOnArrival"]).unwrap_or(false)),
			Launches: AtomicU64::new(0),
			Attaches: AtomicU64::new(0),
			Effects,
		}
	}

	fn Patch(&self, Values: &Value) {
		if let Some(Open) = Values.get("open").and_then(Value::as_str) {
			*self.Open.lock().unwrap() = Open.to_string();
		}
		if let Some(Failure) = Values.get("failure") {
			*self.Failure.lock().unwrap() = if Failure.is_null() { None } else { Some(Failure.clone()) };
		}
		if let Some(Dead) = Values.get("deadOnArrival").and_then(Value::as_bool) {
			self.DeadOnArrival.store(Dead, Ordering::SeqCst);
		}
	}

	fn Answer(&self) -> FutureBox<'_, Result<Arc<dyn Transport>, Error>> {
		let Open = self.Open.lock().unwrap().clone();
		if Open == "hang" {
			return Box::pin(std::future::pending::<Result<Arc<dyn Transport>, Error>>());
		}
		if Open == "throw" {
			let Failure = self.Failure.lock().unwrap().clone().unwrap_or(Value::Null);
			let Why = failure_of(&Failure);
			return Box::pin(async move { Err(Why) });
		}
		if self.DeadOnArrival.load(Ordering::SeqCst) {
			self.Transport.Closed.store(true, Ordering::SeqCst);
		}
		self.Effects.lock().unwrap().push(format!("handed:{}", self.Transport.Endpoint));
		let Transport: Arc<dyn Transport> = self.Transport.clone();
		Box::pin(async move { Ok(Transport) })
	}
}

impl Launcher for FakeLauncher {
	fn Attach<'a>(
		&'a self,
		Endpoint: &'a str,
		_Want: PageWant<'a>,
		_Call: Call,
	) -> FutureBox<'a, Result<Arc<dyn Transport>, Error>> {
		self.Effects.lock().unwrap().push(format!("attach:{Endpoint}"));
		self.Attaches.fetch_add(1, Ordering::SeqCst);
		self.Answer()
	}

	fn Launch<'a>(&'a self, Url: &'a str, _Call: Call) -> FutureBox<'a, Result<Arc<dyn Transport>, Error>> {
		self.Effects.lock().unwrap().push(format!("launch:{Url}"));
		self.Launches.fetch_add(1, Ordering::SeqCst);
		self.Answer()
	}
}

/// The error a scripted launcher failure describes.
fn failure_of(Failure: &Value) -> Error {
	let Message = string_at(Failure, &["message"]).unwrap_or_else(|| "the browser failed".to_string());
	let Detail = string_at(Failure, &["detail"]).unwrap_or_default();
	match string_at(Failure, &["code"]).unwrap_or_else(|| "unavailable".to_string()).as_str() {
		"navigation" => Error::Navigation(Message, Detail),
		"timeout" => Error::Timeout(Message, Detail),
		_ => Error::Unavailable(Message, Detail),
	}
}

/// A clock the test owns, and a wait that advances it.
///
/// A pace that is about to fire models a caller giving up, not time passing: it
/// never finishes its wait, so the cancellation is what ends the call. That is
/// what makes `signal: "after:N"` deterministic without a real timer.
struct TestPace {
	Now: Mutex<Instant>,
	Fire: Mutex<Option<Arc<Signal>>>,
}

impl TestPace {
	fn New() -> Arc<Self> { Arc::new(Self { Now: Mutex::new(Instant::now()), Fire: Mutex::new(None) }) }

	fn Firing(self: &Arc<Self>, Signal: Arc<Signal>) -> Arc<Self> {
		*self.Fire.lock().unwrap() = Some(Signal);
		self.clone()
	}
}

impl Clock for TestPace {
	fn Now(&self) -> Instant { *self.Now.lock().unwrap() }
}

impl Pace for TestPace {
	fn Sleep(&self, Span: Duration) -> FutureBox<'_, ()> {
		if let Some(Signal) = self.Fire.lock().unwrap().take() {
			*self.Now.lock().unwrap() += Duration::from_millis(1);
			Signal.Fire();
			return Box::pin(std::future::pending());
		}
		*self.Now.lock().unwrap() += Span.max(Duration::from_millis(1));
		Box::pin(std::future::ready(()))
	}
}

/// Read a dotted path out of a JSON value.
fn at<'a>(Document: &'a Value, Path: &[&str]) -> Option<&'a Value> {
	let mut Cursor = Document;
	for Step in Path {
		Cursor = Cursor.get(*Step)?;
	}
	Some(Cursor)
}

fn string_at(Document: &Value, Path: &[&str]) -> Option<String> {
	at(Document, Path).and_then(Value::as_str).map(str::to_string)
}

fn bool_at(Document: &Value, Path: &[&str]) -> Option<bool> { at(Document, Path).and_then(Value::as_bool) }

fn number_at(Document: &Value, Path: &[&str]) -> Option<u64> { at(Document, Path).and_then(Value::as_u64) }

/// Merge a patch into a fixture, one level deep per section, recursively. A
/// missing patch is not a patch: it leaves the fixture alone.
fn merge(Base: &Value, Patch: &Value) -> Value {
	if Patch.is_null() {
		return Base.clone();
	}
	match (Base, Patch) {
		(Value::Object(Base), Value::Object(Patch)) => {
			let mut Merged = Base.clone();
			for (Key, Patched) in Patch {
				let Next = match Merged.get(Key) {
					Some(Existing) => merge(Existing, Patched),
					None => Patched.clone(),
				};
				Merged.insert(Key.clone(), Next);
			}
			Value::Object(Merged)
		}
		_ => Patch.clone(),
	}
}

/// The scenario files this crate answers to.
fn scenario_files() -> Vec<Value> {
	let Base = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../scenarios");
	["browser-cdp.json", "generated/browser-cdp.timing.json"]
		.iter()
		.map(|Name| Base.join(Name))
		.filter(|Path| Path.exists())
		.filter_map(|Path| std::fs::read_to_string(Path).ok())
		.filter_map(|Text| serde_json::from_str::<Value>(&Text).ok())
		.collect()
}

/// A case this side cannot express is skipped, and the file says why.
fn skipped_on_rust(Entry: &Value) -> bool {
	Entry
		.get("skip")
		.and_then(Value::as_array)
		.map(|Sides| Sides.iter().any(|Side| Side.as_str() == Some("rust")))
		.unwrap_or(false)
}

/// One step's bound: its budget, and whatever cancellation the step names.
fn call_for(Step: &Value, Pace: &Arc<TestPace>, Cancellation: Arc<dyn Cancellation>) -> Call {
	let Budget = Duration::from_millis(number_at(Step, &["timeoutMs"]).unwrap_or(40));
	Call::Within(Budget, Pace.as_ref(), Cancellation)
}

fn target_of(Document: &Value) -> Target {
	Target {
		Url: string_at(Document, &["url"]).unwrap_or_default(),
		Ws: string_at(Document, &["ws"]).unwrap_or_default(),
		Ownership: match string_at(Document, &["ownership"]).unwrap_or_else(|| "launched".to_string()).as_str() {
			"attached" => Ownership::Attached,
			_ => Ownership::Launched,
		},
	}
}

fn error_of(Why: &Error) -> Value {
	json!({
		"kind": "error",
		"code": Why.Code().Name(),
		"message": Why.Message(),
		"detail": Why.Detail(),
	})
}

/// Run one scenario, and answer what it did: the result, the ordered effects and
/// how many failures were reported.
fn run_case(Fixture: &Value, Entry: &Value) -> Value {
	let Fixture = merge(Fixture, Entry.get("fixture").unwrap_or(&Value::Null));
	let Effects: Effects = Arc::new(Mutex::new(Vec::new()));
	let Reports = Arc::new(AtomicU64::new(0));
	let ReportMode = Arc::new(Mutex::new(
		string_at(&Fixture, &["session", "report"]).unwrap_or_else(|| "record".to_string()),
	));
	let DisposeBudget = Duration::from_millis(number_at(&Fixture, &["session", "disposeTimeoutMs"]).unwrap_or(40));

	let Transport = Arc::new(FakeTransport::New(&Fixture, Effects.clone()));
	let Launcher = Arc::new(FakeLauncher::New(&Fixture, Transport.clone(), Effects.clone()));

	let Steps = Entry.get("steps").and_then(Value::as_array).cloned().unwrap_or_default();
	// A step that asks to be cancelled mid-operation needs a pace that fires, and
	// the pace fires on its first wait - which is that operation's guard, because
	// every step before it answers without waiting.
	let Wanted = Steps
		.iter()
		.filter_map(|Step| Step.get("signal").and_then(Value::as_str))
		.find(|Kind| Kind.starts_with("after:") || *Kind == "aborted");
	let Shared = Signal::New();
	let Pace = match Wanted {
		Some("aborted") => {
			Shared.Fire();
			TestPace::New()
		}
		Some(_) => TestPace::New().Firing(Shared.clone()),
		None => TestPace::New(),
	};

	let Report: Report = {
		let Effects = Effects.clone();
		let Reports = Reports.clone();
		let Mode = ReportMode.clone();
		Arc::new(move |Target: &Target, _Why: &Error| {
			Effects.lock().unwrap().push(format!("report:{}", Target.Ws));
			Reports.fetch_add(1, Ordering::SeqCst);
			if Mode.lock().unwrap().as_str() == "throw" {
				panic!("the reporter is broken too");
			}
		})
	};

	let Session = Create(Launcher.clone(), Pace.clone(), Options { Report, DisposeBudget });

	let mut Held: Option<Target> = None;
	let mut Result = Value::Null;

	for Step in &Steps {
		if let Some(Before) = Step.get("before") {
			if let Some(Values) = Before.get("transport") {
				Transport.Patch(Values);
			}
			if let Some(Values) = Before.get("launcher") {
				Launcher.Patch(Values);
			}
			if let Some(Values) = Before.get("session") {
				if let Some(Mode) = Values.get("report").and_then(Value::as_str) {
					*ReportMode.lock().unwrap() = Mode.to_string();
				}
			}
		}

		let Cancellation: Arc<dyn Cancellation> = match Step.get("signal").and_then(Value::as_str) {
			Some(Kind) if Kind == "aborted" || Kind.starts_with("after:") => Shared.clone(),
			_ => Arc::new(Never),
		};

		match string_at(Step, &["op"]).unwrap_or_default().as_str() {
			"open" => {
				let Url = string_at(Step, &["url"]).unwrap_or_default();
				let Bound = call_for(Step, &Pace, Cancellation);
				let Options = match string_at(Step, &["attachTo"]) {
					Some(Endpoint) => OpenOptions::Attach(Endpoint, Bound),
					None => OpenOptions::Launch(Bound),
				};
				match BlockOn(Session.Open(&Url, Options)) {
					Ok(Target) => {
						Result = json!({
							"kind": "target",
							"ownership": Target.Ownership.Name(),
							"url": Target.Url,
							"ws": Target.Ws,
						});
						Held = Some(Target);
					}
					Err(Why) => {
						Result = error_of(&Why);
						break;
					}
				}
			}
			"evaluate" => {
				let One = match Step.get("target") {
					Some(Value::Object(_)) => Some(target_of(Step.get("target").unwrap_or(&Value::Null))),
					_ => Held.clone(),
				};
				let Some(One) = One else {
					Result = json!({ "kind": "error", "code": "browser-unavailable", "message": "no target" });
					break;
				};
				let Expression = string_at(Step, &["expression"]).unwrap_or_default();
				let Bound = call_for(Step, &Pace, Cancellation);
				match BlockOn(Session.Evaluate(&One, &Expression, Bound)) {
					Ok(Produced) => Result = json!({ "kind": "value", "value": Produced }),
					Err(Why) => {
						Result = error_of(&Why);
						break;
					}
				}
			}
			"dispose" => {
				let One = match Step.get("target") {
					Some(Value::Object(_)) => Some(target_of(Step.get("target").unwrap_or(&Value::Null))),
					_ => Held.clone(),
				};
				let Some(One) = One else {
					Result = json!({ "kind": "void" });
					continue;
				};
				for _ in 0..number_at(Step, &["times"]).unwrap_or(1) {
					BlockOn(Session.Dispose(One.clone()));
				}
				Result = json!({ "kind": "void" });
			}
			Other => panic!("unknown browse step {Other}"),
		}
	}

	json!({
		"result": Result,
		"effects": Effects.lock().unwrap().clone(),
		"reports": Reports.load(Ordering::SeqCst),
	})
}

/// What one observation got wrong, as sentences. Empty means the case agreed.
fn mismatches(Observed: &Value, Expect: &Value) -> Vec<String> {
	let mut Wrong = Vec::new();
	let Want = Expect.get("result").cloned().unwrap_or(Value::Null);
	let Got = Observed.get("result").cloned().unwrap_or(Value::Null);
	let Kind = |Document: &Value| Document.get("kind").and_then(Value::as_str).unwrap_or("").to_string();
	if Kind(&Want) != Kind(&Got) {
		Wrong.push(format!("result kind: wanted {}, got {}", Kind(&Want), Kind(&Got)));
	} else if Kind(&Want) == "error" {
		if Want.get("code") != Got.get("code") {
			Wrong.push(format!("result code: wanted {:?}, got {:?}", Want.get("code"), Got.get("code")));
		}
		if let Some(Message) = Want.get("message").and_then(Value::as_str) {
			if !Got.get("message").and_then(Value::as_str).unwrap_or("").contains(Message) {
				Wrong.push(format!("result message: wanted to contain {Message:?}, got {:?}", Got.get("message")));
			}
		}
		if let Some(Token) = Want.get("observed").and_then(Value::as_str) {
			let Both = format!(
				"{} {}",
				Got.get("message").and_then(Value::as_str).unwrap_or(""),
				Got.get("detail").and_then(Value::as_str).unwrap_or("")
			);
			if !Both.contains(Token) {
				Wrong.push(format!("result observation: wanted {Token:?} in message or detail, got {Both:?}"));
			}
		}
	} else if Kind(&Want) == "target" {
		for Field in ["ownership", "url", "ws"] {
			if Want.get(Field) != Got.get(Field) {
				Wrong.push(format!("result {Field}: wanted {:?}, got {:?}", Want.get(Field), Got.get(Field)));
			}
		}
	} else if Kind(&Want) == "value" && Want.get("value") != Got.get("value") {
		Wrong.push(format!("result value: wanted {:?}, got {:?}", Want.get("value"), Got.get("value")));
	}

	if let Some(Effects) = Expect.get("effects") {
		if Effects != Observed.get("effects").unwrap_or(&Value::Null) {
			Wrong.push(format!("effects: wanted {Effects}, got {}", Observed.get("effects").unwrap_or(&Value::Null)));
		}
	}
	if let Some(Reports) = Expect.get("reports").and_then(Value::as_u64) {
		if Some(Reports) != Observed.get("reports").and_then(Value::as_u64) {
			Wrong.push(format!("reports: wanted {Reports}, got {:?}", Observed.get("reports")));
		}
	}
	Wrong
}

/// Every scenario, against its expectation, with what was observed left behind
/// for the differential run to compare against the TypeScript half.
#[test]
fn every_scenario_agrees_with_its_expectation() {
	let mut Observed: BTreeMap<String, Value> = BTreeMap::new();
	let mut Wrong: Vec<String> = Vec::new();
	let mut Seen = 0usize;

	for File in scenario_files() {
		let Fixture = File.get("fixture").cloned().unwrap_or(Value::Null);
		let Cases = File.get("cases").and_then(Value::as_array).cloned().unwrap_or_default();
		for Entry in Cases {
			if skipped_on_rust(&Entry) {
				continue;
			}
			let Id = string_at(&Entry, &["id"]).unwrap_or_else(|| "<unnamed>".to_string());
			let One = run_case(&Fixture, &Entry);
			let Expect = Entry.get("expect").cloned().unwrap_or(Value::Null);
			for Sentence in mismatches(&One, &Expect) {
				Wrong.push(format!("{Id}: {Sentence}"));
			}
			Observed.insert(Id, One);
			Seen += 1;
		}
	}

	if let Ok(Path) = std::env::var("AGENTIC_PARITY_OUT") {
		let Document =
			serde_json::to_string_pretty(&Value::Object(Observed.clone().into_iter().collect::<Map<String, Value>>()))
				.unwrap_or_default();
		let _ = std::fs::write(Path, Document);
	}

	assert!(Seen > 0, "no scenario files were found");
	assert!(
		Wrong.is_empty(),
		"the Rust port disagrees with the scenario files:\n{}",
		Wrong.join("\n")
	);
}

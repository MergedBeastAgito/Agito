//! The Rust side of the differential run for `agentic.chat.ask`.
//!
//! These cases are not written here: they are read from
//! `scenarios/chat-driver.json` and `scenarios/generated/chat-driver.timing.json`,
//! the same files `packages/chat-driver/Test/Parity.test.ts` reads. What this
//! file adds is only the fake, the interpreter and the assertion - the fixture,
//! the steps, the expected result and the expected ordered effect list all come
//! from the scenario files, so the two implementations cannot drift by being
//! tested against two hand-written suites.
//!
//! `effects` is the protocol the implementation spoke: every open, evaluation,
//! write, send, release and report the fake observed, in order. Two ports that
//! returned the same value by different routes disagree here, which is the whole
//! point.
//!
//! The fake answers by *kind*, and it decides the kind by calling the same
//! functions the driver calls (`Expressions::ReadMode` and friends), so a change
//! to the protocol breaks the fake rather than passing silently. What it answers
//! is the value the page's expression would have produced - a mode label folded
//! to lower case, a model label trimmed - because that is what the TypeScript
//! half sees when it evaluates the same expression against a page-shaped object.
//!
//! Time is the test's: the pace below is the one `BrowserCDP/tests/parity.rs`
//! uses. A pace that is about to fire models a caller giving up, not time
//! passing - it fires the signal on its first wait and never finishes that wait,
//! so the cancellation is what ends the call and no timer is involved.
//!
//! `Test/Chat.rs` is kept as it is: that suite is the contract, this one is the
//! comparison. `AGENTIC_PARITY_OUT=<path>` makes this test also write what it
//! observed as JSON, which is how `scenarios/run.mjs --compare` gets the Rust
//! half of the differential run.

#![allow(non_snake_case, non_upper_case_globals)]

use std::{
	collections::BTreeMap,
	sync::{
		Arc, Mutex,
		atomic::{AtomicU64, Ordering},
	},
	time::{Duration, Instant},
};

use AgenticChatDriver::{
	Chat, Create, DefaultSelectors, Error, Expressions, Failure, Mode, Options, Page, Question,
	Selectors, Target,
};
use AgenticRuntime::{BlockOn, Call, Cancellation, Clock, FutureBox, Never, Pace, Signal};
use serde_json::{Map, Value, json};

/// Where every observation the fake made is written, in order.
type Effects = Arc<Mutex<Vec<String>>>;

/// Which of the driver's expressions the fake was handed.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Kind {
	Mode,
	Press,
	Model,
	Composer,
	Submit,
	Answers,
	Collapse,
	Trace,
	Sources,
	Unknown,
}

impl Kind {
	/// The name the scenario's effect vocabulary uses.
	fn Name(self) -> &'static str {
		match self {
			Kind::Mode => "mode",
			Kind::Press => "press",
			Kind::Model => "model",
			Kind::Composer => "composer",
			Kind::Submit => "submit",
			Kind::Answers => "answers",
			Kind::Collapse => "collapse",
			Kind::Trace => "trace",
			Kind::Sources => "sources",
			Kind::Unknown => "unknown",
		}
	}
}

/// The expression's identity, decided by the driver's own functions.
fn KindOf(Selectors: &Selectors, Question: &str, Expression: &str) -> Kind {
	if Expression == Expressions::ReadMode(Selectors) {
		Kind::Mode
	} else if Expression == Expressions::PressMode(Selectors) {
		Kind::Press
	} else if Expression == Expressions::ReadModel(Selectors) {
		Kind::Model
	} else if Expression == Expressions::TypeQuestion(Selectors, Question) {
		Kind::Composer
	} else if Expression == Expressions::PressSubmit(Selectors, Question) {
		Kind::Submit
	} else if Expression == Expressions::ReadAnswers(Selectors) {
		Kind::Answers
	} else if Expression == Expressions::ReadCollapse(Selectors) {
		Kind::Collapse
	} else if Expression == Expressions::ReadTrace(Selectors) {
		Kind::Trace
	} else if Expression == Expressions::ReadSources(Selectors) {
		Kind::Sources
	} else {
		Kind::Unknown
	}
}

/// A page, scripted by the fixture.
///
/// The script is the fixture's `page` object, kept whole, so a step's `before`
/// patch is the same one-level merge the TypeScript half applies. Every field is
/// flat, and `answersHangFrom` / `answersFailFrom` count *every* read of the
/// thread - the correlation read the driver takes before the send is read one -
/// so both halves index the same sequence.
struct FakePage {
	Page: Mutex<Value>,
	Question: String,
	Selectors: Selectors,
	Effects: Effects,
	/// Every expression kind the driver asked for, in order.
	Asked: Mutex<Vec<Kind>>,
	Opens: AtomicU64,
	Releases: AtomicU64,
	Presses: AtomicU64,
	Reads: AtomicU64,
}

impl FakePage {
	fn New(Fixture: &Value, Effects: Effects) -> Arc<Self> {
		Arc::new(Self {
			Page: Mutex::new(Fixture.get("page").cloned().unwrap_or_else(|| json!({}))),
			Question: string_at(Fixture, &["question", "text"]).unwrap_or_default(),
			Selectors: DefaultSelectors(),
			Effects,
			Asked: Mutex::new(Vec::new()),
			Opens: AtomicU64::new(0),
			Releases: AtomicU64::new(0),
			Presses: AtomicU64::new(0),
			Reads: AtomicU64::new(0),
		})
	}

	/// The script as it stands. Cloned rather than borrowed, so no lock is held
	/// across an await.
	fn State(&self) -> Value { self.Page.lock().unwrap().clone() }

	/// Apply the `page` half of a step's `before` patch, one level deep.
	fn Patch(&self, Values: &Value) {
		let mut Current = self.Page.lock().unwrap();
		let (Some(Current), Some(Patch)) = (Current.as_object_mut(), Values.as_object()) else {
			return;
		};
		for (Key, Value) in Patch {
			Current.insert(Key.clone(), Value.clone());
		}
	}

	fn Note(&self, Effect: String) { self.Effects.lock().unwrap().push(Effect); }

	/// The thread on its `Read`th read, counting from zero.
	fn ThreadAt(&self, State: &Value, Read: u64) -> Value {
		if bool_at(State, &["answersStall"]).unwrap_or(false) {
			return json!({ "count": 1, "text": format!("still writing {Read}") });
		}
		let Reads = State.get("answersReads").and_then(Value::as_array);
		match Reads {
			None => json!({ "count": 0, "text": "" }),
			Some(Reads) if Reads.is_empty() => json!({ "count": 0, "text": "" }),
			Some(Reads) => {
				let At = (Read as usize).min(Reads.len() - 1);
				Reads.get(At).cloned().unwrap_or(Value::Null)
			}
		}
	}
}

impl Page for FakePage {
	fn Open<'a>(&'a self, Url: &'a str, _Call: Call) -> FutureBox<'a, Result<Target, Failure>> {
		let State = self.State();
		self.Note(format!("open:{Url}"));
		match string_at(&State, &["open"]).unwrap_or_else(|| "answer".to_string()).as_str() {
			"hang" => return Box::pin(std::future::pending::<Result<Target, Failure>>()),
			"throw" => {
				let Message = string_at(&State, &["openFailure"]).unwrap_or_default();
				self.Note(format!("failed:open:{Message}"));
				return Box::pin(async move { Err(Failure::Of(Message)) });
			}
			_ => {}
		}
		self.Opens.fetch_add(1, Ordering::SeqCst);
		self.Note(format!("opened:{Url}"));
		Box::pin(async move { Ok(Target { Url: Url.to_string() }) })
	}

	fn Evaluate<'a>(
		&'a self,
		_Target: &'a Target,
		Expression: &'a str,
		_Call: Call,
	) -> FutureBox<'a, Result<Value, Failure>> {
		let State = self.State();
		let Kind = KindOf(&self.Selectors, &self.Question, Expression);
		self.Asked.lock().unwrap().push(Kind);
		self.Note(format!("evaluate:{}", Kind.Name()));

		if string_at(&State, &["evaluate"]).unwrap_or_else(|| "answer".to_string()) == "hang" {
			return Box::pin(std::future::pending::<Result<Value, Failure>>());
		}
		if string_at(&State, &["evaluate"]).unwrap_or_else(|| "answer".to_string()) == "throw" {
			let Message = string_at(&State, &["evaluateFailure"]).unwrap_or_default();
			self.Note(format!("failed:{}:{Message}", Kind.Name()));
			return Box::pin(async move { Err(Failure::Of(Message)) });
		}

		let Answer = match Kind {
			// A page that cannot name its mode is answered as the script says it,
			// whatever shape that is; a page that can is answered with the label
			// the expression would have folded.
			Kind::Mode => {
				let Label = if self.Presses.load(Ordering::SeqCst) == 0 {
					State.get("mode").cloned().unwrap_or(Value::Null)
				} else {
					State.get("modeAfterPress").cloned().unwrap_or(Value::Null)
				};
				match Label.as_str() {
					Some(Text) => json!(Text.to_lowercase()),
					None => Label,
				}
			}
			Kind::Press => {
				self.Presses.fetch_add(1, Ordering::SeqCst);
				json!(true)
			}
			Kind::Model => match State.get("model").and_then(Value::as_str) {
				Some(Label) => json!(Label.trim()),
				None => State.get("model").cloned().unwrap_or(Value::Null),
			},
			Kind::Composer => {
				if string_at(&State, &["composer"]).unwrap_or_else(|| "textarea".to_string())
					== "missing"
				{
					json!(false)
				} else {
					self.Note(format!("written:{}", self.Question));
					json!(true)
				}
			}
			Kind::Submit => {
				let Outcome =
					string_at(&State, &["submit"]).unwrap_or_else(|| "sent".to_string());
				match Outcome.as_str() {
					"sent" => self.Note("sent".to_string()),
					"disabled" => self.Note("refused:submit:disabled".to_string()),
					_ => {}
				}
				json!(Outcome)
			}
			Kind::Answers => {
				let Read = self.Reads.fetch_add(1, Ordering::SeqCst);
				let At = Read + 1;
				if number_at(&State, &["answersHangFrom"]) == Some(At) {
					return Box::pin(std::future::pending::<Result<Value, Failure>>());
				}
				if number_at(&State, &["answersFailFrom"]) == Some(At) {
					let Message = string_at(&State, &["answersFailMessage"]).unwrap_or_default();
					self.Note(format!("failed:answers:{Message}"));
					return Box::pin(async move { Err(Failure::Of(Message)) });
				}
				self.ThreadAt(&State, Read)
			}
			Kind::Collapse => match State.get("collapse") {
				Some(Value::String(Label)) => json!(Label.trim()),
				Some(Other) => Other.clone(),
				None => Value::Null,
			},
			Kind::Trace => State.get("trace").cloned().unwrap_or_else(|| json!([])),
			Kind::Sources => State.get("sources").cloned().unwrap_or_else(|| json!([])),
			Kind::Unknown => Value::Null,
		};

		Box::pin(async move { Ok(Answer) })
	}

	fn Dispose<'a>(&'a self, Target: Target) -> FutureBox<'a, ()> {
		self.Releases.fetch_add(1, Ordering::SeqCst);
		self.Note(format!("release:{}", Target.Url));
		let Hang = string_at(&self.State(), &["release"]).unwrap_or_else(|| "answer".to_string())
			== "hang";
		Box::pin(async move {
			if Hang {
				std::future::pending::<()>().await;
			}
		})
	}
}

/// A clock the test owns, and a wait that advances it.
///
/// A pace that is about to fire models a caller giving up, not time passing: it
/// fires the signal on its first wait and never finishes that wait, so the
/// cancellation is what ends the call. That is what makes `signal: "after:N"`
/// deterministic without a real timer. The first wait a chat call reaches is the
/// guard of the first operation that has not answered yet - every operation
/// before it is ready on its first poll, and a ready operation never waits.
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

/// Merge a patch into a fixture the way `scenarios/lib/load.mjs` does: section by
/// section, one level deep. A patch that is not an object replaces the section,
/// and a section that is not an object is replaced rather than merged into.
fn merge(Base: &Value, Patch: &Value) -> Value {
	if Patch.is_null() {
		return Base.clone();
	}
	let (Some(BaseObject), Some(PatchObject)) = (Base.as_object(), Patch.as_object()) else {
		return Patch.clone();
	};
	let mut Merged = BaseObject.clone();
	for (Key, Patched) in PatchObject {
		let Next = match Patched.as_object() {
			Some(Values) => {
				let mut Section = Merged
					.get(Key)
					.and_then(Value::as_object)
					.cloned()
					.unwrap_or_default();
				for (Field, Value) in Values {
					Section.insert(Field.clone(), Value.clone());
				}
				Value::Object(Section)
			}
			None => Patched.clone(),
		};
		Merged.insert(Key.clone(), Next);
	}
	Value::Object(Merged)
}

/// The scenario files this crate answers to.
fn scenario_files() -> Vec<Value> {
	let Base = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../scenarios");
	["chat-driver.json", "generated/chat-driver.timing.json"]
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

fn error_of(Why: &Error) -> Value {
	json!({
		"kind": "error",
		"code": Why.Code().Name(),
		"message": Why.Message(),
		"detail": Why.Detail(),
	})
}

fn answer_of(Answer: &AgenticChatDriver::Answer) -> Value {
	json!({
		"kind": "answer",
		"text": Answer.Text,
		"trace": Answer.Trace.iter().map(|Step| json!({
			"depth": Step.Depth,
			"detail": Step.Detail,
			"label": Step.Label,
		})).collect::<Vec<Value>>(),
		"sources": Answer.Sources.iter().map(|Source| json!({
			"label": Source.Label,
			"url": Source.Url,
		})).collect::<Vec<Value>>(),
		"model": Answer.Model,
		"mode": Answer.Mode,
	})
}

/// Run one scenario, and answer what it did: the result, the ordered effects and
/// how many failures were reported.
fn run_case(Fixture: &Value, Entry: &Value) -> Value {
	let Fixture = merge(Fixture, Entry.get("fixture").unwrap_or(&Value::Null));
	let Effects: Effects = Arc::new(Mutex::new(Vec::new()));
	let Reports = Arc::new(AtomicU64::new(0));

	let Page = FakePage::New(&Fixture, Effects.clone());
	let Signal = Signal::New();
	let Wanted = string_at(&Fixture, &["driver", "signal"]).unwrap_or_else(|| "fresh".to_string());
	let Pace = match Wanted.as_str() {
		"aborted" => {
			Signal.Fire();
			TestPace::New()
		}
		Kind if Kind.starts_with("after:") => TestPace::New().Firing(Signal.clone()),
		_ => TestPace::New(),
	};

	let Report: Arc<dyn Fn(&Target, &Error) + Send + Sync> = {
		let Effects = Effects.clone();
		let Reports = Reports.clone();
		Arc::new(move |_Target: &Target, Why: &Error| {
			Effects
				.lock()
				.unwrap()
				.push(format!("report:{}", Why.Code().Name()));
			Reports.fetch_add(1, Ordering::SeqCst);
		})
	};

	let Driver = Create(
		Page.clone(),
		Options {
			Url: string_at(&Fixture, &["url"]).unwrap_or_default(),
			Selectors: DefaultSelectors(),
			Poll: Duration::from_millis(number_at(&Fixture, &["driver", "pollMs"]).unwrap_or(0)),
			DisposeBudget: Duration::from_millis(
				number_at(&Fixture, &["driver", "disposeTimeoutMs"]).unwrap_or(40),
			),
		},
		Pace.clone(),
	)
	.Reporting(Report);

	let Budget = Duration::from_millis(number_at(&Fixture, &["driver", "timeoutMs"]).unwrap_or(400));
	let Mode = match string_at(&Fixture, &["question", "mode"]).unwrap_or_else(|| "search".to_string()).as_str() {
		"search" => Mode::Search,
		Other => Mode::Other(Other.to_string()),
	};
	let Question = Question {
		Text: string_at(&Fixture, &["question", "text"]).unwrap_or_default(),
		Mode,
		Model: string_at(&Fixture, &["question", "model"]).unwrap_or_default(),
	};

	let Steps = Entry.get("steps").and_then(Value::as_array).cloned().unwrap_or_default();
	let mut Result = json!({ "kind": "void" });

	for Step in &Steps {
		if let Some(Before) = Step.get("before") {
			if let Some(Values) = Before.get("page") {
				Page.Patch(Values);
			}
		}
		let Op = string_at(Step, &["op"]).unwrap_or_default();
		assert_eq!(Op, "ask", "unknown chat step {Op}");

		let Cancellation: Arc<dyn Cancellation> = match Wanted.as_str() {
			"aborted" => Signal.clone(),
			Kind if Kind.starts_with("after:") => Signal.clone(),
			_ => Arc::new(Never),
		};
		let Call = Call { Deadline: Pace.Now() + Budget, Cancellation };

		match BlockOn(Driver.Ask(Question.clone(), Call)) {
			Ok(Answer) => Result = answer_of(&Answer),
			Err(Why) => {
				Result = error_of(&Why);
				break;
			}
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
	} else if Kind(&Want) == "answer" {
		for Field in ["text", "model", "mode"] {
			if let Some(Wanted) = Want.get(Field) {
				if Got.get(Field) != Some(Wanted) {
					Wrong.push(format!("result {Field}: wanted {Wanted}, got {:?}", Got.get(Field)));
				}
			}
		}
		for Field in ["trace", "sources"] {
			if let Some(Wanted) = Want.get(Field) {
				if Got.get(Field) != Some(Wanted) {
					Wrong.push(format!("result {Field}: wanted {Wanted}, got {:?}", Got.get(Field)));
				}
			}
		}
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

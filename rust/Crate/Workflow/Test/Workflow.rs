//! The bridge and the composition, exercised against a fake browser.
//!
//! No browser and no network: the substrate is a fake that answers the
//! expressions the bindings send, and a clock the test owns decides what a
//! deadline means.

use std::{
	sync::{
		Arc,
		Mutex,
		atomic::{AtomicU64, Ordering},
	},
	time::{Duration, Instant},
};

use AgenticBrowserCDP::{Browse as BrowserBrowse, Error as BrowserError, OpenOptions, Target};
use AgenticChatDriver::{Failure, Page, Target as ChatTarget};
use AgenticDesignVerification::{Browse, Create as CreateVerifier, Invariant, Measurement};
use AgenticRuntime::{BlockOn, Call, Clock, FutureBox, Never, Pace};
use AgenticSkillContract::Create as CreateContract;
use AgenticWorkflow::{MeasurementExpression, VerifySkillPage};
use serde_json::{Value, json};

fn Span(Milliseconds:u64) -> Duration { Duration::from_millis(Milliseconds) }

/// A clock the test owns.
struct TestPace {
	Now:Mutex<Instant>,
}

impl TestPace {
	fn New() -> Arc<Self> { Arc::new(Self { Now:Mutex::new(Instant::now()) }) }
}

impl Clock for TestPace {
	fn Now(&self) -> Instant { *self.Now.lock().unwrap() }
}

impl Pace for TestPace {
	fn Sleep(&self, Span:Duration) -> FutureBox<'_, ()> {
		*self.Now.lock().unwrap() += Span.max(Duration::from_millis(1));
		Box::pin(std::future::ready(()))
	}
}

/// A browser that answers the expression it is handed.
struct FakeBrowse {
	Answer:Mutex<Result<Value, BrowserError>>,
	Opens:AtomicU64,
	Disposes:AtomicU64,
	Expressions:Mutex<Vec<String>>,
}

impl FakeBrowse {
	fn Answering(Value:Value) -> Arc<Self> {
		Arc::new(Self {
			Answer:Mutex::new(Ok(Value)),
			Opens:AtomicU64::new(0),
			Disposes:AtomicU64::new(0),
			Expressions:Mutex::new(Vec::new()),
		})
	}

	fn Failing(Why:BrowserError) -> Arc<Self> {
		Arc::new(Self {
			Answer:Mutex::new(Err(Why)),
			Opens:AtomicU64::new(0),
			Disposes:AtomicU64::new(0),
			Expressions:Mutex::new(Vec::new()),
		})
	}
}

impl BrowserBrowse for FakeBrowse {
	fn Open<'a>(&'a self, Url:&'a str, _Options:OpenOptions) -> FutureBox<'a, Result<Target, BrowserError>> {
		self.Opens.fetch_add(1, Ordering::SeqCst);
		let Answer = self.Answer.lock().unwrap().clone();
		Box::pin(async move {
			// A fake that is scripted to fail fails everywhere, so a test can
			// say "this browser is unreachable" once and mean it.
			if let Err(Why) = Answer {
				return Err(Why);
			}
			Ok(Target {
				Url:Url.to_string(),
				Ws:"ws://fake".to_string(),
				Ownership:AgenticBrowserCDP::Ownership::Launched,
			})
		})
	}

	fn Evaluate<'a>(
		&'a self,
		_Target:&'a Target,
		Expression:&'a str,
		_Call:Call,
	) -> FutureBox<'a, Result<Value, BrowserError>> {
		self.Expressions.lock().unwrap().push(Expression.to_string());
		let Answer = self.Answer.lock().unwrap().clone();
		Box::pin(async move { Answer })
	}

	fn Dispose<'a>(&'a self, _Target:Target) -> FutureBox<'a, ()> {
		self.Disposes.fetch_add(1, Ordering::SeqCst);
		Box::pin(async move {})
	}
}

fn Call(Pace:&TestPace) -> Call { Call::Within(Span(2_000), Pace, Arc::new(Never)) }

#[test]
fn the_page_binding_opens_launches_and_hands_the_driver_a_target_it_can_pass_back() {
	let Pace = TestPace::New();
	let Browse = FakeBrowse::Answering(json!("ok"));
	let Page = AgenticWorkflow::Struct::Binding::Page::Create(Browse.clone());
	let Target:ChatTarget = BlockOn(Page.Open("https://chat.example/", Call(&Pace))).expect("opened");
	assert_eq!(Target.Url, "https://chat.example/");
	assert_eq!(Browse.Opens.load(Ordering::SeqCst), 1);
	let Value = BlockOn(Page.Evaluate(&Target, "1 + 1", Call(&Pace))).expect("evaluated");
	assert_eq!(Value, json!("ok"));
	BlockOn(Page.Dispose(Target));
	assert_eq!(Browse.Disposes.load(Ordering::SeqCst), 1);
}

#[test]
fn the_page_binding_releases_the_browser_target_when_the_driver_releases_its_own() {
	let Pace = TestPace::New();
	let Browse = FakeBrowse::Answering(json!("ok"));
	let Page = AgenticWorkflow::Struct::Binding::Page::Create(Browse.clone());
	let Target = BlockOn(Page.Open("https://chat.example/", Call(&Pace))).expect("opened");
	BlockOn(Page.Dispose(Target.clone()));
	// A second release is silent, and the browser is not asked twice.
	BlockOn(Page.Dispose(Target.clone()));
	assert_eq!(Browse.Disposes.load(Ordering::SeqCst), 1);
	// And the handle is gone: asking the page again fails in the binding's
	// vocabulary.
	let Failure = BlockOn(Page.Evaluate(&Target, "1 + 1", Call(&Pace))).expect_err("failed");
	assert!(Failure.Message.contains("holds no browser target"));
}

#[test]
fn the_page_binding_keeps_the_browser_code_in_the_message_and_the_observation_in_the_detail() {
	let Pace = TestPace::New();
	let Browse = FakeBrowse::Failing(BrowserError::Navigation("the page did not load", "404"));
	let Page = AgenticWorkflow::Struct::Binding::Page::Create(Browse);
	let Failure:Failure = BlockOn(Page.Open("https://chat.example/", Call(&Pace))).expect_err("failed");
	assert!(Failure.Message.contains("navigation-failed"), "{}", Failure.Message);
	assert_eq!(Failure.Detail, "404");
}

#[test]
fn the_measure_binding_opens_measures_and_releases_the_page_every_time() {
	let Pace = TestPace::New();
	let Browse = FakeBrowse::Answering(json!([
		{ "selector": "[data-rail]", "present": true, "value": 320 }
	]));
	let Measure = AgenticWorkflow::Struct::Binding::Measure::Create(Browse.clone(), "https://site.example/");
	let Selectors = vec!["[data-rail]".to_string()];
	let Measurements = BlockOn(Measure.Measure(&Selectors, Call(&Pace))).expect("measured");
	assert_eq!(Measurements.len(), 1);
	assert_eq!(Browse.Opens.load(Ordering::SeqCst), 1);
	assert_eq!(Browse.Disposes.load(Ordering::SeqCst), 1, "stateless: it released");
}

#[test]
fn the_measure_binding_sends_the_expression_the_workflow_built() {
	let Pace = TestPace::New();
	let Browse = FakeBrowse::Answering(json!([]));
	let Measure = AgenticWorkflow::Struct::Binding::Measure::Create(Browse.clone(), "https://site.example/");
	let Selectors = vec!["[data-rail]".to_string(), "[data-drawer]".to_string()];
	let _ = BlockOn(Measure.Measure(&Selectors, Call(&Pace)));
	let Sent = Browse.Expressions.lock().unwrap().clone();
	assert_eq!(Sent.len(), 1);
	assert_eq!(Sent[0], MeasurementExpression(&Selectors));
	assert!(Sent[0].contains("data-value"));
	assert!(Sent[0].contains("[data-drawer]"));
}

#[test]
fn the_measure_binding_refuses_an_answer_that_is_not_a_list() {
	let Pace = TestPace::New();
	let Browse = FakeBrowse::Answering(json!("not a list"));
	let Measure = AgenticWorkflow::Struct::Binding::Measure::Create(Browse.clone(), "https://site.example/");
	let Selectors = vec!["[data-rail]".to_string()];
	let Failure = BlockOn(Measure.Measure(&Selectors, Call(&Pace))).expect_err("failed");
	assert!(Failure.Message.contains("instead of a list"));
	assert_eq!(Browse.Disposes.load(Ordering::SeqCst), 1, "still released");
}

#[test]
fn the_composition_reads_the_corpus_before_it_touches_a_page() {
	let Pace = TestPace::New();
	let Browse = FakeBrowse::Answering(json!([
		{ "selector": "[data-rail]", "present": true, "value": 320 }
	]));
	let Measure = AgenticWorkflow::Struct::Binding::Measure::Create(Browse.clone(), "https://site.example/");
	let Verifier = CreateVerifier(Arc::new(Measure), Pace.clone());
	let Contract = CreateContract();
	let Root = std::path::Path::new(env!("CARGO_TARGET_TMPDIR")).join("no-corpus-here");
	let Selectors = vec!["[data-rail]".to_string()];
	let Outcome = BlockOn(VerifySkillPage(
		&Contract,
		&Verifier,
		&Root.to_string_lossy(),
		&Selectors,
		&[],
		Call(&Pace),
	));
	assert!(Outcome.Skills.is_empty());
	assert!(Outcome.Findings.is_empty());
	assert!(Outcome.Digest.starts_with("sha256:"));
	assert!(Outcome.Verdict.is_some());
	assert!(Outcome.Held());
}

#[test]
fn the_composition_reports_a_corpus_that_broke_before_it_reports_a_page() {
	let Pace = TestPace::New();
	let Root = std::path::Path::new(env!("CARGO_TARGET_TMPDIR")).join("workflow-corpus");
	let _ = std::fs::remove_dir_all(&Root);
	let Skill = Root.join("Broken");
	std::fs::create_dir_all(&Skill).expect("the corpus is writable");
	std::fs::write(Skill.join("SKILL.md"), "---\nname: broken\n---\n").expect("SKILL.md");
	let Browse = FakeBrowse::Answering(json!([]));
	let Measure = AgenticWorkflow::Struct::Binding::Measure::Create(Browse.clone(), "https://site.example/");
	let Verifier = CreateVerifier(Arc::new(Measure), Pace.clone());
	let Contract = CreateContract();
	let Outcome = BlockOn(VerifySkillPage(
		&Contract,
		&Verifier,
		&Root.to_string_lossy(),
		&[],
		&[],
		Call(&Pace),
	));
	assert_eq!(Outcome.Skills.len(), 1);
	assert!(!Outcome.Findings.is_empty());
	assert!(!Outcome.Held());
	assert_eq!(Browse.Opens.load(Ordering::SeqCst), 0, "no page was touched");
}

#[test]
fn the_composition_judges_the_page_against_the_callers_invariants() {
	let Pace = TestPace::New();
	let Browse = FakeBrowse::Answering(json!([
		{ "selector": "[data-rail]", "present": true, "value": 320 }
	]));
	let Measure = AgenticWorkflow::Struct::Binding::Measure::Create(Browse, "https://site.example/");
	let Verifier = CreateVerifier(Arc::new(Measure), Pace.clone());
	let Contract = CreateContract();
	let Root = std::path::Path::new(env!("CARGO_TARGET_TMPDIR")).join("no-corpus-here");
	let Selectors = vec!["[data-rail]".to_string()];
	let Invariants = vec![Invariant::Equal(
		"the rail is 320 wide",
		|Measurements:&[Measurement]| Ok(Measurements[0].Value.clone()),
		json!(320),
	)];
	let Outcome = BlockOn(VerifySkillPage(
		&Contract,
		&Verifier,
		&Root.to_string_lossy(),
		&Selectors,
		&Invariants,
		Call(&Pace),
	));
	let Verdict = Outcome.Verdict.expect("a verdict");
	assert!(Verdict.Held);
	assert_eq!(Verdict.Evidence[0].Label, "the rail is 320 wide");
	assert_eq!(Outcome.Measurements.len(), 1);
}

#[test]
fn the_composition_keeps_the_findings_when_the_page_cannot_be_measured() {
	let Pace = TestPace::New();
	let Browse = FakeBrowse::Failing(BrowserError::Unavailable("connection refused", "ws://none"));
	let Measure = AgenticWorkflow::Struct::Binding::Measure::Create(Browse, "https://site.example/");
	let Verifier = CreateVerifier(Arc::new(Measure), Pace.clone());
	let Contract = CreateContract();
	let Root = std::path::Path::new(env!("CARGO_TARGET_TMPDIR")).join("no-corpus-here");
	let Selectors = vec!["[data-rail]".to_string()];
	let Outcome = BlockOn(VerifySkillPage(
		&Contract,
		&Verifier,
		&Root.to_string_lossy(),
		&Selectors,
		&[],
		Call(&Pace),
	));
	assert!(Outcome.Failure.as_deref().unwrap_or_default().contains("not-ready"));
	assert!(Outcome.Verdict.is_none());
	assert!(!Outcome.Held());
}

#[test]
fn the_measurement_expression_can_be_read_back() {
	let Selectors = vec!["[data-rail]".to_string()];
	let Expression = MeasurementExpression(&Selectors);
	let Parsed = AgenticWorkflow::Fn::Measure::MeasurementSelectors(&Expression).expect("parsed");
	assert_eq!(Parsed, json!(["[data-rail]"]));
}

#[test]
fn the_workflow_names_the_ports_it_needs_and_holds_no_design() {
	let Manifest = include_str!("../module.json");
	for Port in [
		"agentic.browse",
		"agentic.chat.ask",
		"agentic.verify.design",
		"agentic.skill.validate",
	] {
		assert!(Manifest.contains(Port), "{Port} is not in module.json");
	}
}

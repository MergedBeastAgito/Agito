//! `agentic.verify.design`, exercised against a scripted transport.
//!
//! No browser and no network: the transport answers what the test told it to,
//! and a deadline passes because the test advanced its own clock.

use std::{
	sync::{Arc, Mutex},
	time::{Duration, Instant},
};

use AgenticDesignVerification::{
	Browse,
	Code,
	Create,
	Error,
	Failure,
	Invariant,
	Measurement,
	Outcome,
	Verdict,
	Verifier,
	Verify,
	Want,
};
use AgenticRuntime::{BlockOn, Call, Clock, FutureBox, Never, Pace, Signal};
use serde_json::{Value, json};

fn Span(Milliseconds:u64) -> Duration { Duration::from_millis(Milliseconds) }

/// A clock the test owns, and a wait that advances it.
struct TestPace {
	Now:Mutex<Instant>,
	Fire:Mutex<Option<Arc<Signal>>>,
}

impl TestPace {
	fn New() -> Arc<Self> { Arc::new(Self { Now:Mutex::new(Instant::now()), Fire:Mutex::new(None) }) }

	fn Firing(self: &Arc<Self>, Signal:Arc<Signal>) -> Arc<Self> {
		*self.Fire.lock().unwrap() = Some(Signal);
		self.clone()
	}
}

impl Clock for TestPace {
	fn Now(&self) -> Instant { *self.Now.lock().unwrap() }
}

impl Pace for TestPace {
	fn Sleep(&self, Span:Duration) -> FutureBox<'_, ()> {
		// A pace that is about to fire a signal models a caller giving up, not
		// time passing: it advances a millisecond, fires, and never
		// finishes its wait - so the cancellation and not the deadline
		// is what stops the call.
		if let Some(Signal) = self.Fire.lock().unwrap().take() {
			*self.Now.lock().unwrap() += Duration::from_millis(1);
			Signal.Fire();
			return Box::pin(std::future::pending());
		}
		*self.Now.lock().unwrap() += Span.max(Duration::from_millis(1));
		Box::pin(std::future::ready(()))
	}
}

/// A transport that answers what the test scripted.
struct Script {
	/// The wire answer: one object per selector.
	Answer:Vec<Value>,
	/// A failure instead of an answer.
	Fail:Option<String>,
	/// Never answer at all.
	Hang:bool,
}

struct FakeTransport {
	Script:Mutex<Script>,
	Asked:Mutex<Vec<Vec<String>>>,
}

impl FakeTransport {
	fn New(Answer:Vec<Value>) -> Arc<Self> {
		Arc::new(Self {
			Script:Mutex::new(Script { Answer, Fail:None, Hang:false }),
			Asked:Mutex::new(Vec::new()),
		})
	}

	fn Failing(Message:&str) -> Arc<Self> {
		let Fake = Self::New(vec![]);
		Fake.Script.lock().unwrap().Fail = Some(Message.to_string());
		Fake
	}

	fn Hanging() -> Arc<Self> {
		let Fake = Self::New(vec![]);
		Fake.Script.lock().unwrap().Hang = true;
		Fake
	}

	fn Asked(&self) -> Vec<Vec<String>> { self.Asked.lock().unwrap().clone() }
}

impl Browse for FakeTransport {
	fn Measure<'a>(&'a self, Selectors:&'a [String], _Call:Call) -> FutureBox<'a, Result<Vec<Value>, Failure>> {
		self.Asked.lock().unwrap().push(Selectors.to_vec());
		let Script = self.Script.lock().unwrap();
		let Fail = Script.Fail.clone();
		let Answer = Script.Answer.clone();
		let Hang = Script.Hang;
		Box::pin(async move {
			if Hang {
				std::future::pending::<()>().await;
			}
			if let Some(Message) = Fail {
				return Err(Failure::Of(Message));
			}
			Ok(Answer)
		})
	}
}

fn Built(Transport:&Arc<FakeTransport>, Pace:&Arc<TestPace>) -> Verifier { Create(Transport.clone(), Pace.clone()) }

fn Measured(
	Verifier:&impl Verify,
	Pace:&TestPace,
	Selectors:&[String],
	Budget:Duration,
) -> Result<Vec<Measurement>, Error> {
	let Call = Call::Within(Budget, Pace, Arc::new(Never));
	BlockOn(Verifier.Measure(Selectors, Call))
}

fn Selectors(Names:&[&str]) -> Vec<String> { Names.iter().map(|Name| Name.to_string()).collect() }

/// The selectors a test asks about, none of them a design of this package.
fn Asked() -> Vec<String> { Selectors(&["[data-rail]", "[data-drawer]"]) }

fn Present(Selector:&str, Value:Value) -> Value { json!({ "selector": Selector, "present": true, "value": Value }) }

fn Absent(Selector:&str) -> Value { json!({ "selector": Selector, "present": false, "value": null }) }

fn Readings() -> Vec<Measurement> {
	vec![
		Measurement::Present("[data-rail]", json!(320)),
		Measurement::Present("[data-drawer]", json!(320)),
	]
}

#[test]
fn the_error_codes_are_the_manifests_errors_and_there_are_no_others() {
	let Declared:Vec<&str> = Code::All().into_iter().map(Code::Name).collect();
	assert_eq!(Declared, vec!["absent-element", "invariant-broke", "not-ready"]);
	let Manifest = include_str!("../module.json");
	for Name in Declared {
		assert!(Manifest.contains(&format!("\"{Name}\"")), "{Name} is not in module.json");
	}
}

#[test]
fn measure_asks_for_every_selector_once_and_answers_in_the_order_it_was_given() {
	let Pace = TestPace::New();
	let Transport = FakeTransport::New(vec![Absent("[data-drawer]"), Present("[data-rail]", json!(320))]);
	let Verifier = Built(&Transport, &Pace);
	let Measurements = Measured(&Verifier, &Pace, &Asked(), Span(2_000)).expect("measured");
	assert_eq!(Transport.Asked(), vec![Asked()], "one request, once");
	assert_eq!(Measurements.len(), 2);
	assert_eq!(Measurements[0].Selector, "[data-rail]");
	assert_eq!(Measurements[1].Selector, "[data-drawer]");
}

#[test]
fn an_answer_that_arrives_in_another_order_is_matched_by_selector_not_by_position() {
	let Pace = TestPace::New();
	let Transport = FakeTransport::New(vec![Present("[data-drawer]", json!(200)), Present("[data-rail]", json!(320))]);
	let Verifier = Built(&Transport, &Pace);
	let Measurements = Measured(&Verifier, &Pace, &Asked(), Span(2_000)).expect("measured");
	assert_eq!(Measurements[0].Value, json!(320));
	assert_eq!(Measurements[1].Value, json!(200));
}

#[test]
fn an_element_the_page_does_not_have_is_a_measurement_never_a_failure() {
	let Pace = TestPace::New();
	let Transport = FakeTransport::New(vec![Absent("[data-rail]"), Absent("[data-drawer]")]);
	let Verifier = Built(&Transport, &Pace);
	let Measurements = Measured(&Verifier, &Pace, &Asked(), Span(2_000)).expect("measured");
	assert_eq!(Measurements.len(), 2);
	assert!(Measurements.iter().all(|Measurement| !Measurement.Present));
	assert_eq!(Measurements[0].Value, Value::Null);
}

#[test]
fn an_absent_element_cannot_carry_a_value_out_of_the_transport() {
	let Pace = TestPace::New();
	let Transport = FakeTransport::New(vec![
		json!({ "selector": "[data-rail]", "present": false, "value": 320 }),
		Absent("[data-drawer]"),
	]);
	let Verifier = Built(&Transport, &Pace);
	let Measurements = Measured(&Verifier, &Pace, &Asked(), Span(2_000)).expect("measured");
	assert_eq!(Measurements[0].Value, Value::Null, "the stale number was dropped");
}

#[test]
fn a_dead_transport_is_not_ready_which_is_not_the_answer_an_absent_element_gives() {
	let Pace = TestPace::New();
	let Transport = FakeTransport::Failing("the socket closed");
	let Verifier = Built(&Transport, &Pace);
	let Error = Measured(&Verifier, &Pace, &Asked(), Span(2_000)).expect_err("refused");
	assert_eq!(Error.Code(), Code::NotReady);
	assert_eq!(Error.Detail(), "the socket closed");
}

#[test]
fn a_transport_that_never_answers_is_not_ready_at_the_deadline_not_a_hang() {
	let Pace = TestPace::New();
	let Transport = FakeTransport::Hanging();
	let Verifier = Built(&Transport, &Pace);
	let Error = Measured(&Verifier, &Pace, &Asked(), Span(1_000)).expect_err("refused");
	assert_eq!(Error.Code(), Code::NotReady);
	assert!(Error.Detail().contains("never answered"));
}

#[test]
fn a_signal_that_had_already_fired_refuses_before_the_page_is_asked() {
	let Pace = TestPace::New();
	let Transport = FakeTransport::New(vec![Absent("[data-rail]")]);
	let Verifier = Built(&Transport, &Pace);
	let Signal = Signal::New();
	Signal.Fire();
	let Call = Call { Deadline:Pace.Now() + Span(5_000), Cancellation:Signal.clone() };
	let Error = BlockOn(Verifier.Measure(&Asked(), Call)).expect_err("refused");
	assert_eq!(Error.Code(), Code::NotReady);
	assert!(Error.Detail().contains("signal"));
	assert!(Transport.Asked().is_empty(), "the page was never asked");
}

#[test]
fn a_signal_that_fires_while_the_page_is_being_read_is_not_ready_named_as_a_cancellation() {
	let Signal = Signal::New();
	let Pace = TestPace::New().Firing(Signal.clone());
	let Transport = FakeTransport::Hanging();
	let Verifier = Built(&Transport, &Pace);
	let Call = Call { Deadline:Pace.Now() + Span(30_000), Cancellation:Signal.clone() };
	let Error = BlockOn(Verifier.Measure(&Asked(), Call)).expect_err("refused");
	assert_eq!(Error.Code(), Code::NotReady);
	assert!(Error.Detail().contains("signal"));
}

#[test]
fn a_selector_the_transport_did_not_answer_for_is_a_broken_transport_not_an_absent_element() {
	let Pace = TestPace::New();
	let Transport = FakeTransport::New(vec![Present("[data-rail]", json!(320))]);
	let Verifier = Built(&Transport, &Pace);
	let Error = Measured(&Verifier, &Pace, &Asked(), Span(2_000)).expect_err("refused");
	assert_eq!(Error.Code(), Code::NotReady);
	assert!(Error.Detail().contains("nothing was measured for \"[data-drawer]\""));
}

#[test]
fn a_measurement_that_does_not_say_whether_the_element_was_there_is_refused() {
	let Pace = TestPace::New();
	let Transport = FakeTransport::New(vec![
		json!({ "selector": "[data-rail]", "value": 320 }),
		Absent("[data-drawer]"),
	]);
	let Verifier = Built(&Transport, &Pace);
	let Error = Measured(&Verifier, &Pace, &Asked(), Span(2_000)).expect_err("refused");
	assert_eq!(Error.Code(), Code::NotReady);
	assert!(Error.Message().contains("whether the element was there"));
}

#[test]
fn a_value_that_is_not_a_number_a_string_or_null_is_refused_rather_than_flattened() {
	let Pace = TestPace::New();
	let Transport = FakeTransport::New(vec![Present("[data-rail]", json!({ "width": 320 })), Absent("[data-drawer]")]);
	let Verifier = Built(&Transport, &Pace);
	let Error = Measured(&Verifier, &Pace, &Asked(), Span(2_000)).expect_err("refused");
	assert_eq!(Error.Code(), Code::NotReady);
	assert!(Error.Detail().contains("a measurement is a number, a string or null"));
}

#[test]
fn a_transport_that_answers_with_something_other_than_a_list_is_refused() {
	let Pace = TestPace::New();
	// A list of one number: the entry is not an object, which is the wire-shape
	// version of "the transport answered with something that is not a
	// measurement".
	let Transport = FakeTransport::New(vec![json!(42)]);
	let Verifier = Built(&Transport, &Pace);
	let Error = Measured(&Verifier, &Pace, &Asked(), Span(2_000)).expect_err("refused");
	assert_eq!(Error.Code(), Code::NotReady);
	assert!(Error.Message().contains("not an object"));
}

#[test]
fn a_measurement_of_nothing_asks_the_transport_for_nothing() {
	let Pace = TestPace::New();
	let Transport = FakeTransport::New(vec![]);
	let Verifier = Built(&Transport, &Pace);
	let Measurements = Measured(&Verifier, &Pace, &[], Span(2_000)).expect("measured");
	assert_eq!(Measurements, vec![]);
	assert!(Transport.Asked().is_empty(), "a no-op must not touch a page");
}

#[test]
fn a_verdict_holds_when_nothing_is_absent_and_every_invariant_holds() {
	let Pace = TestPace::New();
	let Verifier = Built(&FakeTransport::New(vec![]), &Pace);
	let Invariants = vec![Invariant::Equal(
		"the rail is 320 wide",
		|Measurements| Ok(Measurements[0].Value.clone()),
		json!(320),
	)];
	let Verdict = Verifier.Check(&Invariants, &Readings());
	assert!(Verdict.Held);
	assert_eq!(Verdict.Absent, Vec::<String>::new());
	assert_eq!(Verdict.Evidence.len(), 1);
	assert_eq!(Verdict.Evidence[0].Got, Outcome::Value(json!(320)));
	assert_eq!(Verdict.Evidence[0].Want, Outcome::Value(json!(320)));
	assert_eq!(Error::From(&Verdict), None);
}

#[test]
fn an_absent_element_fails_the_verdict_even_when_every_invariant_reads_a_number() {
	let Pace = TestPace::New();
	let Verifier = Built(&FakeTransport::New(vec![]), &Pace);
	let Measurements = vec![
		Measurement::Absent("[data-rail]"),
		Measurement::Present("[data-drawer]", json!(320)),
	];
	let Invariants = vec![Invariant::Equal(
		"the drawer is 320 wide",
		|Measurements| Ok(Measurements[1].Value.clone()),
		json!(320),
	)];
	let Verdict = Verifier.Check(&Invariants, &Measurements);
	assert!(!Verdict.Held);
	assert_eq!(Verdict.Absent, vec!["[data-rail]".to_string()]);
	assert!(Verdict.Evidence[0].Held, "the row that held still held");
}

#[test]
fn a_broken_invariant_names_itself_and_only_itself_among_the_failures() {
	let Pace = TestPace::New();
	let Verifier = Built(&FakeTransport::New(vec![]), &Pace);
	let Invariants = vec![
		Invariant::Equal("holds", |Measurements| Ok(Measurements[0].Value.clone()), json!(320)),
		Invariant::Equal("breaks", |Measurements| Ok(Measurements[1].Value.clone()), json!(200)),
	];
	let Verdict = Verifier.Check(&Invariants, &Readings());
	assert!(!Verdict.Held);
	let Error = Error::From(&Verdict).expect("a failure");
	assert_eq!(Error.Code(), Code::Broke);
	assert_eq!(Error.Detail(), "breaks");
}

#[test]
fn a_read_that_fails_is_a_failed_invariant_not_an_exception_out_of_check() {
	let Pace = TestPace::New();
	let Verifier = Built(&FakeTransport::New(vec![]), &Pace);
	let Invariants = vec![Invariant::Equal(
		"reads a missing element",
		|_| Err("nothing was measured for [data-rail]".to_string()),
		json!(320),
	)];
	let Verdict = Verifier.Check(&Invariants, &Readings());
	assert!(!Verdict.Held);
	assert_eq!(
		Verdict.Evidence[0].Got,
		Outcome::Threw("nothing was measured for [data-rail]".to_string())
	);
	assert_eq!(Verdict.Evidence[0].Want, Outcome::Value(json!(320)));
}

#[test]
fn an_expectation_that_is_a_function_of_the_measurements_is_derived_not_hard_coded() {
	let Pace = TestPace::New();
	let Verifier = Built(&FakeTransport::New(vec![]), &Pace);
	let Invariants = vec![Invariant::Derived(
		"the rail sits level with the drawer",
		|Measurements| Ok(Measurements[0].Value.clone()),
		|Measurements| Ok(Measurements[1].Value.clone()),
	)];
	let Verdict = Verifier.Check(&Invariants, &Readings());
	assert!(Verdict.Held);
	assert_eq!(Verdict.Evidence[0].Want, Outcome::Value(json!(320)));
}

#[test]
fn a_comparator_makes_a_relational_invariant_expressible() {
	let Pace = TestPace::New();
	let Verifier = Built(&FakeTransport::New(vec![]), &Pace);
	let Invariants = vec![
		Invariant::Equal(
			"the rail is wider than the drawer",
			|Measurements| Ok(Measurements[0].Value.clone()),
			json!(200),
		)
		.Tested(|Got, Want| Got.as_f64().unwrap_or_default() > Want.as_f64().unwrap_or_default()),
	];
	let Verdict = Verifier.Check(&Invariants, &Readings());
	assert!(Verdict.Held, "320 is wider than 200");
}

#[test]
fn a_comparator_that_panics_counts_as_not_held() {
	let Pace = TestPace::New();
	let Verifier = Built(&FakeTransport::New(vec![]), &Pace);
	let Invariants = vec![
		Invariant::Equal(
			"a comparison that cannot be made",
			|Measurements| Ok(Measurements[0].Value.clone()),
			json!(320),
		)
		.Tested(|_, _| panic!("the comparator is broken")),
	];
	let Verdict = Verifier.Check(&Invariants, &Readings());
	assert!(!Verdict.Held);
	assert_eq!(Verdict.Evidence[0].Got, Outcome::Value(json!(320)));
}

#[test]
fn a_present_element_that_read_null_is_a_value_not_an_absence() {
	let Pace = TestPace::New();
	let Verifier = Built(&FakeTransport::New(vec![]), &Pace);
	let Measurements = vec![
		Measurement::Present("[data-rail]", Value::Null),
		Measurement::Present("[data-drawer]", json!(320)),
	];
	let Invariants = vec![Invariant::Equal(
		"the rail reported nothing",
		|Measurements| Ok(Measurements[0].Value.clone()),
		Value::Null,
	)];
	let Verdict = Verifier.Check(&Invariants, &Measurements);
	assert!(Verdict.Held, "null is a value the element reported");
	assert!(Verdict.Absent.is_empty());
}

#[test]
fn the_absent_list_names_each_selector_once_in_the_order_it_was_measured() {
	let Pace = TestPace::New();
	let Verifier = Built(&FakeTransport::New(vec![]), &Pace);
	let Measurements = vec![
		Measurement::Absent("[data-second]"),
		Measurement::Absent("[data-first]"),
		Measurement::Absent("[data-second]"),
	];
	let Verdict = Verifier.Check(&[], &Measurements);
	assert_eq!(Verdict.Absent, vec!["[data-second]".to_string(), "[data-first]".to_string()]);
}

#[test]
fn a_run_with_no_invariants_holds_when_nothing_is_absent_and_fails_when_something_is() {
	let Pace = TestPace::New();
	let Verifier = Built(&FakeTransport::New(vec![]), &Pace);
	assert!(Verifier.Check(&[], &Readings()).Held);
	let WithAbsence = vec![
		Measurement::Absent("[data-rail]"),
		Measurement::Present("[data-drawer]", json!(320)),
	];
	assert!(!Verifier.Check(&[], &WithAbsence).Held);
}

#[test]
fn check_is_a_pure_function_of_its_arguments() {
	let Pace = TestPace::New();
	let Verifier = Built(&FakeTransport::New(vec![]), &Pace);
	let Invariants = vec![Invariant::Equal(
		"the rail is 320 wide",
		|Measurements| Ok(Measurements[0].Value.clone()),
		json!(320),
	)];
	let First = Verifier.Check(&Invariants, &Readings());
	let Second = Verifier.Check(&Invariants, &Readings());
	assert_eq!(First, Second);
}

#[test]
fn a_verdict_becomes_the_error_the_manifest_names_absence_before_a_broken_invariant() {
	let Pace = TestPace::New();
	let Verifier = Built(&FakeTransport::New(vec![]), &Pace);
	let Measurements = vec![
		Measurement::Absent("[data-rail]"),
		Measurement::Present("[data-drawer]", json!(200)),
	];
	let Invariants = vec![Invariant::Equal(
		"the drawer is 320 wide",
		|Measurements| Ok(Measurements[1].Value.clone()),
		json!(320),
	)];
	let Verdict = Verifier.Check(&Invariants, &Measurements);
	assert!(!Verdict.Held);
	assert_eq!(Verdict.Absent.len(), 1, "there is a broken invariant too");
	let Error = Error::From(&Verdict).expect("a failure");
	assert_eq!(Error.Code(), Code::Absent, "absence is reported first");
	assert_eq!(Error.Detail(), "[data-rail]");
}

#[test]
fn the_package_carries_no_selector_length_or_colour_of_any_design() {
	// The rule this crate exists to keep, checked against its own source: a
	// port that knew one site's rail width could not measure another site
	// at all.
	let Sources = [
		include_str!("../Source/Struct/Port.rs"),
		include_str!("../Source/Struct/Verify.rs"),
		include_str!("../Source/Struct/Error.rs"),
		include_str!("../Source/Fn/Judge.rs"),
		include_str!("../Source/Fn/Measure.rs"),
	];
	for Source in Sources {
		for Forbidden in ["data-testid", "aria-label", "#ffffff", "#fff", " px"] {
			assert!(!Source.contains(Forbidden), "the package names {Forbidden}, which is a design");
		}
	}
}

#[test]
fn a_verdict_with_nothing_wrong_becomes_no_error_at_all() {
	let Pace = TestPace::New();
	let Verifier = Built(&FakeTransport::New(vec![]), &Pace);
	let Verdict:Verdict = Verifier.Check(&[], &Readings());
	assert!(Verdict.Held);
	assert_eq!(Error::From(&Verdict), None);
	let _ = Want::Value(Value::Null);
}

//! `agentic.chat.ask`, exercised against a scripted page and a clock the test
//! owns.
//!
//! No browser, no network and no sleeping: a page that never answers is a page
//! whose script says so, and a deadline is passed because the test advanced its
//! own clock.

mod Fake;

use std::{
	sync::{Arc, Mutex, atomic::Ordering},
	time::Duration,
};

use AgenticChatDriver::{Answer, Chat, Code, Create, DefaultSelectors, Driver, Error, Mode, Options, Question};
use AgenticRuntime::{BlockOn, Call, Clock, Never, Signal};
use Fake::{Answers, FakePage, Release, Script, TestPace};
use serde_json::json;

fn Span(Milliseconds:u64) -> Duration { Duration::from_millis(Milliseconds) }

fn Asked(Text:&str, Model:&str) -> Question {
	Question { Text:Text.to_string(), Mode:Mode::Search, Model:Model.to_string() }
}

fn Built(Page:&Arc<FakePage>, Pace:&Arc<TestPace>) -> Driver {
	Create(
		Page.clone(),
		Options {
			Url:"https://chat.example/".to_string(),
			Selectors:DefaultSelectors(),
			Poll:Span(250),
			DisposeBudget:Duration::from_secs(5),
		},
		Pace.clone(),
	)
}

fn Ask(Driver:&Driver, Pace:&TestPace, Question:Question, Budget:Duration) -> Result<Answer, Error> {
	let Call = Call::Within(Budget, Pace, Arc::new(Never));
	BlockOn(Driver.Ask(Question, Call))
}

#[test]
fn the_port_declares_exactly_the_errors_the_manifest_promises() {
	let Declared:Vec<&str> = Code::All().into_iter().map(Code::Name).collect();
	assert_eq!(Declared, vec!["not-ready", "wrong-mode", "wrong-model", "truncated"]);
	let Manifest = include_str!("../module.json");
	for Name in Declared {
		assert!(Manifest.contains(&format!("\"{Name}\"")), "{Name} is not in module.json");
	}
}

#[test]
fn returns_the_answer_its_trace_and_its_sources() {
	let Pace = TestPace::New();
	let Page = FakePage::New(Script::New("why", "gpt-6.1 sol", "Because."));
	let Driver = Built(&Page, &Pace);
	let Answer = Ask(&Driver, &Pace, Asked("why", "gpt-6.1 sol"), Span(2_000)).expect("answered");
	assert_eq!(Answer.Text, "Because.");
	assert_eq!(Answer.Model, "gpt-6.1 sol");
	assert_eq!(Answer.Mode, "search");
	assert_eq!(Answer.Trace, vec![]);
	assert_eq!(Answer.Sources, vec![]);
}

#[test]
fn opens_the_configured_page_and_disposes_the_target_once() {
	let Pace = TestPace::New();
	let Page = FakePage::New(Script::New("why", "sol", "Because."));
	let Driver = Built(&Page, &Pace);
	Ask(&Driver, &Pace, Asked("why", "sol"), Span(2_000)).expect("answered");
	assert_eq!(Page.Opens.load(Ordering::SeqCst), 1);
	assert_eq!(Page.Releases.load(Ordering::SeqCst), 1);
}

#[test]
fn presses_the_toggle_when_the_composer_is_not_in_search_mode() {
	let Pace = TestPace::New();
	let mut Script = Script::New("why", "sol", "Because.");
	Script.Mode = json!("Computer");
	Script.ModeAfterPress = json!("Search");
	let Page = FakePage::New(Script);
	let Driver = Built(&Page, &Pace);
	let Answer = Ask(&Driver, &Pace, Asked("why", "sol"), Span(2_000)).expect("answered");
	assert_eq!(Answer.Mode, "search");
	assert_eq!(Page.Presses.load(Ordering::SeqCst), 1);
}

#[test]
fn refuses_with_wrong_mode_and_sends_nothing_when_the_toggle_never_reads_search() {
	let Pace = TestPace::New();
	let mut Script = Script::New("why", "sol", "Because.");
	Script.Mode = json!("Computer");
	Script.ModeAfterPress = json!("Computer");
	let Page = FakePage::New(Script);
	let Driver = Built(&Page, &Pace);
	let Error = Ask(&Driver, &Pace, Asked("why", "sol"), Span(2_000)).expect_err("refused");
	assert_eq!(Error.Code(), Code::WrongMode);
	assert!(Error.Detail().contains("\"Computer\""));
	assert!(!Page.Typed(), "nothing was typed");
	assert!(!Page.Sent(), "nothing was sent");
}

#[test]
fn refuses_a_question_that_asks_for_a_mode_this_port_does_not_offer() {
	let Pace = TestPace::New();
	let Page = FakePage::New(Script::New("why", "sol", "Because."));
	let Driver = Built(&Page, &Pace);
	let Question = Question {
		Text:"why".to_string(),
		Mode:Mode::Other("computer".to_string()),
		Model:"sol".to_string(),
	};
	let Error = Ask(&Driver, &Pace, Question, Span(2_000)).expect_err("refused");
	assert_eq!(Error.Code(), Code::WrongMode);
	assert!(Error.Detail().contains("computer"));
	assert_eq!(Page.Opens.load(Ordering::SeqCst), 0, "no page was opened");
}

#[test]
fn refuses_with_wrong_model_and_sends_nothing() {
	let Pace = TestPace::New();
	let Page = FakePage::New(Script::New("why", "another-model", "Because."));
	let Driver = Built(&Page, &Pace);
	let Error = Ask(&Driver, &Pace, Asked("why", "sol"), Span(2_000)).expect_err("refused");
	assert_eq!(Error.Code(), Code::WrongModel);
	assert!(Error.Detail().contains("another-model"));
	assert!(!Page.Typed(), "nothing was typed");
	assert_eq!(Page.Releases.load(Ordering::SeqCst), 1, "the page was released");
}

#[test]
fn matches_the_label_across_case_and_whitespace_and_reports_what_it_read() {
	let Pace = TestPace::New();
	let mut Script = Script::New("why", "  GPT-6.1   SOL ", "Because.");
	Script.Model = json!("  GPT-6.1   SOL ");
	let Page = FakePage::New(Script);
	let Driver = Built(&Page, &Pace);
	let Answer = Ask(&Driver, &Pace, Asked("why", "gpt-6.1 sol"), Span(2_000)).expect("answered");
	assert_eq!(Answer.Model, "GPT-6.1   SOL");
}

#[test]
fn returns_the_last_text_of_an_answer_that_stops_changing() {
	let Pace = TestPace::New();
	let Page = FakePage::New(Script::New("why", "sol", "answer"));
	let Driver = Built(&Page, &Pace);
	let Answer = Ask(&Driver, &Pace, Asked("why", "sol"), Span(2_000)).expect("answered");
	assert_eq!(Answer.Text, "answer");
	// One correlation read plus four answer reads: the fourth is the third
	// comparison that agreed.
	assert_eq!(Page.Reads.load(Ordering::SeqCst), 5);
}

#[test]
fn reads_the_answer_this_question_caused_not_the_one_already_in_the_thread() {
	let Pace = TestPace::New();
	let mut Script = Script::New("why", "sol", "answer");
	Script.Thread = Answers::Fixed(vec![
		json!({ "count": 1, "text": "an earlier answer" }),
		json!({ "count": 2, "text": "this answer" }),
	]);
	let Page = FakePage::New(Script);
	let Driver = Built(&Page, &Pace);
	let Answer = Ask(&Driver, &Pace, Asked("why", "sol"), Span(2_000)).expect("answered");
	assert_eq!(Answer.Text, "this answer");
}

#[test]
fn reads_the_steps_and_sources_of_this_turn() {
	let Pace = TestPace::New();
	let mut Script = Script::New("why", "sol", "answer");
	Script.Trace = json!([
		{ "depth": 0, "label": "Finished", "detail": "12s" },
		{ "depth": 1, "label": "Searching", "detail": "" },
	]);
	Script.Sources = json!([{ "url": "https://example.test/a", "label": "A source" }]);
	let Page = FakePage::New(Script);
	let Driver = Built(&Page, &Pace);
	let Answer = Ask(&Driver, &Pace, Asked("why", "sol"), Span(2_000)).expect("answered");
	assert_eq!(Answer.Trace.len(), 2);
	assert_eq!(Answer.Trace[0].Label, "Finished");
	assert_eq!(Answer.Trace[0].Detail, "12s");
	assert_eq!(Answer.Trace[1].Depth, 1);
	assert_eq!(Answer.Sources.len(), 1);
	assert_eq!(Answer.Sources[0].Url, "https://example.test/a");
	assert_eq!(Answer.Sources[0].Label, "A source");
}

#[test]
fn refuses_with_not_ready_when_the_answer_never_stops_changing() {
	let Pace = TestPace::New();
	let mut Script = Script::New("why", "sol", "answer");
	Script.Thread = Answers::Changing;
	let Page = FakePage::New(Script);
	let Driver = Built(&Page, &Pace);
	let Error = Ask(&Driver, &Pace, Asked("why", "sol"), Span(1_000)).expect_err("refused");
	assert_eq!(Error.Code(), Code::NotReady);
	assert!(Error.Message().contains("did not settle"));
}

#[test]
fn refuses_before_asking_when_the_deadline_has_already_passed() {
	let Pace = TestPace::New();
	let Page = FakePage::New(Script::New("why", "sol", "answer"));
	let Driver = Built(&Page, &Pace);
	let Call = Call { Deadline:Pace.Now() - Span(1), Cancellation:Arc::new(Never) };
	let Error = BlockOn(Driver.Ask(Asked("why", "sol"), Call)).expect_err("refused");
	assert_eq!(Error.Code(), Code::NotReady);
	assert_eq!(Page.Opens.load(Ordering::SeqCst), 0, "no page was opened");
	assert!(Page.Asked().is_empty(), "no expression was sent");
}

#[test]
fn hands_the_page_the_callers_own_deadline() {
	let Pace = TestPace::New();
	let Page = FakePage::New(Script::New("why", "sol", "answer"));
	let Driver = Built(&Page, &Pace);
	let Call = Call::Within(Span(3_000), Pace.as_ref(), Arc::new(Never));
	let Deadline = Call.Deadline;
	BlockOn(Driver.Ask(Asked("why", "sol"), Call)).expect("answered");
	assert_eq!(*Page.Deadline.lock().unwrap(), Some(Deadline));
}

#[test]
fn refuses_with_truncated_when_the_answer_hides_behind_a_read_more_control() {
	let Pace = TestPace::New();
	let mut Script = Script::New("why", "sol", "answer");
	Script.Collapse = json!("Read more");
	let Page = FakePage::New(Script);
	let Driver = Built(&Page, &Pace);
	let Error = Ask(&Driver, &Pace, Asked("why", "sol"), Span(2_000)).expect_err("refused");
	assert_eq!(Error.Code(), Code::Truncated);
	assert!(Error.Detail().contains("Read more"));
}

#[test]
fn reports_not_ready_when_the_page_cannot_be_opened() {
	let Pace = TestPace::New();
	let mut Script = Script::New("why", "sol", "answer");
	Script.FailOpen = Some("connection refused".to_string());
	let Page = FakePage::New(Script);
	let Driver = Built(&Page, &Pace);
	let Error = Ask(&Driver, &Pace, Asked("why", "sol"), Span(2_000)).expect_err("refused");
	assert_eq!(Error.Code(), Code::NotReady);
	assert_eq!(Error.Detail(), "connection refused");
	assert_eq!(Page.Releases.load(Ordering::SeqCst), 0, "nothing to release");
}

#[test]
fn reports_not_ready_when_the_page_fails_while_it_is_being_read() {
	let Pace = TestPace::New();
	let mut Script = Script::New("why", "sol", "answer");
	Script.FailEvaluate = Some("the socket closed".to_string());
	let Page = FakePage::New(Script);
	let Driver = Built(&Page, &Pace);
	let Error = Ask(&Driver, &Pace, Asked("why", "sol"), Span(2_000)).expect_err("refused");
	assert_eq!(Error.Code(), Code::NotReady);
	assert_eq!(Error.Detail(), "the socket closed");
	assert_eq!(Page.Releases.load(Ordering::SeqCst), 1, "the page was released");
}

#[test]
fn refuses_to_press_a_send_control_that_never_enables() {
	let Pace = TestPace::New();
	let mut Script = Script::New("why", "sol", "answer");
	Script.Submit = json!("disabled");
	let Page = FakePage::New(Script);
	let Driver = Built(&Page, &Pace);
	let Error = Ask(&Driver, &Pace, Asked("why", "sol"), Span(2_000)).expect_err("refused");
	assert_eq!(Error.Code(), Code::NotReady);
	assert!(Error.Detail().contains("disabled"));
}

#[test]
fn reports_not_ready_before_opening_anything_when_the_signal_already_fired() {
	let Pace = TestPace::New();
	let Page = FakePage::New(Script::New("why", "sol", "answer"));
	let Driver = Built(&Page, &Pace);
	let Signal = Signal::New();
	Signal.Fire();
	let Call = Call { Deadline:Pace.Now() + Span(5_000), Cancellation:Signal.clone() };
	let Error = BlockOn(Driver.Ask(Asked("why", "sol"), Call)).expect_err("refused");
	assert_eq!(Error.Code(), Code::NotReady);
	assert!(Error.Detail().contains("signal"));
	assert_eq!(Page.Opens.load(Ordering::SeqCst), 0);
}

#[test]
fn reports_not_ready_when_the_caller_gives_up_while_the_answer_is_arriving() {
	let Signal = Signal::New();
	let Pace = TestPace::New().Firing(Signal.clone());
	let mut Script = Script::New("why", "sol", "answer");
	Script.Thread = Answers::Changing;
	let Page = FakePage::New(Script);
	let Driver = Built(&Page, &Pace);
	let Call = Call { Deadline:Pace.Now() + Span(30_000), Cancellation:Signal.clone() };
	let Error = BlockOn(Driver.Ask(Asked("why", "sol"), Call)).expect_err("refused");
	assert_eq!(Error.Code(), Code::NotReady);
	assert!(Error.Detail().contains("signal"));
	assert!(Page.Reads.load(Ordering::SeqCst) < 5, "it stopped early");
}

#[test]
fn reports_a_release_that_hangs_instead_of_waiting_for_it() {
	let Pace = TestPace::New();
	let mut Script = Script::New("why", "sol", "answer");
	Script.Release = Release::Hang;
	let Page = FakePage::New(Script);
	let Reported:Arc<Mutex<Vec<String>>> = Arc::new(Mutex::new(Vec::new()));
	let Collected = Reported.clone();
	let Driver = Built(&Page, &Pace).Reporting(Arc::new(move |_Target, Error| {
		Collected.lock().unwrap().push(Error.Message().to_string());
	}));
	let Answer = Ask(&Driver, &Pace, Asked("why", "sol"), Span(30_000));
	assert!(Answer.is_ok(), "the caller keeps its answer");
	let Reported = Reported.lock().unwrap();
	assert_eq!(Reported.len(), 1);
	assert!(Reported[0].contains("dispose"));
}

#[test]
fn a_reporter_that_panics_does_not_turn_a_release_into_a_failure() {
	let Pace = TestPace::New();
	let mut Script = Script::New("why", "sol", "answer");
	Script.Release = Release::Hang;
	let Page = FakePage::New(Script);
	let Driver = Built(&Page, &Pace).Reporting(Arc::new(|_Target, _Error| {
		panic!("the reporter is broken");
	}));
	let Answer = Ask(&Driver, &Pace, Asked("why", "sol"), Span(30_000));
	assert!(Answer.is_ok(), "the caller keeps its answer");
}

#[test]
fn refuses_a_mode_that_is_not_text_with_wrong_mode_and_sends_nothing() {
	let Pace = TestPace::New();
	let mut Script = Script::New("why", "sol", "answer");
	Script.Mode = json!(42);
	Script.ModeAfterPress = json!(42);
	let Page = FakePage::New(Script);
	let Driver = Built(&Page, &Pace);
	let Error = Ask(&Driver, &Pace, Asked("why", "sol"), Span(2_000)).expect_err("refused");
	assert_eq!(Error.Code(), Code::WrongMode);
	assert!(Error.Detail().contains("42"));
	assert!(!Page.Typed());
}

#[test]
fn refuses_a_thread_read_that_is_not_a_thread() {
	let Pace = TestPace::New();
	let mut Script = Script::New("why", "sol", "answer");
	Script.Thread = Answers::Fixed(vec![json!(42)]);
	let Page = FakePage::New(Script);
	let Driver = Built(&Page, &Pace);
	let Error = Ask(&Driver, &Pace, Asked("why", "sol"), Span(2_000)).expect_err("refused");
	assert_eq!(Error.Code(), Code::NotReady);
	assert!(Error.Detail().contains("instead of a thread"));
}

#[test]
fn refuses_a_trace_that_is_not_a_list_of_steps() {
	let Pace = TestPace::New();
	let mut Script = Script::New("why", "sol", "answer");
	Script.Trace = json!("not a list");
	let Page = FakePage::New(Script);
	let Driver = Built(&Page, &Pace);
	let Error = Ask(&Driver, &Pace, Asked("why", "sol"), Span(2_000)).expect_err("refused");
	assert_eq!(Error.Code(), Code::NotReady);
	assert!(Error.Detail().contains("instead of a list of steps"));
}

#[test]
fn refuses_sources_that_are_not_a_list() {
	let Pace = TestPace::New();
	let mut Script = Script::New("why", "sol", "answer");
	Script.Sources = json!(42);
	let Page = FakePage::New(Script);
	let Driver = Built(&Page, &Pace);
	let Error = Ask(&Driver, &Pace, Asked("why", "sol"), Span(2_000)).expect_err("refused");
	assert_eq!(Error.Code(), Code::NotReady);
	assert!(Error.Detail().contains("instead of a list of sources"));
}

#[test]
fn the_settlement_state_settles_on_the_third_agreement_and_not_before() {
	use AgenticChatDriver::Settlement;
	let mut Progress = Settlement::Empty;
	assert!(!Progress.Observe(Some("answer")), "first read");
	assert!(!Progress.Observe(Some("answer")), "first agreement");
	assert!(!Progress.Observe(Some("answer")), "second agreement");
	assert!(Progress.Observe(Some("answer")), "third agreement");
	assert_eq!(Progress.Text(), Some("answer"));
}

#[test]
fn the_settlement_state_resets_when_the_thread_stops_growing() {
	use AgenticChatDriver::Settlement;
	let mut Progress = Settlement::Empty;
	Progress.Observe(Some("answer"));
	Progress.Observe(Some("answer"));
	assert!(!Progress.Observe(None), "an unchanged thread confirms nothing");
	assert!(!Progress.Observe(Some("answer")), "the count started again");
}

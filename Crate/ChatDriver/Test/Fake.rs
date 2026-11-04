//! A page that answers the driver's own expressions, with no browser in sight.
//!
//! The TypeScript fake evaluates the expressions against a page-shaped object.
//! Rust has no JavaScript engine in a std-only test, so this fake answers by
//! recognising the expression it was handed - and it recognises it by calling
//! the same function the driver calls, so a change to the protocol breaks the
//! fake rather than passing silently.

use std::{
	sync::{
		Arc,
		Mutex,
		atomic::{AtomicU64, Ordering},
	},
	time::{Duration, Instant},
};

use AgenticChatDriver::{DefaultSelectors, Expressions, Failure, Format, Page, Selectors, Target};
use AgenticRuntime::{Call, Clock, FutureBox, Pace, Signal};
use serde_json::{Value, json};

/// What the fake answers for a thread read.
pub enum Answers {
	/// Successive reads; the last value repeats.
	Fixed(Vec<Value>),
	/// A thread whose newest answer never stops changing.
	Changing,
}

/// What the fake's release does.
pub enum Release {
	/// Finishes at once, the way a healthy release does.
	Quick,
	/// Never finishes, the way a hung one does.
	Hang,
}

/// Everything the fake answers with.
pub struct Script {
	/// The mode the composer reports before anything is pressed.
	pub Mode:Value,
	/// The mode it reports after the toggle is pressed.
	pub ModeAfterPress:Value,
	/// What pressing the toggle answers.
	pub Press:Value,
	/// The model label.
	pub Model:Value,
	/// The thread, read by read.
	pub Thread:Answers,
	/// The collapse control's label.
	pub Collapse:Value,
	/// The trace.
	pub Trace:Value,
	/// The sources.
	pub Sources:Value,
	/// What typing the question answers.
	pub Typed:Value,
	/// What pressing the send control answers.
	pub Submit:Value,
	/// The question the fake expects to be typed, so it can build the same
	/// expression the driver builds.
	pub Question:String,
	/// A failure for every open, instead of a page.
	pub FailOpen:Option<String>,
	/// A failure for every evaluate, instead of an answer.
	pub FailEvaluate:Option<String>,
	/// What the release does.
	pub Release:Release,
}

impl Script {
	/// A page that is in search mode, set to `Model`, and answers `Text`.
	pub fn New(Question:&str, Model:&str, Text:&str) -> Self {
		Self {
			Mode:json!("Search"),
			ModeAfterPress:json!("Search"),
			Press:json!(true),
			Model:json!(Model),
			Thread:Answers::Fixed(vec![json!({ "count": 0, "text": "" }), json!({ "count": 1, "text": Text })]),
			Collapse:Value::Null,
			Trace:json!([]),
			Sources:json!([]),
			Typed:json!(true),
			Submit:json!("sent"),
			Question:Question.to_string(),
			FailOpen:None,
			FailEvaluate:None,
			Release:Release::Quick,
		}
	}
}

/// Which of the driver's expressions the fake was handed.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
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

/// The expression's identity, decided by the driver's own functions.
///
/// A read of the thread has two expressions - one per format - and both are a
/// read of the thread: the fake answers the same value either way, because the
/// shape it answers is the port's, not the page's.
pub fn KindOf(Selectors:&Selectors, Question:&str, Expression:&str) -> Kind {
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
	} else if Expression == Expressions::ReadAnswers(Selectors, Format::Text)
		|| Expression == Expressions::ReadAnswers(Selectors, Format::Markdown)
	{
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

/// A page, scripted.
pub struct FakePage {
	Script:Mutex<Script>,
	Selectors:Selectors,
	/// Every expression kind the driver asked for, in order.
	pub Asked:Mutex<Vec<Kind>>,
	/// How many times the page was opened.
	pub Opens:AtomicU64,
	/// How many times it was released.
	pub Releases:AtomicU64,
	/// How many times the toggle was pressed.
	pub Presses:AtomicU64,
	/// How many times an answer was read.
	pub Reads:AtomicU64,
	/// The last deadline a call handed the page.
	pub Deadline:Mutex<Option<Instant>>,
}

impl FakePage {
	/// A fake with the default selectors.
	pub fn New(Script:Script) -> Arc<Self> { Self::With(DefaultSelectors(), Script) }

	/// A fake with the caller's selectors.
	pub fn With(Selectors:Selectors, Script:Script) -> Arc<Self> {
		Arc::new(Self {
			Script:Mutex::new(Script),
			Selectors,
			Asked:Mutex::new(Vec::new()),
			Opens:AtomicU64::new(0),
			Releases:AtomicU64::new(0),
			Presses:AtomicU64::new(0),
			Reads:AtomicU64::new(0),
			Deadline:Mutex::new(None),
		})
	}

	/// The kinds asked for so far.
	pub fn Asked(&self) -> Vec<Kind> { self.Asked.lock().unwrap().clone() }

	/// Whether the driver ever asked the page to type anything.
	pub fn Typed(&self) -> bool { self.Asked().contains(&Kind::Composer) }

	/// Whether the driver ever asked the page to send.
	pub fn Sent(&self) -> bool { self.Asked().contains(&Kind::Submit) }
}

impl Page for FakePage {
	fn Open<'a>(&'a self, Url:&'a str, Call:Call) -> FutureBox<'a, Result<Target, Failure>> {
		*self.Deadline.lock().unwrap() = Some(Call.Deadline);
		let Fail = self.Script.lock().unwrap().FailOpen.clone();
		Box::pin(async move {
			if let Some(Message) = Fail {
				return Err(Failure::Of(Message));
			}
			self.Opens.fetch_add(1, Ordering::SeqCst);
			Ok(Target { Url:Url.to_string() })
		})
	}

	fn Evaluate<'a>(
		&'a self,
		_Target:&'a Target,
		Expression:&'a str,
		Call:Call,
	) -> FutureBox<'a, Result<Value, Failure>> {
		*self.Deadline.lock().unwrap() = Some(Call.Deadline);
		let Script = self.Script.lock().unwrap();
		let Kind = KindOf(&self.Selectors, &Script.Question, Expression);
		self.Asked.lock().unwrap().push(Kind);
		if let Some(Message) = Script.FailEvaluate.clone() {
			return Box::pin(async move { Err(Failure::Of(Message)) });
		}
		let Answer = match Kind {
			Kind::Mode => {
				if self.Presses.load(Ordering::SeqCst) == 0 {
					Script.Mode.clone()
				} else {
					Script.ModeAfterPress.clone()
				}
			},
			Kind::Press => {
				self.Presses.fetch_add(1, Ordering::SeqCst);
				Script.Press.clone()
			},
			Kind::Model => Script.Model.clone(),
			Kind::Composer => Script.Typed.clone(),
			Kind::Submit => Script.Submit.clone(),
			Kind::Answers => {
				let Read = self.Reads.fetch_add(1, Ordering::SeqCst);
				match &Script.Thread {
					Answers::Changing => json!({ "count": 1, "text": format!("read {Read}") }),
					Answers::Fixed(Values) => {
						let At = (Read as usize).min(Values.len().saturating_sub(1));
						Values.get(At).cloned().unwrap_or_else(|| json!({ "count": 0, "text": "" }))
					},
				}
			},
			Kind::Collapse => Script.Collapse.clone(),
			Kind::Trace => Script.Trace.clone(),
			Kind::Sources => Script.Sources.clone(),
			Kind::Unknown => Value::Null,
		};
		Box::pin(async move { Ok(Answer) })
	}

	fn Dispose<'a>(&'a self, _Target:Target) -> FutureBox<'a, ()> {
		self.Releases.fetch_add(1, Ordering::SeqCst);
		let Hang = matches!(self.Script.lock().unwrap().Release, Release::Hang);
		Box::pin(async move {
			if Hang {
				std::future::pending::<()>().await;
			}
		})
	}
}

/// A clock the test owns, and a wait that advances it.
///
/// The wait moves the clock by at least a millisecond, so a poll loop with a
/// zero interval still terminates: time passing is the test's decision, not the
/// scheduler's.
pub struct TestPace {
	Now:Mutex<Instant>,
	Fire:Mutex<Option<Arc<Signal>>>,
}

impl TestPace {
	/// A pace that starts now.
	pub fn New() -> Arc<Self> { Arc::new(Self { Now:Mutex::new(Instant::now()), Fire:Mutex::new(None) }) }

	/// The same pace, which fires `Signal` on its first wait.
	pub fn Firing(self: &Arc<Self>, Signal:Arc<Signal>) -> Arc<Self> {
		*self.Fire.lock().unwrap() = Some(Signal);
		self.clone()
	}
}

impl Clock for TestPace {
	fn Now(&self) -> Instant { *self.Now.lock().unwrap() }
}

impl Pace for TestPace {
	fn Sleep(&self, Span:Duration) -> FutureBox<'_, ()> {
		*self.Now.lock().unwrap() += Span.max(Duration::from_millis(1));
		if let Some(Signal) = self.Fire.lock().unwrap().take() {
			Signal.Fire();
		}
		Box::pin(std::future::ready(()))
	}
}

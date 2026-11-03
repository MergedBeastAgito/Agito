//! `agentic.chat.ask` over a page the composition root supplies.
//!
//! The four rules of the port, applied in the order that makes each of them
//! mean something:
//!
//! 1. **The mode is a precondition.** The composer's toggle is read before
//!    anything is typed. When it does not read `search` it is pressed once and
//!    read again; when it still does not, the call fails with `wrong-mode` and
//!    nothing was sent. One press, not a loop: a toggle that does not move on
//!    the first press is a page that will not move on the fifth, and a retry
//!    loop would spend the caller's deadline proving it.
//! 2. **The model is a precondition.** The label is compared before anything is
//!    typed, so a page set to another model is refused rather than asked.
//! 3. **Completion is a rule, not a sleep.** The answer's text is read until
//!    three consecutive reads report no change, which is [`Settlement`]. A
//!    sleep would return whatever the network happened to have delivered; a
//!    count of reads would return an answer that had stopped at a paragraph
//!    break. A deadline hit is `not-ready`, never a partial answer presented as
//!    a whole one.
//! 4. **A collapsed answer is `truncated`.** The control that hides the rest of
//!    an answer is looked for inside the newest answer once it has settled,
//!    because the text in the DOM is then demonstrably not all of it.
//!
//! Everything the page does that is not one of those four is `not-ready`: a
//! page that will not open, a page that fails mid-read, a composer that is not
//! there, a send control that never enables. The manifest has four codes and
//! the driver does not invent a fifth; the cause rides along in `detail`
//! instead.
//!
//! Nothing here opens a browser, knows a transport, or reads an environment
//! variable: the page is the composition root's to supply, and the fake in
//! `Test/` is the proof that the rules hold without one.

use std::{sync::Arc, time::Duration};

use AgenticRuntime::{Bound, Call, Expired, FutureBox, Never, Pace};
use serde_json::Value;

use crate::{
	Fn::{
		Expressions::{
			PressMode,
			PressSubmit,
			ReadAnswers,
			ReadCollapse,
			ReadMode,
			ReadModel,
			ReadSources,
			ReadTrace,
			TypeQuestion,
		},
		Read::{Answers, AsAnswers, AsSources, AsSteps, Describe, DescribeText, Normalise, TextOf},
	},
	Struct::{
		Error::Error,
		Port::{Answer, Chat, Page, Question, Target},
		Selectors::{DefaultSelectors, Selectors},
		Settlement::Settlement,
	},
};

/// How long between two reads of an answer while it is being written.
pub const DefaultPoll:Duration = Duration::from_millis(250);

/// A release that has not finished in this long is reported, not waited on.
pub const DefaultDisposeBudget:Duration = Duration::from_secs(5);

/// Where a dispose failure goes, because it may not be returned.
pub type Report = Arc<dyn Fn(&Target, &Error) + Send + Sync>;

/// How one driver is configured.
#[derive(Clone, Debug)]
pub struct Options {
	/// The page to open: the chat product's URL.
	pub Url:String,
	/// How the page is read.
	pub Selectors:Selectors,
	/// How long between two reads of the answer while it is being written.
	pub Poll:Duration,
	/// How long a release may take before it is reported as a failure.
	pub DisposeBudget:Duration,
}

impl Options {
	/// The defaults: the reference product's selectors, a quarter-second
	/// between reads, and five seconds for a release.
	pub fn New(Url:impl Into<String>) -> Self {
		Self {
			Url:Url.into(),
			Selectors:DefaultSelectors(),
			Poll:DefaultPoll,
			DisposeBudget:DefaultDisposeBudget,
		}
	}
}

/// A `Chat` over one `Page`.
pub struct Driver {
	Page:Arc<dyn Page>,
	Options:Options,
	Pace:Arc<dyn Pace>,
	Report:Report,
}

/// Build a `Chat` over one `Page`, on the pace the caller supplied.
///
/// The pace is a parameter for the same reason the clock is: a driver that
/// reached for a timer would decide which executor is running it, and a driver
/// whose waits are supplied can be tested by advancing a clock instead of
/// sleeping.
pub fn Create(Page:Arc<dyn Page>, Options:Options, Pace:Arc<dyn Pace>) -> Driver {
	Driver {
		Page,
		Options,
		Pace,
		// Silent by default: a library that writes to the process's stderr behind a
		// caller's back is a library that corrupts a protocol. The composition root
		// supplies the record.
		Report:Arc::new(|_, _| {}),
	}
}

impl Driver {
	/// The same driver, with somewhere to put a dispose failure.
	pub fn Reporting(mut self, Report:Report) -> Self {
		self.Report = Report;
		self
	}

	/// How much of the caller's budget is left, for a failure's wording.
	fn Budget(&self, Call:&Call) -> Duration { Call.Remaining(self.Pace.as_ref()) }

	/// One expression, run inside whatever budget is left.
	async fn Probe(
		&self,
		Phase:&str,
		Expression:String,
		Target:&Target,
		Call:&Call,
		Budget:Duration,
	) -> Result<Value, Error> {
		if Call.Cancelled() {
			return Err(Error::NotReady(
				format!("{Phase} was cancelled before it started"),
				"the caller's signal had already fired",
			));
		}
		if Call.Expired(self.Pace.as_ref()) {
			return Err(Error::NotReady(
				format!("{Phase} exceeded the {}ms deadline", Budget.as_millis()),
				"the page was still working when the budget ran out",
			));
		}
		let Work = self.Page.Evaluate(Target, &Expression, Call.clone());
		match Bound(Work, Call, self.Pace.as_ref()).await {
			Ok(Ok(Value)) => Ok(Value),
			Ok(Err(Failure)) => {
				Err(Error::NotReady(
					format!("{Phase} could not be read from the page"),
					Failure.Message,
				))
			},
			Err(Expired::Timeout) => {
				Err(Error::NotReady(
					format!("{Phase} exceeded the {}ms deadline", Budget.as_millis()),
					"the page never answered",
				))
			},
			Err(Expired::Cancelled) => {
				Err(Error::NotReady(
					format!("{Phase} was cancelled by the caller"),
					"the signal fired while the page was being asked",
				))
			},
		}
	}

	/// Open the configured page, bounded.
	async fn Open(&self, Call:&Call, Budget:Duration) -> Result<Target, Error> {
		if Call.Cancelled() {
			return Err(Error::NotReady(
				"open was cancelled before it started",
				"the caller's signal had already fired",
			));
		}
		if Call.Expired(self.Pace.as_ref()) {
			return Err(Error::NotReady(
				format!("open exceeded the {}ms deadline", Budget.as_millis()),
				"the page was still working when the budget ran out",
			));
		}
		let Work = self.Page.Open(&self.Options.Url, Call.clone());
		match Bound(Work, Call, self.Pace.as_ref()).await {
			Ok(Ok(Target)) => Ok(Target),
			Ok(Err(Failure)) => Err(Error::NotReady("the chat page could not be opened", Failure.Message)),
			Err(Expired::Timeout) => {
				Err(Error::NotReady("the chat page could not be opened", "the page never answered"))
			},
			Err(Expired::Cancelled) => {
				Err(Error::NotReady(
					"the chat page could not be opened",
					"the caller gave up before the page opened",
				))
			},
		}
	}

	/// One read of the thread, refusing a page that answers something else.
	async fn ReadAnswers(&self, Phase:&str, Target:&Target, Call:&Call, Budget:Duration) -> Result<Answers, Error> {
		let Expression = ReadAnswers(&self.Options.Selectors);
		let Value = self.Probe(Phase, Expression, Target, Call, Budget).await?;
		AsAnswers(&Value).ok_or_else(|| {
			Error::NotReady(
				format!("{Phase} could not be read from the page"),
				format!("the page answered {} instead of a thread", Describe(Some(&Value))),
			)
		})
	}

	/// Release the target, bounded, and never fail.
	async fn Release(&self, Target:&Target) {
		let Deadline = self
			.Pace
			.Now()
			.checked_add(self.Options.DisposeBudget)
			.unwrap_or_else(|| self.Pace.Now());
		let Call = Call { Deadline, Cancellation:Arc::new(Never) };
		let Work = self.Page.Dispose(Target.clone());
		if let Err(Why) = Bound(Work, &Call, self.Pace.as_ref()).await {
			let Error = Error::NotReady(
				format!(
					"dispose of {} exceeded its {}ms budget",
					Target.Url,
					self.Options.DisposeBudget.as_millis()
				),
				Why.Name(),
			);
			// A reporter that panics must not turn a release into a failure the
			// caller never asked for. Under `panic = "abort"` this cannot catch
			// anything, which is the stated cost of the release profile.
			let Report = self.Report.clone();
			let _ = std::panic::catch_unwind(std::panic::AssertUnwindSafe(move || {
				Report(Target, &Error);
			}));
		}
	}

	/// Everything between an open page and a released one.
	async fn Body(&self, Question:&Question, Call:&Call, Budget:Duration, Target:&Target) -> Result<Answer, Error> {
		let Selectors = &self.Options.Selectors;

		// The mode is a precondition, not a preference.
		let mut Mode = self.Probe("mode", ReadMode(Selectors), Target, Call, Budget).await?;
		if Normalise(Some(&Mode)) != "search" {
			self.Probe("mode press", PressMode(Selectors), Target, Call, Budget).await?;
			Mode = self.Probe("mode", ReadMode(Selectors), Target, Call, Budget).await?;
			if Normalise(Some(&Mode)) != "search" {
				return Err(Error::WrongMode(
					"the composer does not offer search mode",
					format!("after pressing the toggle it reads {}", Describe(Some(&Mode))),
				));
			}
		}

		// The model is a precondition, and it is compared before the send.
		let Label = self.Probe("model", ReadModel(Selectors), Target, Call, Budget).await?;
		let Wanted = Value::String(Question.Model.clone());
		if Normalise(Some(&Label)) != Normalise(Some(&Wanted)) {
			return Err(Error::WrongModel(
				"the page is set to a different model",
				format!(
					"wanted {}, the page reads {}",
					serde_json::to_string(&Question.Model).unwrap_or_else(|_| format!("{:?}", Question.Model)),
					Describe(Some(&Label))
				),
			));
		}

		// Correlate the turn: what is already in the thread is not this answer.
		let Before = self.ReadAnswers("answers", Target, Call, Budget).await?;

		let Typed = self
			.Probe("composer", TypeQuestion(Selectors, &Question.Text), Target, Call, Budget)
			.await?;
		if Typed.as_bool() != Some(true) {
			return Err(Error::NotReady(
				"the composer could not be found",
				format!("nothing matched {:?}", Selectors.Composer),
			));
		}

		let Sent = self.Probe("submit", PressSubmit(Selectors), Target, Call, Budget).await?;
		match Sent.as_str() {
			Some("sent") => {},
			Some("disabled") => {
				return Err(Error::NotReady(
					"the question could not be sent",
					"the send control is disabled after the question was typed",
				));
			},
			_ => {
				return Err(Error::NotReady(
					"the question could not be sent",
					format!("nothing matched {:?}", Selectors.Submit),
				));
			},
		}

		// Completion is stable text: unchanged for three consecutive reads.
		let mut Progress = Settlement::Empty;
		let Text = loop {
			// The deadline is checked here, not only inside the read, so a page
			// that keeps writing is reported as an answer that
			// never settled rather than as one slow read.
			if Call.Expired(self.Pace.as_ref()) {
				return Err(Error::NotReady(
					format!("the answer did not settle within the {}ms deadline", Budget.as_millis()),
					format!("the last read was {}", DescribeText(Progress.Text())),
				));
			}
			let Read = self.ReadAnswers("answer", Target, Call, Budget).await?;
			let Current = if Read.Count > Before.Count { Some(Read.Text.as_str()) } else { None };
			if Progress.Observe(Current) {
				break Progress.Text().unwrap_or_default().to_string();
			}
			let Wait = self.Options.Poll.min(Call.Remaining(self.Pace.as_ref()));
			self.Pace.Sleep(Wait).await;
		};

		// A collapsed answer is not the whole answer.
		let Collapsed = self.Probe("collapse", ReadCollapse(Selectors), Target, Call, Budget).await?;
		let Collapsed = TextOf(Some(&Collapsed)).trim().to_string();
		if !Collapsed.is_empty() {
			return Err(Error::Truncated(
				"the answer is collapsed behind a control",
				format!(
					"the page shows {} over {} characters",
					serde_json::to_string(&Collapsed).unwrap_or_else(|_| format!("{Collapsed:?}")),
					Text.chars().count()
				),
			));
		}

		let Trace = self.Probe("trace", ReadTrace(Selectors), Target, Call, Budget).await?;
		let Trace = AsSteps(&Trace).ok_or_else(|| {
			Error::NotReady(
				"the trace could not be read from the page",
				format!("the page answered {} instead of a list of steps", Describe(Some(&Trace))),
			)
		})?;

		let Sources = self.Probe("sources", ReadSources(Selectors), Target, Call, Budget).await?;
		let Sources = AsSources(&Sources).ok_or_else(|| {
			Error::NotReady(
				"the sources could not be read from the page",
				format!("the page answered {} instead of a list of sources", Describe(Some(&Sources))),
			)
		})?;

		let Model = {
			let Read = TextOf(Some(&Label)).trim().to_string();
			if Read.is_empty() { Question.Model.trim().to_string() } else { Read }
		};

		Ok(Answer { Text, Trace, Sources, Model, Mode:"search".to_string() })
	}

	/// The whole call, with the release guaranteed on every path.
	async fn Perform(&self, Question:Question, Call:Call) -> Result<Answer, Error> {
		let Budget = self.Budget(&Call);

		// The port is defined for search mode, so a question that asks for
		// another one is refused here, before a page is opened: there
		// is nothing to verify against a toggle the driver is not
		// willing to use.
		if !Question.Mode.IsSearch() {
			return Err(Error::WrongMode(
				"this port only asks in search mode",
				format!(
					"the question asked for {}",
					serde_json::to_string(Question.Mode.Name())
						.unwrap_or_else(|_| format!("{:?}", Question.Mode.Name()))
				),
			));
		}

		let Target = self.Open(&Call, Budget).await?;
		let Outcome = self.Body(&Question, &Call, Budget, &Target).await;
		self.Release(&Target).await;
		Outcome
	}
}

impl Chat for Driver {
	fn Ask<'a>(&'a self, Question:Question, Call:Call) -> FutureBox<'a, Result<Answer, Error>> {
		Box::pin(async move { self.Perform(Question, Call).await })
	}
}

//! `agito.chat.ask` over a page the composition root supplies.
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
//! # One read, two renderings
//!
//! A call that asks for [`Format::Both`] is the same call with a wider read:
//! the expression returns the rendered text and the page's own Markdown from
//! the same element in the same evaluate, so the answer carries two renderings
//! of one snapshot rather than two renderings of two. None of the four rules
//! changes - the completion rule runs on the text half, which is the half it
//! always ran on, and the marks are computed from that half too, so a `Both`
//! call settles and correlates exactly as a `Text` call does.
//!
//! The pair is not written down anywhere. The driver hands it back and the
//! caller decides: a redirect, a file it owns, a sink of its own.
//!
//! Nothing here opens a browser, knows a transport, or reads an environment
//! variable: the page is the composition root's to supply.

use std::{sync::Arc, time::Duration};

use ::RunTime::{Bound, Call, Expired, FutureBox, Never, Pace};
use serde_json::Value;

use crate::{
	Fn::{
		Expression::{
			PressMode,
			PressSubmit,
			ReadAnswers,
			ReadCollapse,
			ReadComposer,
			ReadMode,
			ReadModel,
			ReadPlatform,
			ReadSources,
			ReadTrace,
			TypeQuestion,
		},
		Read::{
			Answers,
			AsAnswers,
			AsComposer,
			AsInspection,
			AsMac,
			AsSources,
			AsSteps,
			Compare,
			Describe,
			DescribeText,
			Fold,
			Mark,
			Normalise,
			TextOf,
		},
	},
	Struct::{
		Error::Error,
		Format::Format,
		Port::{
			Answer,
			Chat,
			Cleared,
			Composer,
			Control,
			Input,
			Inspection,
			Modifiers,
			Observation,
			Page,
			Press,
			Question,
			Target,
		},
		Selectors::{DefaultSelectors, Selectors},
		Settlement::Settlement,
	},
};

/// How long between two reads of an answer while it is being written.
pub const DefaultPoll:Duration = Duration::from_millis(250);

/// A release that has not finished in this long is reported, not waited on.
pub const DefaultDisposeBudget:Duration = Duration::from_secs(5);

/// The longest question this driver will type into a composer.
///
/// Measured, not chosen, against the reference product's composer - a Lexical
/// contenteditable - through both insertion paths:
///
/// | characters | what the editor did                                    |
/// | ---------- | ------------------------------------------------------ |
/// | 4,219      | inserted, read back in full                            |
/// | 200,000    | inserted, read back in full                            |
/// | 240,000    | inserted, read back in full                            |
/// | 400,000    | inserted, read back in full                            |
/// | 800,000    | reported success, held its previous content            |
/// | 1,200,000  | reported success, held its previous content            |
///
/// The failure mode above the ceiling is the dangerous one: the editing command
/// answers `true` and the editor keeps what it already had, so nothing but a
/// read-back can tell the caller that the question never landed. `getconf
/// ARG_MAX` on the machine that measured this answers 1,048,576, so a question
/// above that never reaches `--question` at all.
///
/// This bound is a quarter of the largest size measured to work. It is not the
/// editor's limit - the editor has none below 400,000 - it is the point past
/// which this driver will not promise an insert landed, and a question above it
/// is refused with its own length instead of surfacing as a read-back mismatch
/// nobody can act on.
///
/// A second measurement belongs beside this one and is not a length at all: an
/// insert sent to a composer whose editor has not finished initialising reports
/// success and holds nothing, at any size. That is what the read-back and the
/// one retry through the input domain exist for.
pub const MaxQuestion:usize = 100_000;

/// Where a dispose failure goes, because it may not be returned.
pub type Report = Arc<dyn Fn(&Target, &Error) + Send + Sync>;

/// How one driver is configured.
#[derive(Clone, Debug)]
pub struct Options {
	/// The page to open: the chat product's URL.
	pub URL:String,
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
	pub fn New(URL:impl Into<String>) -> Self {
		Self {
			URL:URL.into(),
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

	/// One input, run inside whatever budget is left.
	///
	/// The same bounding and the same mapping as [`Driver::Probe`], because a
	/// press that never arrives is the same failure as an expression that never
	/// answers: the page did not do what it was asked, and the caller's deadline
	/// is the only thing that ends the wait.
	async fn Deliver(&self, Phase:&str, Target:&Target, Input:Input, Call:&Call, Budget:Duration) -> Result<(), Error> {
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
		let Work = self.Page.Input(Target, &Input, Call.clone());
		match Bound(Work, Call, self.Pace.as_ref()).await {
			Ok(Ok(())) => Ok(()),
			Ok(Err(Failure)) => {
				Err(Error::NotReady(
					format!("{Phase} could not be delivered to the page"),
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
					"the signal fired while the page was being typed into",
				))
			},
		}
	}

	/// The composer row's popup controls, and which one names the model.
	async fn Row(&self, Phase:&str, Target:&Target, Call:&Call, Budget:Duration) -> Result<Inspection, Error> {
		let Value = self
			.Probe(Phase, ReadModel(&self.Options.Selectors), Target, Call, Budget)
			.await?;
		AsInspection(&Value).ok_or_else(|| {
			Error::NotReady(
				format!("{Phase} could not be read from the page"),
				format!(
					"the page answered {} instead of a list of popup controls",
					Describe(Some(&Value))
				),
			)
		})
	}

	/// What the composer holds, read from the editor and from its rendering.
	async fn Composer(&self, Phase:&str, Target:&Target, Call:&Call, Budget:Duration) -> Result<Composer, Error> {
		let Selectors = &self.Options.Selectors;
		let Value = self.Probe(Phase, ReadComposer(Selectors), Target, Call, Budget).await?;
		AsComposer(&Value).ok_or_else(|| {
			Error::NotReady(
				format!("{Phase} could not be found on the page"),
				format!("nothing matched {:?}", Selectors.Composer),
			)
		})
	}

	/// Whether the page selects everything with Command.
	///
	/// A page that will not say falls back to this process's platform, which is
	/// right whenever the browser is local and a named guess when it is not. It
	/// is a fallback and not an assumption: the clear reads the composer back,
	/// so a wrong chord is a refusal with the composer's length in it rather
	/// than a silent append.
	async fn Platform(&self, Target:&Target, Call:&Call, Budget:Duration) -> bool {
		match self.Probe("platform", ReadPlatform(), Target, Call, Budget).await {
			Ok(Value) => AsMac(&Value),
			Err(_) => cfg!(target_os = "macos"),
		}
	}

	/// Empty the composer with real input, and refuse when it will not empty.
	///
	/// Two attempts, because the first press can land on a page whose editor has
	/// not taken focus yet and a single lost press would read as a composer that
	/// cannot be emptied. Two is not a retry loop: a chord that does nothing on
	/// the second press is a page this driver will not type into.
	async fn ClearAt(&self, Target:&Target, Call:&Call, Budget:Duration) -> Result<Cleared, Error> {
		let Before = self.Composer("composer", Target, Call, Budget).await?;
		let Mac = self.Platform(Target, Call, Budget).await;
		let Mutating = Press::New("a", Modifiers::SelectAll(Mac)).Meaning("selectAll");
		let Deleting = Press::New("Backspace", Modifiers::default()).Meaning("deleteBackward");
		let Pressed = vec![format!("{}+a", if Mac { "Meta" } else { "Control" }), "Backspace".to_string()];
		let mut After = Before.clone();
		for _ in 0..2 {
			if After.Empty() {
				break;
			}
			self.Deliver("composer clear", Target, Input::Press(Mutating.clone()), Call, Budget)
				.await?;
			self.Deliver("composer clear", Target, Input::Press(Deleting.clone()), Call, Budget)
				.await?;
			After = self.Composer("composer", Target, Call, Budget).await?;
		}
		let Cleared = Cleared {
			URL:Target.URL.clone(),
			Before:Before.Text().to_string(),
			After:After.Text().to_string(),
			Pressed,
		};
		if !Cleared.Empty() {
			return Err(Error::NotReady(
				"the composer could not be emptied",
				format!(
					"after {} and a delete the editor still holds {} characters through {}; a composer this driver \
					 cannot empty is refused before the question is typed",
					Cleared.Pressed.join(", "),
					After.Text().chars().count(),
					if After.Source.is_empty() {
						"no readable model"
					} else {
						After.Source.as_str()
					}
				),
			));
		}
		Ok(Cleared)
	}

	/// Put the question in the composer, and prove it landed.
	///
	/// One path, then one different path. The first is an editing command inside
	/// the page; a failure - the command refusing, or the read-back disagreeing -
	/// is retried once through the protocol's own insert, after the composer has
	/// been emptied again, because a half-landed insert is appended to by an
	/// insert and replaced by nothing.
	async fn Write(&self, Text:&str, Target:&Target, Call:&Call, Budget:Duration) -> Result<(), Error> {
		let Selectors = &self.Options.Selectors;
		let Verdict = self
			.Probe("composer", TypeQuestion(Selectors, Text), Target, Call, Budget)
			.await?;
		match TextOf(Some(&Verdict)).as_str() {
			"typed" => {},
			"missing" => {
				return Err(Error::NotReady(
					"the composer could not be found",
					format!("nothing matched {:?}", Selectors.Composer),
				));
			},
			// `refused`, or a page that answered something this driver does not
			// recognise: both mean the edit did not happen, and both are
			// retried below.
			_ => {},
		}
		let mut Held = self.Composer("composer", Target, Call, Budget).await?;
		let mut Checked = Compare(Held.Text(), Text);
		if Checked.Matched {
			return Ok(());
		}
		// The second path: empty the composer through the input domain and type
		// the text the way a paste arrives. This is the path an editor that
		// refuses an editing command still obeys.
		self.ClearAt(Target, Call, Budget).await?;
		self.Deliver("composer", Target, Input::Type(Text.to_string()), Call, Budget)
			.await?;
		Held = self.Composer("composer", Target, Call, Budget).await?;
		Checked = Compare(Held.Text(), Text);
		if !Checked.Matched {
			return Err(Error::NotReady(
				"the composer does not hold the question",
				format!(
					"{} - after an editing command and then the protocol's own insert, so nothing was sent",
					Checked.Detail()
				),
			));
		}
		Ok(())
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
		let Work = self.Page.Open(&self.Options.URL, Call.clone());
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
		let Selectors = &self.Options.Selectors;
		let Expression = ReadAnswers(Selectors, Selectors.Format);
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
					Target.URL,
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
	async fn Body<'a>(
		&'a self,
		Question:&Question,
		Call:&Call,
		Budget:Duration,
		Target:&Target,
		mut Sink:Option<&'a mut (dyn FnMut(&Observation) -> Control + Send)>,
	) -> Result<Answer, Error> {
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

		// The model is a report field, and a gate only when the caller named one.
		//
		// It used to be a gate unconditionally, reading whatever the model
		// selector matched first. On a build whose composer row puts a drafts
		// control before the model control that read "1 draft", so no model name
		// could satisfy it and the value it demanded changed with the number of
		// drafts. A label that cannot be identified is now `unknown` and the call
		// proceeds: a build this driver cannot read is not a build it will refuse
		// a caller over.
		let Seen = self.Row("model", Target, Call, Budget).await?;
		if !Question.Model.trim().is_empty()
			&& let Some(Identified) = &Seen.Model
			&& Fold(Identified) != Fold(&Question.Model)
		{
			return Err(Error::WrongModel(
				"the page is set to a different model",
				format!(
					"wanted {}, the page reads {}",
					serde_json::to_string(&Question.Model).unwrap_or_else(|_| format!("{:?}", Question.Model)),
					serde_json::to_string(Identified).unwrap_or_else(|_| format!("{Identified:?}"))
				),
			));
		}

		// Correlate the turn: what is already in the thread is not this answer.
		let Before = self.ReadAnswers("answers", Target, Call, Budget).await?;
		// A page that is already producing an answer is not a page this call
		// may type into: the question would land in a turn that is still being
		// written, and the answer read back would belong to neither.
		if Before.Busy {
			return Err(Error::NotReady(
				"the page was already producing an answer",
				"the busy control was visible before the question was typed",
			));
		}

		// The composer, before anything is typed into it.
		//
		// Three refusals and one repair, in this order, because each one is
		// cheaper than the one after it and every one of them is cheaper than a
		// wrong answer:
		//
		// - no composer at all is `not-ready` before anything else happens;
		// - an editor whose model disagrees with its DOM is refused rather than
		//   typed into: the DOM is not what the editor will send;
		// - a composer that already holds something is emptied through the input
		//   domain, and a composer that will not empty is a named blocker here
		//   rather than a read-back mismatch after the insert.
		let Held = self.Composer("composer", Target, Call, Budget).await?;
		if Held.Kind.is_empty() {
			return Err(Error::NotReady(
				"the composer could not be found",
				format!("nothing matched {:?}", Selectors.Composer),
			));
		}
		if Held.Diverged() {
			return Err(Error::NotReady(
				"the composer's editor and its rendering disagree",
				format!(
					"the editor holds {} characters through {} and the DOM reads {}; typing into it would append to a \
					 draft that was never read",
					Held.Text().chars().count(),
					if Held.Source.is_empty() { "no readable model" } else { Held.Source.as_str() },
					Held.DOM.chars().count()
				),
			));
		}
		if !Held.Empty() {
			self.ClearAt(Target, Call, Budget).await?;
		}

		// The length, before typing. A question past the bound is refused with
		// its own length instead of arriving as an insert that silently did
		// nothing and a read-back that cannot say why.
		let Length = Question.Text.chars().count();
		if Length > MaxQuestion {
			return Err(Error::NotReady(
				"the question is longer than this driver will type",
				format!(
					"the question is {Length} characters and the limit is {MaxQuestion}; the reference editor was \
					 measured to insert 400,000 and to report success while dropping 800,000, so past this bound a \
					 refusal is the only honest answer"
				),
			));
		}

		self.Write(&Question.Text, Target, Call, Budget).await?;

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
		//
		// The loop carries a pair out of itself: the text it settled on, and
		// the second rendering the same read returned. They leave together
		// because they arrived together - reading the page again after it had
		// settled, to fetch the other rendering, would be reading a second
		// moment and calling it the same answer.
		let mut Progress = Settlement::Empty;
		let (Text, Second) = loop {
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
			// A new turn is recognised by membership, not by position: a
			// virtualised thread reorders and unmounts its turns, so the last
			// element in document order is not reliably the newest turn, and a
			// count is not reliably growing. An answer whose identity was not
			// in the thread before the question is the only reading that
			// survives both.
			let Fresh = !Read.Text.is_empty() && !Before.Marks.contains(&Mark(&Read.Text));
			let Current = if Fresh { Some(Read.Text.as_str()) } else { None };
			// A page that says it is still producing the answer has not
			// finished, whatever its text is doing: a pause between tokens, a
			// reasoning step and a slow code block all leave the text unchanged
			// while the answer is demonstrably incomplete. So a busy read
			// resets the confirmations instead of advancing them.
			let Settled = if Read.Busy {
				Progress = Settlement::Empty;
				false
			} else {
				Progress.Observe(Current)
			};
			// The observation is pushed before the loop acts on it, so an
			// observer sees the read that settled the answer rather than
			// missing the one that mattered.
			if let Some(Sink) = Sink.as_deref_mut() {
				let Seen = Observation { Count:Read.Count, Text:Read.Text.clone(), Busy:Read.Busy, Fresh, Settled };
				if Sink(&Seen) == Control::Stop {
					break (Read.Text, Read.Markdown);
				}
			}
			if Settled {
				// `Progress` holds the text this read confirmed, which is this
				// read's own text: the second rendering beside it is from the
				// same evaluate.
				break (Progress.Text().unwrap_or_default().to_string(), Read.Markdown);
			}
			let Wait = self.Options.Poll.min(Call.Remaining(self.Pace.as_ref()));
			self.Pace.Sleep(Wait).await;
		};

		// A call that asked for both renderings has not got them if the page
		// answered only one. Reporting an empty second rendering would be a
		// claim that the answer serialised to nothing, which is a different
		// claim from a page that never made one.
		let Markdown = match (Selectors.Format, Second) {
			(Format::Both, Some(Markdown)) => Markdown,
			// An answer that is not there has no second rendering to miss: a
			// caller that stopped the observation before the page produced one
			// gets the empty answer every format returns, not an error about a
			// rendering that was never owed.
			(Format::Both, None) if Text.is_empty() => String::new(),
			(Format::Both, None) => {
				return Err(Error::NotReady(
					"the page did not answer the second rendering",
					"the read asked for both and the expression returned no markdown",
				));
			},
			(..) => String::new(),
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

		let Model = Seen.Named().to_string();

		Ok(Answer {
			URL:Target.URL.clone(),
			Text,
			Markdown,
			Trace,
			Sources,
			Model,
			Mode:"search".to_string(),
		})
	}

	/// The whole call, with the release guaranteed on every path.
	async fn Perform<'a>(
		&'a self,
		Question:Question,
		Call:Call,
		Sink:Option<&'a mut (dyn FnMut(&Observation) -> Control + Send)>,
	) -> Result<Answer, Error> {
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
		let Outcome = self.Body(&Question, &Call, Budget, &Target, Sink).await;
		self.Release(&Target).await;
		Outcome
	}
}

impl Chat for Driver {
	fn Ask<'a>(&'a self, Question:Question, Call:Call) -> FutureBox<'a, Result<Answer, Error>> {
		Box::pin(async move { self.Perform(Question, Call, None).await })
	}

	fn Clear<'a>(&'a self, Call:Call) -> FutureBox<'a, Result<Cleared, Error>> {
		Box::pin(async move {
			let Budget = self.Budget(&Call);
			let Target = self.Open(&Call, Budget).await?;
			let Outcome = self.ClearAt(&Target, &Call, Budget).await;
			self.Release(&Target).await;
			Outcome
		})
	}

	fn Inspect<'a>(&'a self, Call:Call) -> FutureBox<'a, Result<Inspection, Error>> {
		Box::pin(async move {
			let Budget = self.Budget(&Call);
			let Target = self.Open(&Call, Budget).await?;
			let Outcome = async {
				let mut Seen = self.Row("model", &Target, &Call, Budget).await?;
				Seen.Composer = self.Composer("composer", &Target, &Call, Budget).await?;
				Seen.URL = Target.URL.clone();
				Ok(Seen)
			}
			.await;
			self.Release(&Target).await;
			Outcome
		})
	}

	fn Observe<'a>(
		&'a self,
		Question:Question,
		Call:Call,
		Sink:&'a mut (dyn FnMut(&Observation) -> Control + Send),
	) -> FutureBox<'a, Result<Answer, Error>> {
		Box::pin(async move { self.Perform(Question, Call, Some(Sink)).await })
	}
}

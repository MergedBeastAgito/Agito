//! Settle: arm, wait, read once.
//!
//! The completion rule is the same claim it always was - an answer is finished
//! when the page stops changing it - and what changed is who decides. The page
//! observes itself: an agent installed by [`crate::Fn::Page::Agent`] holds a
//! `MutationObserver`, a `ResizeObserver` and an `IntersectionObserver` over the
//! answer container, and calls a CDP binding when nothing has moved for the
//! quiet window. This module waits for that call and reads once.
//!
//! Three things it is careful about:
//!
//! - **A quiet window is not a finished answer.** A reasoning step, a slow code
//!   block and a pause between tokens all leave the page still for longer than
//!   the window while the answer is demonstrably unfinished. So the one read is
//!   *checked*: the text must be non-empty, its mark must not already have been
//!   in the thread before the question, and the page must not say it is still
//!   working. A read that fails the check re-arms and waits again.
//! - **`--stall` is a detector, not a budget.** A page that says nothing for
//!   `--stall` is reported as `stalled` and the wait continues; only `--timeout`
//!   ends a call, and it ends it with `settled: false` rather than with partial
//!   text presented as an answer.
//! - **A missing signal is never a failure.** No binding, no agent, no
//!   observers: the caller polls the thread the way it always did, and the
//!   result says so. A page lost to a navigation is re-armed once; a second loss
//!   is a named blocker.

use std::time::Duration;

use ::RunTime::{Bound, Call, Expired};

use crate::{
	Fn::{Page::Agent, Read::Mark},
	Struct::{
		Driver::Driver,
		Error::Error,
		Port::{Control, Observation, Target},
		Settlement::Settlement,
		Signal::{Binding, Settle, Signal},
	},
};

/// What the settle step produced: one read, and how it was decided.
#[derive(Clone, Debug, Default)]
pub struct Outcome {
	/// The answer's text, read once the page stopped changing it.
	pub Text:String,
	/// The same answer read the other way, when the read asked for both.
	pub Markdown:Option<String>,
	/// Whether the newest answer was off screen. A read, never a correction.
	pub Offscreen:bool,
	/// Whether the page's signal decided this, or the stable-reads loop.
	pub Mode:Settle,
	/// Whether the text is an answer that had stopped changing. False only
	/// when an observer stopped the call early, which is a caller's choice.
	pub Settled:bool,
	/// Whether the page went silent for `--stall` at least once.
	pub Stalled:bool,
}

/// The failure a call ends with when its own ceiling expires.
///
/// The detail says `settled: false` in as many words, because the alternative -
/// returning the text read so far - is an answer that was never settled
/// presented as one.
fn Ceiling(Budget:Duration, Mode:Settle, Stalled:bool) -> Error {
	Error::NotReady(
		format!("the answer did not settle before the {}ms ceiling", Budget.as_millis()),
		format!(
			"settled: false; mode: {}; stalled: {Stalled}; the page never said it had stopped changing",
			Mode.Name()
		),
	)
}

impl Driver {
	/// Give the page a binding and install the agent that calls it.
	///
	/// `false` is the page saying it cannot signal - the binding was refused, or
	/// the agent did not install, or the binding is not in the page's global
	/// scope after arming. Every one of those is a reason to read the thread
	/// instead, and none of them is a reason to fail.
	pub(crate) async fn Arm(&self, Target:&Target, Call:&Call, Budget:Duration) -> Result<bool, Error> {
		let Work = self.Page.Bind(Target, Binding, Call.clone());
		match Bound(Work, Call, self.Pace.as_ref()).await {
			Ok(Ok(())) => {},
			Ok(Err(_)) => return Ok(false),
			Err(Expired::Timeout) if Call.Expired(self.Pace.as_ref()) => {
				return Err(Ceiling(Budget, Settle::Signal, false));
			},
			Err(Expired::Timeout) => return Ok(false),
			Err(Expired::Cancelled) => {
				return Err(Error::NotReady(
					"arming the page was cancelled by the caller".to_string(),
					"the caller's signal fired while the page was being given a binding".to_string(),
				));
			},
		}
		let Expression = Agent::Arm(&self.Options.Selectors, self.Options.Quiet);
		let Armed = match self.Probe("arm", Expression, Target, Call, Budget).await {
			Ok(Value) => Value,
			Err(Why) if Call.Expired(self.Pace.as_ref()) => return Err(Why),
			// The page went away between the binding and the agent. The
			// caller's next step is a read, which reports what happened.
			Err(_) => return Ok(false),
		};
		let Flag = |Name:&str| Armed.get(Name).and_then(serde_json::Value::as_bool).unwrap_or(false);
		Ok(Flag("armed") && Flag("bound"))
	}

	/// Wait for the page's next call of the binding.
	///
	/// `Ok(None)` is the stall detector: nothing arrived inside `--stall`, which
	/// says the page went quiet without saying so. `Err` is a page that stopped
	/// answering, which is a loss of the target rather than a slow answer.
	pub(crate) async fn Wait(&self, Target:&Target, Call:&Call, Budget:Duration) -> Result<Option<String>, Error> {
		let Work = self.Page.Signal(Target, Binding, self.Options.Stall, Call.clone());
		match Bound(Work, Call, self.Pace.as_ref()).await {
			Ok(Ok(Payload)) => Ok(Payload),
			Ok(Err(Failure)) => {
				Err(Error::NotReady(
					"the page stopped answering while it was being waited on".to_string(),
					Failure.Message,
				))
			},
			Err(Expired::Timeout) if Call.Expired(self.Pace.as_ref()) => Err(Ceiling(Budget, Settle::Signal, false)),
			Err(Expired::Timeout) => Ok(None),
			Err(Expired::Cancelled) => {
				Err(Error::NotReady(
					"the wait was cancelled by the caller".to_string(),
					"the caller's signal fired while the page was being waited on".to_string(),
				))
			},
		}
	}

	/// The signal path: wait for the page, read once, check the read.
	///
	/// `Ok(None)` means this page cannot signal and the caller should poll.
	pub(crate) async fn Signalled<'a>(
		&self,
		Target:&Target,
		Before:&[String],
		Call:&Call,
		Budget:Duration,
		mut Sink:Option<&mut (dyn FnMut(&Observation) -> Control + Send + 'a)>,
	) -> Result<Option<Outcome>, Error> {
		if !self.Arm(Target, Call, Budget).await? {
			return Ok(None);
		}
		let mut Losses = 0u8;
		let mut Stalled = false;
		let mut Armed = true;
		let mut Progress = Settlement::Empty;
		loop {
			if Call.Expired(self.Pace.as_ref()) {
				return Err(Ceiling(Budget, Settle::Signal, Stalled));
			}
			if !Armed {
				match self.Arm(Target, Call, Budget).await {
					Ok(true) => Armed = true,
					Ok(false) => return Ok(None),
					Err(Why) => return Err(Why),
				}
			}
			let Payload = match self.Wait(Target, Call, Budget).await {
				Ok(Some(Payload)) => Payload,
				Ok(None) => {
					// The stall detector fired. Not a budget: the page is
					// re-armed and the wait continues until the ceiling.
					Stalled = true;
					Armed = false;
					continue;
				},
				Err(Why) if Call.Expired(self.Pace.as_ref()) => return Err(Why),
				Err(_) => {
					// The channel to the page ended - a navigation, a reload,
					// a closed target. One re-arm, and a second loss is named.
					Losses = Losses.saturating_add(1);
					if Losses > 1 {
						return Err(Error::NotReady(
							"the bound page stopped answering twice".to_string(),
							format!(
								"settled: false; mode: signal; the binding was re-armed once and lost again, so {} is \
								 no longer a page this call can hear",
								Target.URL
							),
						));
					}
					if !self.Arm(Target, Call, Budget).await? {
						return Ok(None);
					}
					Armed = true;
					continue;
				},
			};
			let Some(Signal) = Signal::Parse(&Payload) else {
				// The page said something this driver does not read. It is
				// not a page that said nothing, so the wait goes on.
				Armed = false;
				continue;
			};
			// The agent's own reading of the viewport is on the signal, and
			// the read below measures it again at the moment of the read.
			// The read wins: it is later, so it is the one that describes the
			// answer that was actually taken.
			if Signal.Faulted() {
				return Ok(None);
			}
			if !Signal.Quiet() {
				continue;
			}
			// The one read, checked rather than trusted.
			//
			// A quiet window is not a finished answer: measured on the
			// reference product, a long answer pauses for longer than the
			// window while it is still being written, and the page's own stop
			// control is not always on screen to say so. Reading once and
			// returning it gave three of a requested ten items as a settled
			// answer. So the three-read rule survives here as the check it was
			// always meant to be: the text must be new to this call and the
			// page must not say it is still working, and it must still be the
			// same text on the next quiet window. No loop of its own - each
			// confirmation is one more quiet signal, which costs nothing while
			// the page is still writing because the mutations reset the timer.
			let Read = self.ReadAnswers("answer", Target, Call, Budget).await?;
			let Fresh = !Read.Text.is_empty() && !Before.contains(&Mark(&Read.Text));
			let Current = if Fresh && !Read.Busy { Some(Read.Text.as_str()) } else { None };
			if Read.Busy {
				Progress = Settlement::Empty;
			}
			let Settled = Progress.Observe(Current);
			if let Some(Watched) = Sink.as_deref_mut() {
				let Seen = Observation { Count:Read.Count, Text:Read.Text.clone(), Busy:Read.Busy, Fresh, Settled };
				if Watched(&Seen) == Control::Stop {
					return Ok(Some(Outcome {
						Text:Read.Text,
						Markdown:Read.Markdown,
						Offscreen:Read.Offscreen,
						Mode:Settle::Signal,
						Settled:false,
						Stalled,
					}));
				}
			}
			if !Settled {
				Armed = false;
				continue;
			}
			return Ok(Some(Outcome {
				Text:Progress.Text().unwrap_or_default().to_string(),
				Markdown:Read.Markdown,
				Offscreen:Read.Offscreen,
				Mode:Settle::Signal,
				Settled:true,
				Stalled,
			}));
		}
	}

	/// The fallback: read the thread until the text stops moving.
	///
	/// The rule the driver had before the page could say it was done, kept for
	/// the pages that cannot say it and for `--poll`, which asks for it. Three
	/// consecutive reads must report no change, and a page that says it is still
	/// producing the answer resets the count rather than advancing it.
	pub(crate) async fn Polled<'a>(
		&self,
		Target:&Target,
		Before:&[String],
		Call:&Call,
		Budget:Duration,
		mut Sink:Option<&mut (dyn FnMut(&Observation) -> Control + Send + 'a)>,
	) -> Result<Outcome, Error> {
		let mut Progress = Settlement::Empty;
		loop {
			// The deadline is checked here, not only inside the read, so a page
			// that keeps writing is reported as an answer that never settled
			// rather than as one slow read.
			if Call.Expired(self.Pace.as_ref()) {
				return Err(Error::NotReady(
					format!("the answer did not settle before the {}ms ceiling", Budget.as_millis()),
					format!(
						"settled: false; mode: polled; the last read was {}",
						crate::Fn::Read::DescribeText(Progress.Text())
					),
				));
			}
			let Read = self.ReadAnswers("answer", Target, Call, Budget).await?;
			// A new turn is recognised by membership, not by position: a
			// virtualised thread reorders and unmounts its turns, so the last
			// element in document order is not reliably the newest turn, and a
			// count is not reliably growing.
			let Fresh = !Read.Text.is_empty() && !Before.contains(&Mark(&Read.Text));
			let Current = if Fresh { Some(Read.Text.as_str()) } else { None };
			let Settled = if Read.Busy {
				Progress = Settlement::Empty;
				false
			} else {
				Progress.Observe(Current)
			};
			if let Some(Watched) = Sink.as_deref_mut() {
				let Seen = Observation { Count:Read.Count, Text:Read.Text.clone(), Busy:Read.Busy, Fresh, Settled };
				if Watched(&Seen) == Control::Stop {
					return Ok(Outcome {
						Text:Read.Text,
						Markdown:Read.Markdown,
						Offscreen:Read.Offscreen,
						Mode:Settle::Polled,
						Settled:false,
						Stalled:false,
					});
				}
			}
			if Settled {
				return Ok(Outcome {
					Text:Progress.Text().unwrap_or_default().to_string(),
					Markdown:Read.Markdown,
					Offscreen:Read.Offscreen,
					Mode:Settle::Polled,
					Settled:true,
					Stalled:false,
				});
			}
			let Wait = self.Options.Poll.min(Call.Remaining(self.Pace.as_ref()));
			self.Pace.Sleep(Wait).await;
		}
	}
}

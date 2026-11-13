//! What the page says about itself.
//!
//! The driver used to decide that an answer had stopped changing by reading it
//! again and again and comparing: a guess about the page, paid for with a timer
//! that had to be budgeted in advance. The page knows. An agent installed in it
//! observes the answer container and calls a CDP binding when nothing has
//! changed for a quiet window, and this is the shape of what it says when it
//! does.
//!
//! Nothing here is required for an answer to be returned. A page that cannot
//! take the binding, cannot run the agent, or says something this file cannot
//! read is a page the driver reads instead - [`Settle::Polled`] - because a
//! missing signal is a degradation and never a failure.

use serde_json::Value;

/// The binding name the page's agent calls.
///
/// A constant rather than a per-call name: the agent and the listener have to
/// agree, and the only way to disagree about a constant is to edit one of the
/// two. It is also the name a caller that is not Agito can honour, which is
/// what makes the signal a contract rather than an implementation detail.
pub const Binding:&str = "__AgitoSettle";

/// How the driver decided an answer had stopped changing.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Settle {
	/// The page's own agent said it had gone quiet.
	#[default]
	Signal,
	/// The page could not signal, so the text was read until it stopped moving.
	Polled,
}

impl Settle {
	/// The word the result uses.
	pub const fn Name(self) -> &'static str {
		match self {
			Settle::Signal => "signal",
			Settle::Polled => "polled",
		}
	}
}

/// One signal a page's agent sent through [`Binding`].
///
/// Every field is what the page observed rather than what the driver asked for:
/// `Chars` is the newest answer's length when the signal was sent, `Turn` is its
/// identity, `Offscreen` is whether that answer was in the person's viewport,
/// and `At` is the page's own clock. None of them is trusted for the answer
/// itself - the answer is read - and all of them are what makes a run
/// diagnosable when the page and the driver disagree.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Signal {
	/// `busy`, `quiet`, `offscreen` or `error`.
	pub Event:String,
	/// The newest answer's identity, as the read expressions compute it.
	pub Turn:String,
	/// How many characters the newest answer held.
	pub Chars:u64,
	/// Whether the newest answer was off screen when this was sent.
	pub Offscreen:bool,
	/// The page's clock, in milliseconds.
	pub At:u64,
	/// Which installation of the agent sent it, so a stale agent from before a
	/// re-arm can be told from the one that is listening now.
	pub Generation:u64,
	/// How many mutations the agent had counted when it sent this.
	pub Mutations:u64,
}

impl Signal {
	/// Read one payload, or `None` when the page sent something else.
	///
	/// `None` is not a failure: a page whose agent a later version of this
	/// driver installed says more than this one reads, and the driver keeps
	/// waiting rather than giving up on the answer.
	pub fn Parse(Payload:&str) -> Option<Signal> {
		let Value:Value = serde_json::from_str(Payload).ok()?;
		let Object = Value.as_object()?;
		let Event = Object.get("event")?.as_str()?;
		Some(Signal {
			Event:Event.to_string(),
			Turn:Object.get("turn").and_then(Value::as_str).unwrap_or_default().to_string(),
			Chars:Object.get("chars").and_then(Value::as_u64).unwrap_or_default(),
			Offscreen:Object.get("offscreen").and_then(Value::as_bool).unwrap_or_default(),
			At:Object.get("at").and_then(Value::as_u64).unwrap_or_default(),
			Generation:Object.get("generation").and_then(Value::as_u64).unwrap_or_default(),
			Mutations:Object.get("mutations").and_then(Value::as_u64).unwrap_or_default(),
		})
	}

	/// Whether the page says it has stopped changing.
	pub fn Quiet(&self) -> bool { self.Event == "quiet" }

	/// Whether the page says it is working again.
	pub fn Busy(&self) -> bool { self.Event == "busy" }

	/// Whether the page's agent could not do its job.
	pub fn Faulted(&self) -> bool { self.Event == "error" }

	/// The signal as one line, for a run that is being watched.
	pub fn Line(&self) -> String {
		format!(
			"{} chars={} offscreen={} generation={} mutations={}",
			self.Event, self.Chars, self.Offscreen, self.Generation, self.Mutations
		)
	}
}

//! The port `agito.browse` promises, written the way a consumer asks for it.
//!
//! `module.json` names three errors - `browser-unavailable`,
//! `navigation-failed` and `timeout` - and [`crate::Struct::Error`] is where
//! they live. A caller narrows on the variant; a provider that fails in some
//! fourth way is mapped onto one of the three at the boundary, so a consumer
//! never has to guess.

use ::RunTime::{Call, FutureBox};
use serde_json::Value;

use crate::Struct::Error::Error;

/// Who owns the page behind a target, and therefore who may close it.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Ownership {
	/// Somebody else owns the browser: disposal only disconnects.
	Attached,
	/// This session started the browser and may close it.
	Launched,
}

impl Ownership {
	/// The word the port uses.
	pub const fn Name(self) -> &'static str {
		match self {
			Ownership::Attached => "attached",
			Ownership::Launched => "launched",
		}
	}
}

/// The handle a session is bound to.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Target {
	/// The page this session is bound to.
	pub URL:String,
	/// The endpoint the transport behind that page is connected to.
	pub Ws:String,
	/// Whether this session launched the browser or attached to one.
	pub Ownership:Ownership,
}

/// Which modifiers are held while a key is pressed.
///
/// Four booleans rather than the protocol's bit mask, because the mask is an
/// encoding and this is a statement: a caller says which keys it is holding,
/// and the substrate is the one that knows the bits.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct Modifiers {
	/// Alt, or Option.
	pub Alt:bool,
	/// Control.
	pub Control:bool,
	/// Command on macOS, the Windows key elsewhere.
	pub Meta:bool,
	/// Shift.
	pub Shift:bool,
}

impl Modifiers {
	/// The protocol's bit mask: Alt=1, Control=2, Meta=4, Shift=8.
	pub const fn Bit(self) -> i64 {
		(self.Alt as i64) | ((self.Control as i64) << 1) | ((self.Meta as i64) << 2) | ((self.Shift as i64) << 3)
	}
}

/// One key press, as the protocol's input domain takes it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Press {
	/// The key, as the protocol names it: `a`, `Backspace`, `Enter`.
	///
	/// A name and not a code, because the code table belongs to the substrate:
	/// a caller that had to supply `KeyA` and `65` would be encoding a
	/// protocol it does not own, and would break the day a substrate spoke a
	/// different one. A name the substrate does not know is a named failure,
	/// never a key that quietly does nothing.
	pub Key:String,
	/// The modifiers held while it is pressed.
	pub Modifiers:Modifiers,
}

impl Press {
	/// A press of `Key` with `Modifiers` held.
	pub fn New(Key:impl Into<String>, Modifiers:Modifiers) -> Self { Self { Key:Key.into(), Modifiers } }
}

/// One thing the input domain can do to whatever has focus.
///
/// This is the half of the protocol that `Evaluate` cannot reach. An expression
/// can write a DOM property, dispatch a synthetic event and call an editing
/// command; none of those is input. An editor that keeps its own document model
/// ignores all three - it accepts the write, reports success, and keeps the
/// text it already had - and the only thing it cannot ignore is an event the
/// browser produced.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Input {
	/// Press one key, with modifiers held.
	Press(Press),
	/// Insert text at the caret, the way a paste or an IME commit does.
	///
	/// The protocol's second way in, and the one that works where
	/// `document.execCommand` does not: an editor that refuses an editing
	/// command still receives input the browser produced.
	Type(String),
}

/// What every operation is bounded by, plus whether to attach.
#[derive(Clone, Debug)]
pub struct OpenOptions {
	/// The deadline and the caller's cancellation.
	pub Call:Call,
	/// Attach to a browser already listening here instead of launching one.
	pub AttachTo:Option<String>,
	/// Bind the page that browser already has, named by target id or by a URL
	/// prefix, instead of opening a new one.
	///
	/// Only meaningful with `AttachTo`: a browser this session launched has no
	/// page to bind, and asking for one is refused rather than silently
	/// opening a tab.
	pub Page:Option<String>,
}

impl OpenOptions {
	/// Open a new browser, bounded by `Call`.
	pub fn Launch(Call:Call) -> Self { Self { Call, AttachTo:None, Page:None } }

	/// Attach to the browser at `Endpoint` and open a new page in it, bounded
	/// by `Call`.
	pub fn Attach(Endpoint:impl Into<String>, Call:Call) -> Self {
		Self { Call, AttachTo:Some(Endpoint.into()), Page:None }
	}

	/// Attach to the browser at `Endpoint` and bind the page it already has.
	pub fn Existing(Endpoint:impl Into<String>, Page:impl Into<String>, Call:Call) -> Self {
		Self { Call, AttachTo:Some(Endpoint.into()), Page:Some(Page.into()) }
	}

	/// The ownership this option implies: attaching means somebody else owns
	/// it.
	pub fn Ownership(&self) -> Ownership {
		match self.AttachTo {
			None => Ownership::Launched,
			Some(_) => Ownership::Attached,
		}
	}

	/// The page this option asks a launcher for.
	pub fn Want<'a>(&'a self, URL:&'a str) -> crate::Struct::Transport::PageWant<'a> {
		match &self.Page {
			Some(Key) => crate::Struct::Transport::PageWant::Existing(Key),
			None => crate::Struct::Transport::PageWant::Fresh(URL),
		}
	}
}

/// The one port this crate offers.
///
/// `Dispose` returns `()` and cannot fail. A release that fails is a fact about
/// the browser, and a caller that already has its answer must not lose it to a
/// failing cleanup; the reason goes to the session's report, which is the
/// supervisor's record.
pub trait Browse: Send + Sync {
	/// Open a page, returning the handle the session is bound to.
	fn Open<'a>(
		&'a self,
		URL:&'a str,
		Options:OpenOptions,
	) -> FutureBox<'a, Result<Target, crate::Struct::Error::Error>>;
	/// Evaluate an expression in the page and return its value.
	fn Evaluate<'a>(
		&'a self,
		Target:&'a Target,
		Expression:&'a str,
		Call:Call,
	) -> FutureBox<'a, Result<Value, Error>>;
	/// Put one thing into the page's input domain, at whatever has focus.
	///
	/// Separate from [`Browse::Evaluate`] because it is a different claim about
	/// the page: an expression reads and writes the DOM, and input is what the
	/// browser delivers to the element the page has focused. An editor that
	/// keeps its own document model answers the first and obeys the second, so
	/// a caller that needs an edit to be real - a composer cleared, a draft
	/// replaced - has no way to do it with an expression.
	///
	/// Returns nothing: the page does not report what an input did. What
	/// happened is read back with an expression, which is the only honest
	/// source for it.
	fn Input<'a>(&'a self, Target:&'a Target, Input:&'a Input, Call:Call) -> FutureBox<'a, Result<(), Error>>;
	/// Release everything this session holds. Never fails.
	fn Dispose<'a>(&'a self, Target:Target) -> FutureBox<'a, ()>;
}

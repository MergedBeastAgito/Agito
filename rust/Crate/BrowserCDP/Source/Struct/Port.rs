//! The port `agentic.browse` promises, written the way a consumer asks for it.
//!
//! `module.json` names three errors - `browser-unavailable`, `navigation-failed`
//! and `timeout` - and [`crate::Struct::Error`] is where they live. A caller
//! narrows on the variant; a provider that fails in some fourth way is mapped onto
//! one of the three at the boundary, so a consumer never has to guess.

use serde_json::Value;

use AgenticRuntime::{Call, FutureBox};

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
	pub Url: String,
	/// The endpoint the transport behind that page is connected to.
	pub Ws: String,
	/// Whether this session launched the browser or attached to one.
	pub Ownership: Ownership,
}

/// What every operation is bounded by, plus whether to attach.
#[derive(Clone, Debug)]
pub struct OpenOptions {
	/// The deadline and the caller's cancellation.
	pub Call: Call,
	/// Attach to a browser already listening here instead of launching one.
	pub AttachTo: Option<String>,
}

impl OpenOptions {
	/// Open a new browser, bounded by `Call`.
	pub fn Launch(Call: Call) -> Self {
		Self {
			Call,
			AttachTo: None,
		}
	}

	/// Attach to the browser at `Endpoint`, bounded by `Call`.
	pub fn Attach(Endpoint: impl Into<String>, Call: Call) -> Self {
		Self {
			Call,
			AttachTo: Some(Endpoint.into()),
		}
	}

	/// The ownership this option implies: attaching means somebody else owns it.
	pub fn Ownership(&self) -> Ownership {
		match self.AttachTo {
			None => Ownership::Launched,
			Some(_) => Ownership::Attached,
		}
	}
}

/// The one port this crate offers.
///
/// `Dispose` returns `()` and cannot fail. A release that fails is a fact about the
/// browser, and a caller that already has its answer must not lose it to a failing
/// cleanup; the reason goes to the session's report, which is the supervisor's
/// record.
pub trait Browse: Send + Sync {
	/// Open a page, returning the handle the session is bound to.
	fn Open<'a>(
		&'a self,
		Url: &'a str,
		Options: OpenOptions,
	) -> FutureBox<'a, Result<Target, crate::Struct::Error::Error>>;
	/// Evaluate an expression in the page and return its value.
	fn Evaluate<'a>(
		&'a self,
		Target: &'a Target,
		Expression: &'a str,
		Call: Call,
	) -> FutureBox<'a, Result<Value, crate::Struct::Error::Error>>;
	/// Release everything this session holds. Never fails.
	fn Dispose<'a>(&'a self, Target: Target) -> FutureBox<'a, ()>;
}

//! The seam a provider implements, and the fake in `Test/` stands in for.
//!
//! Nothing in this file starts a browser. A [`Launcher`] is supplied by the
//! composition root - it may speak CDP over a WebSocket, drive a subprocess, or
//! answer from a fixture - and the session applies the port's rules to whatever it
//! returns.
//!
//! The contract on a provider:
//!
//! - a transport reports its own death through `Closed`, so the session can tell a
//!   dead socket from a page that is merely not ready;
//! - a failed `Attach`/`Launch` carries a [`crate::Struct::Error`] when it knows why
//!   (`Navigation` for a page that did not load); anything else is read as an
//!   unreachable browser;
//! - `Close` and `Detach` are idempotent: the session may call them on a target it
//!   has already released.

use std::sync::Arc;

use serde_json::Value;

use AgenticRuntime::{Call, FutureBox};

use crate::Struct::Error::Error;

/// One live page connection.
pub trait Transport: Send + Sync {
	/// The endpoint this transport is bound to; it becomes `Target::Ws`.
	fn Endpoint(&self) -> &str;
	/// True once the connection is gone. A closed transport is dead, not slow.
	fn Closed(&self) -> bool;
	/// Ask the page one expression and return what it produced.
	fn Evaluate<'a>(
		&'a self,
		Expression: &'a str,
		Call: Call,
	) -> FutureBox<'a, Result<Value, Error>>;
	/// Close the page this session launched.
	fn Close(&self) -> FutureBox<'_, ()>;
	/// Let go of a page somebody else owns, without closing it.
	fn Detach(&self) -> FutureBox<'_, ()>;
}

/// Where a transport comes from: an existing browser, or a new one.
pub trait Launcher: Send + Sync {
	/// Attach to a browser already listening at `Endpoint`, bound to `Url`.
	fn Attach<'a>(
		&'a self,
		Endpoint: &'a str,
		Url: &'a str,
		Call: Call,
	) -> FutureBox<'a, Result<Arc<dyn Transport>, Error>>;
	/// Start a browser and open `Url` in it.
	fn Launch<'a>(
		&'a self,
		Url: &'a str,
		Call: Call,
	) -> FutureBox<'a, Result<Arc<dyn Transport>, Error>>;
}

//! The seam a provider implements.
//!
//! Nothing in this file starts a browser. A [`Launcher`] is supplied by the
//! composition root - it may speak CDP over a WebSocket, drive a subprocess, or
//! answer from a fixture - and the session applies the port's rules to whatever
//! it returns.
//!
//! The contract on a provider:
//!
//! - a transport reports its own death through `Closed`, so the session can
//!   tell a dead socket from a page that is merely not ready;
//! - a failed `Attach`/`Launch` carries a [`crate::Struct::Error`] when it
//!   knows why (`Navigation` for a page that did not load); anything else is
//!   read as an unreachable browser;
//! - `Close` and `Detach` are idempotent: the session may call them on a target
//!   it has already released.

use std::sync::Arc;

use ::RunTime::{Call, FutureBox};
use serde_json::Value;

use crate::Struct::Error::Error;

/// One live page connection.
pub trait Transport: Send + Sync {
	/// The endpoint this transport is bound to; it becomes `Target::Ws`.
	fn Endpoint(&self) -> &str;
	/// The URL the page is actually on, which is not always the URL a caller
	/// asked for: a page bound by target id keeps whatever it was showing.
	fn URL(&self) -> &str;
	/// True once the connection is gone. A closed transport is dead, not slow.
	fn Closed(&self) -> bool;
	/// Ask the page one expression and return what it produced.
	fn Evaluate<'a>(&'a self, Expression:&'a str, Call:Call) -> FutureBox<'a, Result<Value, Error>>;
	/// Close the page this session launched.
	fn Close(&self) -> FutureBox<'_, ()>;
	/// Let go of a page somebody else owns, without closing it.
	fn Detach(&self) -> FutureBox<'_, ()>;
}

/// What an attach asks for: a page that already exists, or a new one.
///
/// Attaching to a browser and then opening a page in it is not the same
/// operation as binding the page a person is already looking at. The first
/// leaves a tab behind that nobody asked for; the second is the only way to act
/// "on this tab". So the difference is a value the caller states, not a
/// boolean a provider guesses.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PageWant<'a> {
	/// A new page at this URL.
	Fresh(&'a str),
	/// The page already open whose target id, or failing that whose URL
	/// starts with this, is this.
	Existing(&'a str),
}

/// Where a transport comes from: an existing browser, or a new one.
pub trait Launcher: Send + Sync {
	/// Attach to a browser already listening at `Endpoint`, bound to `Want`.
	fn Attach<'a>(
		&'a self,
		Endpoint:&'a str,
		Want:PageWant<'a>,
		Call:Call,
	) -> FutureBox<'a, Result<Arc<dyn Transport>, Error>>;
	/// Start a browser and open `URL` in it.
	fn Launch<'a>(&'a self, URL:&'a str, Call:Call) -> FutureBox<'a, Result<Arc<dyn Transport>, Error>>;
}

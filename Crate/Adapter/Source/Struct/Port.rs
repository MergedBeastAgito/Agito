//! The port `agito.adapter` promises, written the way a composition root
//! holds it.
//!
//! Three methods and no more. An adapter is asked what it serves, asked to do
//! one thing within a deadline, and asked to let go - the same three questions
//! `agito-browser-cdp`'s `Browse` answers, because the same thing has to be
//! possible for a substrate this workspace did not write.

use ::RunTime::{Call, FutureBox};
use serde_json::Value;

use crate::Struct::Error::Error;

/// One substrate, behind the port its own `module.json` names.
///
/// # Why every method takes `&self`
///
/// Object safety. A trait method without a receiver - the shape the NFSU2
/// driver's `ADAPTER.md` first proposed for [`Adapter::Port`] - cannot be called
/// on `dyn Adapter`, so the trait could not be held as `Arc<dyn Adapter>` at
/// all, and a consumer would be back to naming the concrete crate. Every method
/// here borrows for `'a` and returns a future bounded by that borrow, which is
/// what makes the trait usable as a value rather than as a generic parameter.
pub trait Adapter: Send + Sync {
	/// The port this adapter provides, as its own `module.json` names it.
	///
	/// `agito verify` cannot check this: it reads a file and never constructs
	/// the adapter. [`crate::Fn::Serves::Serves`] is what compares the two, at
	/// the one place both are in hand - the composition root that built it.
	fn Port(&self) -> &'static str;

	/// One request, bounded by the caller's deadline.
	///
	/// The request is the adapter's own encoding, JSON by convention and
	/// nothing more. A universal request type every adapter shares would be the
	/// ports crate this workspace's design refuses: the first substrate with a
	/// shape of its own would force an edit on every other adapter, which is
	/// exactly the coupling the consumer-owned port exists to avoid.
	///
	/// The answer is a [`Value`] for the same reason a capability's port
	/// answers one: the caller reads the shape it asked for, and a provider that
	/// cannot produce it fails with a code rather than with a type error at the
	/// boundary.
	fn Call<'a>(&'a self, Request:&'a str, Call:Call) -> FutureBox<'a, Result<Value, Error>>;

	/// Release everything this adapter holds. Never fails.
	///
	/// An adapter that owns a session, a socket or a device has to be able to
	/// let go of it, and a release that can fail is a release a caller skips.
	/// `agito-browser-cdp` states the same rule for a browser: disposal closes
	/// only what the session launched, and reports rather than returns a
	/// failure.
	fn Dispose<'a>(&'a self) -> FutureBox<'a, ()>;
}

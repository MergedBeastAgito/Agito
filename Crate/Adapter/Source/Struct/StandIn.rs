//! A scripted adapter, for a consumer that tests its own use of the port.
//!
//! Named as a test double, the way `agentic-runtime`'s `StandIn` is. A consumer
//! that holds `Arc<dyn Adapter>` needs to exercise its own code with no
//! substrate behind it, and a fake that ships with the port is the only one
//! every consumer can share: a copy in each consumer's test tree is a copy that
//! drifts from the port it is supposed to stand in for.
//!
//! Two rules it keeps, because a stand-in that broke them would make a caller's
//! own bug look like a port defect: an exhausted queue answers `null` rather
//! than panicking or failing, and a poisoned lock is read through rather than
//! propagated - a panicked test must not make the double unavailable.

use std::{
	collections::VecDeque,
	sync::{
		Mutex,
		MutexGuard,
		atomic::{AtomicBool, Ordering},
	},
};

use AgenticRuntime::{Call, FutureBox};
use serde_json::Value;

use crate::Struct::{Error::Error, Port::Adapter};

/// An adapter that answers what it was told to answer, in the order it was told.
#[derive(Debug)]
pub struct StandIn {
	/// The port it reports.
	Port:&'static str,
	/// Answers not yet given, oldest first.
	Answers:Mutex<VecDeque<Result<Value, Error>>>,
	/// Every request it was handed, in the order it arrived.
	Requests:Mutex<Vec<String>>,
	/// Whether it was released.
	Disposed:AtomicBool,
}

impl StandIn {
	/// A stand-in for the port named.
	pub fn New(Port:&'static str) -> Self {
		Self {
			Port,
			Answers:Mutex::new(VecDeque::new()),
			Requests:Mutex::new(Vec::new()),
			Disposed:AtomicBool::new(false),
		}
	}

	/// Queue one answer. The nth call receives the nth answer queued.
	pub fn Queue(&self, Answer:Result<Value, Error>) { Lock(&self.Answers).push_back(Answer); }

	/// Every request handed to it so far.
	pub fn Requests(&self) -> Vec<String> { Lock(&self.Requests).clone() }

	/// Whether it was released.
	pub fn Disposed(&self) -> bool { self.Disposed.load(Ordering::SeqCst) }
}

/// A lock whose poisoning is not a failure: a panicked test must not make the
/// double unavailable.
fn Lock<T>(Guard:&Mutex<T>) -> MutexGuard<'_, T> { Guard.lock().unwrap_or_else(|Poison| Poison.into_inner()) }

impl Adapter for StandIn {
	fn Port(&self) -> &'static str { self.Port }

	fn Call<'a>(&'a self, Request:&'a str, _Call:Call) -> FutureBox<'a, Result<Value, Error>> {
		Lock(&self.Requests).push(Request.to_string());
		let Answer = Lock(&self.Answers).pop_front().unwrap_or(Ok(Value::Null));
		Box::pin(std::future::ready(Answer))
	}

	fn Dispose<'a>(&'a self) -> FutureBox<'a, ()> {
		self.Disposed.store(true, Ordering::SeqCst);
		Box::pin(std::future::ready(()))
	}
}

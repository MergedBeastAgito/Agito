//! `Browse` over a transport the composition root supplies.
//!
//! The session is the supervisor the port's rules talk about: it owns the
//! transport map, the pending operations and the cleanup. A caller's
//! cancellation stops *its wait* and nothing else - it never closes an attached
//! page - and disposal is the only thing that ends a transport's life.
//!
//! Every classification rule lives in [`crate::Fn::Classify`], in one place:
//!
//! - silence is a timeout. Nothing but the deadline can end a page that never
//!   answers, so a hang is reported as `timeout` and never as a dead transport;
//! - a closed transport is `browser-unavailable`, whatever error arrived with
//!   it, because the socket is gone rather than the page being slow;
//! - a live transport that could not be used is `navigation-failed`.

use std::{
	collections::{HashMap, HashSet},
	sync::{Arc, Mutex, MutexGuard},
	time::Duration,
};

use ::RunTime::{Bound, Call, Expired, FutureBox, Never, Pace};
use serde_json::Value;

use crate::{
	Fn::Classify::ClassifyPage,
	Struct::{
		Error::Error,
		Port::{Browse, OpenOptions, Ownership, Target},
		Transport::{Launcher, Transport},
	},
};

/// A release that has not finished in this long is reported, not waited on.
pub const DefaultDisposeBudget:Duration = Duration::from_secs(5);

/// Where a dispose failure goes, because it may not be returned.
pub type Report = Arc<dyn Fn(&Target, &Error) + Send + Sync>;

/// How one session is configured.
pub struct Options {
	/// Where a dispose failure goes.
	pub Report:Report,
	/// How long a release may take before it is reported as a failure.
	pub DisposeBudget:Duration,
}

impl Default for Options {
	fn default() -> Self {
		Self {
			// Silent by default: a library that writes to the process's stderr
			// behind a caller's back is a library that corrupts a protocol.
			Report:Arc::new(|_, _| {}),
			DisposeBudget:DefaultDisposeBudget,
		}
	}
}

/// A session is one `Browse` bound to one launcher.
pub struct Session {
	Launcher:Arc<dyn Launcher>,
	Pace:Arc<dyn Pace>,
	Options:Options,
	/// What this session holds, by endpoint.
	Held:Mutex<HashMap<String, Arc<dyn Transport>>>,
	/// What it has already released, so a second dispose is silent.
	Released:Mutex<HashSet<String>>,
}

/// Build a session over one launcher, on the pace the caller supplied.
pub fn Create(Launcher:Arc<dyn Launcher>, Pace:Arc<dyn Pace>, Options:Options) -> Session {
	Session {
		Launcher,
		Pace,
		Options,
		Held:Mutex::new(HashMap::new()),
		Released:Mutex::new(HashSet::new()),
	}
}

impl Session {
	fn Lock<T>(Guard:&Mutex<T>) -> MutexGuard<'_, T> { Guard.lock().unwrap_or_else(|Poison| Poison.into_inner()) }

	/// A reporter that throws must not turn dispose into a throw.
	fn ReportQuietly(&self, Target:&Target, Why:&Error) {
		let Report = self.Options.Report.clone();
		let _ = std::panic::catch_unwind(std::panic::AssertUnwindSafe(move || {
			Report(Target, Why);
		}));
	}

	/// Release one transport, bounded, and report why if it did not finish.
	async fn Release(&self, Transport:Arc<dyn Transport>, Ownership:Ownership) -> Option<Error> {
		let Deadline = self
			.Pace
			.Now()
			.checked_add(self.Options.DisposeBudget)
			.unwrap_or_else(|| self.Pace.Now());
		let Call = Call { Deadline, Cancellation:Arc::new(Never) };
		let Work = match Ownership {
			Ownership::Launched => Transport.Close(),
			Ownership::Attached => Transport.Detach(),
		};
		match Bound(Work, &Call, self.Pace.as_ref()).await {
			Ok(()) => None,
			Err(Why) => {
				Some(Error::Timeout(
					format!("dispose exceeded its {}ms budget", self.Options.DisposeBudget.as_millis()),
					Why.Name(),
				))
			},
		}
	}
}

impl Browse for Session {
	fn Open<'a>(&'a self, URL:&'a str, Options:OpenOptions) -> FutureBox<'a, Result<Target, Error>> {
		Box::pin(async move {
			let Ownership = Options.Ownership();
			let Phase = match Ownership {
				Ownership::Launched => "open (launch)",
				Ownership::Attached => "open (attach)",
			};
			// A caller that has already given up must not start a browser.
			if Options.Call.Cancelled() {
				return Err(Error::Timeout(
					format!("{Phase} was cancelled before it started"),
					"the signal was already fired",
				));
			}
			let Work = match &Options.AttachTo {
				None if Options.Page.is_some() => {
					return Err(Error::Unavailable(
						"a page can only be bound on a browser this session attached to",
						"the options asked for an existing page but named no endpoint",
					));
				},
				None => self.Launcher.Launch(URL, Options.Call.clone()),
				Some(Endpoint) => self.Launcher.Attach(Endpoint, Options.Want(URL), Options.Call.clone()),
			};
			let Transport = match Bound(Work, &Options.Call, self.Pace.as_ref()).await {
				Ok(Ok(Transport)) => Transport,
				Ok(Err(Why)) => return Err(Why),
				Err(Expired::Timeout) => {
					return Err(Error::Timeout(
						format!("{Phase} exceeded its deadline"),
						"the browser never answered",
					));
				},
				Err(Expired::Cancelled) => {
					return Err(Error::Timeout(
						format!("{Phase} was cancelled by the caller"),
						"the signal fired while the browser was being opened",
					));
				},
			};
			let Target = Target { URL:Transport.URL().to_string(), Ws:Transport.Endpoint().to_string(), Ownership };
			if Transport.Closed() {
				if let Some(Why) = self.Release(Transport.clone(), Ownership).await {
					self.ReportQuietly(&Target, &Why);
				}
				return Err(Error::Unavailable(
					"the transport was already closed when it arrived",
					format!("{URL} on {}", Target.Ws),
				));
			}
			Self::Lock(&self.Held).insert(Target.Ws.clone(), Transport);
			Self::Lock(&self.Released).remove(&Target.Ws);
			Ok(Target)
		})
	}

	fn Evaluate<'a>(&'a self, Target:&'a Target, Expression:&'a str, Call:Call) -> FutureBox<'a, Result<Value, Error>> {
		Box::pin(async move {
			let Transport = Self::Lock(&self.Held).get(&Target.Ws).cloned();
			let Some(Transport) = Transport else {
				return Err(Error::Unavailable(
					"this session holds no transport for the target",
					format!("{} was never opened here, or it was already disposed", Target.Ws),
				));
			};
			if Transport.Closed() {
				return Err(Error::Unavailable(
					"the transport is closed",
					format!("{} is no longer connected", Target.Ws),
				));
			}
			// A caller that has already given up must not ask the page
			// anything.
			if Call.Cancelled() {
				return Err(Error::Timeout(
					"evaluate was cancelled before it started",
					"the signal was already fired",
				));
			}
			let Work = Transport.Evaluate(Expression, Call.clone());
			match Bound(Work, &Call, self.Pace.as_ref()).await {
				Ok(Ok(Value)) => Ok(Value),
				Ok(Err(Why)) => Err(ClassifyPage(Why, Transport.Closed())),
				Err(Expired::Timeout) => {
					Err(Error::Timeout("evaluate exceeded its deadline", "the page never answered"))
				},
				Err(Expired::Cancelled) => {
					Err(Error::Timeout(
						"evaluate was cancelled by the caller",
						"the signal fired while the page was being asked",
					))
				},
			}
		})
	}

	fn Dispose<'a>(&'a self, Target:Target) -> FutureBox<'a, ()> {
		Box::pin(async move {
			// Never throws: not on a failed release, and not on a target this
			// session never opened.
			if !Self::Lock(&self.Released).insert(Target.Ws.clone()) {
				return;
			}
			let Transport = Self::Lock(&self.Held).remove(&Target.Ws);
			let Some(Transport) = Transport else {
				return;
			};
			if let Some(Why) = self.Release(Transport, Target.Ownership).await {
				self.ReportQuietly(&Target, &Why);
			}
		})
	}
}

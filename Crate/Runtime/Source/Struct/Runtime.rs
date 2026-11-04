//! The runner: admit a run, run an effect, present its ticket, clean up without
//! throwing.
//!
//! Everything here is delegation. The runner holds one generation, one
//! registration list, one clock and one [`Authority`], and every verb it offers
//! is the authority's verb with the manifest check in front of it. It keeps no
//! lease, no fence and no record, so it cannot become the second authority the
//! review warned about: there is nothing in this file to disagree with.
//!
//! Four mechanisms carry the rules the port states, and only the first is the
//! runner's:
//!
//! 1. **The manifest it reports is the manifest it enforces.** `Admit` answers
//!    from the binding it was constructed with, and nothing else, which is what
//!    makes that sentence checkable rather than asserted.
//! 2. **A record per resource, and a fence counter that outlives it.** The
//!    authority's. The counter is what was ever issued, so deleting a record on
//!    release cannot make an old number current again.
//! 3. **An effect carries the authority it was admitted under.** `WithLease` asks
//!    the authority to admit the effect, and hands the work the [`Ticket`] it
//!    minted. The admission is an optimisation: it refuses doomed work early and
//!    charges the budget. The guarantee is at the destination
//!    ([`crate::Struct::Port::Destination`]), which re-reads the record when the
//!    effect lands and refuses a ticket whose fence, generation, holder or expiry
//!    is no longer current. A check in this file cannot see the instant an effect
//!    writes; a check where it lands can.
//! 4. **Unsettled local futures are counted, and a takeover waits for them.**
//!    Also the authority's, and this file cannot read the count. That is the
//!    point: the count is accounting, not fencing, and a runner that could read
//!    it would be a runner that could decide.

use std::{
	collections::BTreeMap,
	sync::Arc,
	time::{Duration, Instant},
};

use crate::Struct::{
	Authority::{Admitted, Authority},
	Call::{Clock, FutureBox},
	Error::Error,
	Port::{Admission, Binding, Budget, Lease, Registration, Request, Runtime, StateClass, Ticket},
};

/// What the composition root hands the runner.
pub struct Options {
	/// The generation the composition root bound this runtime to.
	pub Generation:u64,
	/// What it may admit, by name, with the state class each declares.
	pub Capabilities:Vec<Registration>,
	/// The clock leases and budgets are read on.
	pub Clock:Arc<dyn Clock>,
}

impl Options {
	/// A runtime with no capabilities and the wall clock.
	pub fn New(Generation:u64) -> Self {
		Self {
			Generation,
			Capabilities:Vec::new(),
			Clock:Arc::new(crate::Struct::Call::Wall),
		}
	}

	/// The same options, with one more registration.
	pub fn With(mut self, Name:impl Into<String>, State:StateClass) -> Self {
		self.Capabilities.push(Registration { Name:Name.into(), State });
		self
	}

	/// The same options, on another clock.
	pub fn On(mut self, Clock:Arc<dyn Clock>) -> Self {
		self.Clock = Clock;
		self
	}
}

/// The runner: one generation, one registration list, one clock, one authority.
pub struct Runner<A: Authority> {
	Generation:u64,
	Capabilities:BTreeMap<String, Registration>,
	Clock:Arc<dyn Clock>,
	Authority:A,
}

/// Bind a runner to one generation, one registration list and one authority.
///
/// A duplicate registration is refused here rather than resolved, because two
/// declarations of one name is a composition root disagreeing with itself and
/// picking one silently is how a pre-warm harness prepares the wrong thing.
///
/// The authority is an argument and not a default. A composition root that has a
/// database passes the adapter that calls it; one that does not passes
/// [`crate::Struct::StandIn::StandIn`] and is, by the name it had to type, running
/// without the authority.
pub fn Create<A: Authority>(Options:Options, Authority:A) -> Result<Runner<A>, String> {
	let mut Capabilities = BTreeMap::new();
	for Registration in Options.Capabilities {
		if Capabilities.contains_key(&Registration.Name) {
			return Err(format!("{} is registered twice", Registration.Name));
		}
		Capabilities.insert(Registration.Name.clone(), Registration);
	}
	Ok(Runner {
		Generation:Options.Generation,
		Capabilities,
		Clock:Options.Clock,
		Authority,
	})
}

impl<A: Authority> Runner<A> {
	/// The authority this runner delegates to.
	pub fn Authority(&self) -> &A { &self.Authority }
}

impl<A: Authority + Clone> Runner<A> {
	/// A handle to the destination an effect presents its ticket to.
	///
	/// Cloning a handle is not cloning an authority: both handles name the same
	/// table, which is why a destination handed to another thread still refuses
	/// what this one would. Hand this to whatever performs an effect. An effect
	/// that never presents its ticket leaves no record, and an unrecorded effect
	/// is invisible to the state: the runtime cannot make the presentation happen,
	/// but it can make the absence observable.
	pub fn Destination(&self) -> A { self.Authority.clone() }
}

impl<A: Authority> Runtime for Runner<A> {
	fn Admit<'a>(&'a self, Request:Request) -> FutureBox<'a, Admission> {
		Box::pin(async move {
			if Request.Generation != self.Generation {
				return Admission::Refused(format!(
					"admission-refused: generation {} is not bound; this runtime enforces generation {}",
					Request.Generation, self.Generation
				));
			}
			let Some(Registered) = self.Capabilities.get(&Request.Capability) else {
				return Admission::Refused(format!(
					"admission-refused: {} is not registered with this runtime",
					Request.Capability
				));
			};
			if Registered.State != Request.State {
				return Admission::Refused(format!(
					"admission-refused: {} declares {}, not {}",
					Registered.Name,
					Registered.State.Name(),
					Request.State.Name()
				));
			}
			Admission::Admitted
		})
	}

	fn Claim<'a>(
		&'a self,
		Resource:&'a str,
		Ttl:Duration,
		Budget:Option<Budget>,
	) -> FutureBox<'a, Result<Lease, Error>> {
		self.Authority.Claim(Resource, Ttl, Budget)
	}

	fn Renew<'a>(&'a self, Lease:Lease, Ttl:Duration) -> FutureBox<'a, Result<Lease, Error>> {
		self.Authority.Renew(Lease, Ttl)
	}

	fn WithLease<'a, T, Write>(&'a self, Lease:Lease, Write:Write) -> FutureBox<'a, Result<T, Error>>
	where
		T: Send + 'a,
		Write: FnOnce(Ticket) -> FutureBox<'a, T> + Send + 'a, {
		Box::pin(async move {
			// One delegation, then the work, then the settlement. The runner
			// never inspects the admission: the ticket is the effect's, and the
			// accounting guard is the authority's.
			let Admitted:Admitted = self.Authority.Admit(Lease).await?;
			let Ticket = Admitted.Ticket.clone();
			let Value = Write(Ticket).await;
			drop(Admitted);
			Ok(Value)
		})
	}

	fn Release<'a>(&'a self, Lease:Lease) -> FutureBox<'a, ()> { self.Authority.Release(Lease) }

	fn Now(&self) -> Instant { self.Clock.Now() }

	fn Reflect(&self) -> Binding {
		Binding {
			Generation:self.Generation,
			Capabilities:self.Capabilities.values().cloned().collect(),
		}
	}

	fn Clock(&self) -> &dyn Clock { self.Clock.as_ref() }
}

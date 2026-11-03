//! The port `agentic.runtime` promises, written the way a consumer asks for it.
//!
//! The port states more than data shapes, because two runtimes that agreed on
//! the shapes alone would still disagree about when authority ends:
//!
//! - **Admission is per effect, not per run.** `WithLease` re-checks the fence
//!   before every write. A lease that expired while its worker was still
//!   running refuses the next write instead of letting it land late: the worker
//!   keeps existing, and stops having authority.
//! - **A fence is a number, and expiry is a clock.** A monotonically increasing
//!   fence alone does not prove a lease is live - an expired worker can arrive
//!   before any newer token has reached the resource - so every check compares
//!   the presented fence against the record *and* the record's expiry against
//!   the clock. Either one failing is a refusal.
//! - **A held resource is refused, not queued.** A second claim while the first
//!   is live is `lease-held`. A queue would turn a scheduling decision into a
//!   promise the runtime cannot keep.
//! - **A budget belongs to one run.** The deadline and the operation count are
//!   the caller's, supplied when the run claims its resource. Nothing is
//!   pooled.
//! - **Release never throws, and is safe twice.** Cleanup runs on the failure
//!   path by definition, so it cannot be the step that turns a handled failure
//!   into an unhandled one.
//!
//! One port shape changed in the port: `claim(resource, ttl, budget?)` and
//! `now()`/`reflect()` are on this one trait instead of a `Policy` extension,
//! because Rust has no method overloading and an extension trait that repeats
//! `claim` with a different arity cannot exist.

use std::time::{Duration, Instant};

use crate::Struct::{
	Call::{Clock, FutureBox},
	Error::Error,
};

/// The four state classes, in the order the manifest names them.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum StateClass {
	/// Nothing is retained between runs.
	Stateless,
	/// A transport or session is retained.
	HoldsTransport,
	/// An identity is retained.
	HoldsIdentity,
	/// Workspace-scoped state is retained.
	HoldsWorkspace,
}

impl StateClass {
	/// The string the manifest uses.
	pub const fn Name(self) -> &'static str {
		match self {
			StateClass::Stateless => "stateless",
			StateClass::HoldsTransport => "holds-transport",
			StateClass::HoldsIdentity => "holds-identity",
			StateClass::HoldsWorkspace => "holds-workspace",
		}
	}

	/// Every class, so a harness can compare the manifest against the type.
	pub const fn All() -> [StateClass; 4] {
		[
			StateClass::Stateless,
			StateClass::HoldsTransport,
			StateClass::HoldsIdentity,
			StateClass::HoldsWorkspace,
		]
	}

	/// The class a manifest string names, if it names one of the four.
	pub fn Parse(Value:&str) -> Option<StateClass> { StateClass::All().into_iter().find(|State| State.Name() == Value) }
}

/// The four state classes, in the order the manifest names them.
pub const States:[StateClass; 4] = StateClass::All();

/// One authority over one resource, granted for a bounded time.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Lease {
	/// This grant's identity. Unique within the process that issued it.
	pub Id:String,
	/// What it grants: a name the composition root chose, never a capability's.
	pub Resource:String,
	/// Monotonic per resource, never reused, never reset by a release.
	pub Fence:u64,
	/// When authority ends, on the runtime's clock. Half-open: the instant
	/// itself is already outside it.
	pub ExpiresAt:Instant,
}

/// One run's allowance: a deadline and a count of effects.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Budget {
	/// Absolute, on the same clock as `Lease::ExpiresAt`. `None` is a run with
	/// no time bound, which is a decision the caller makes explicitly
	/// rather than a default the runtime invents.
	pub Deadline:Option<Instant>,
	/// How many effects the run may still perform. `0` means none: the run was
	/// admitted to hold the resource and do nothing with it, which is a
	/// legitimate probe and not an error.
	pub Operations:u64,
}

/// What the composition root registers: a capability's name and its retention.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Registration {
	/// The name `Admit` will be asked about.
	pub Name:String,
	/// The strongest state class the capability retains between runs.
	pub State:StateClass,
}

/// What the runtime reports about itself: the binding it enforces.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Binding {
	/// The generation this runtime was bound to at construction.
	pub Generation:u64,
	/// The capabilities it will admit, sorted by name.
	pub Capabilities:Vec<Registration>,
}

/// One run asking to be admitted.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Request {
	/// The capability asking.
	pub Capability:String,
	/// The state class it claims to need.
	pub State:StateClass,
	/// The generation it believes it is running under.
	pub Generation:u64,
}

/// The answer `Admit` gives. A refusal is a value, not an error: admission
/// answers a question, while a claim or a write either happened or did not.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Admission {
	/// The run may proceed.
	Admitted,
	/// It may not, for this reason. Prefixed with the code, so the string is
	/// already in the vocabulary the manifest promises.
	Refused(String),
}

impl Admission {
	/// Whether the run was admitted.
	pub fn Ok(&self) -> bool { matches!(self, Admission::Admitted) }

	/// The refusal's reason, or an empty string when it was admitted.
	pub fn Reason(&self) -> &str {
		match self {
			Admission::Admitted => "",
			Admission::Refused(Reason) => Reason,
		}
	}
}

/// The port this crate offers.
///
/// `WithLease` is generic over the write's return type, which is why this trait
/// is not object-safe: a boxed `dyn Runtime` would have to erase `T`, and
/// erasing it is what a caller writing `withLease::<Row>` is asking not to
/// happen. The composition root holds the concrete engine instead.
pub trait Runtime: Send + Sync {
	/// Admit a run, or refuse it with the reason.
	fn Admit<'a>(&'a self, Request:Request) -> FutureBox<'a, Admission>;
	/// Claim a resource for one run, with that run's budget, or fail because
	/// one is held. Omitting the budget means the run is unbounded.
	fn Claim<'a>(
		&'a self,
		Resource:&'a str,
		Ttl:Duration,
		Budget:Option<Budget>,
	) -> FutureBox<'a, Result<Lease, Error>>;
	/// Extend a lease, only for the holder of the current fence, and only
	/// forwards.
	fn Renew<'a>(&'a self, Lease:Lease, Ttl:Duration) -> FutureBox<'a, Result<Lease, Error>>;
	/// Write under a lease; refuse when the fence is stale, released or
	/// expired.
	fn WithLease<'a, T, Write>(&'a self, Lease:Lease, Write:Write) -> FutureBox<'a, Result<T, Error>>
	where
		T: Send + 'a,
		Write: FnOnce() -> FutureBox<'a, T> + Send + 'a;
	/// Release, always. Never fails.
	fn Release<'a>(&'a self, Lease:Lease) -> FutureBox<'a, ()>;
	/// The runtime's clock, so a deadline can be expressed on it.
	fn Now(&self) -> Instant;
	/// The manifest this runtime enforces, and the generation it is bound to.
	fn Reflect(&self) -> Binding;
	/// The clock itself, for a caller that must express a deadline on it.
	fn Clock(&self) -> &dyn Clock;
}

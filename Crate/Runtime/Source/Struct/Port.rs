//! The port `agentic.runtime` promises, written the way a consumer asks for it.
//!
//! The port states more than data shapes, because two runtimes that agreed on
//! the shapes alone would still disagree about when authority ends:
//!
//! - **An effect carries its authority.** `WithLease` hands the work a
//!   [`Ticket`]: the lease, the fence, the generation and the seal it was
//!   admitted under. An effect is not a bare closure, because a closure has
//!   nothing to present when it arrives late.
//! - **The destination is the guarantee; the admission is the optimisation.**
//!   `WithLease` checks the lease so doomed work does not start, but that check
//!   ran before the effect did any work and cannot say anything about the
//!   instant it writes. The effect presents its ticket to the place that
//!   records it - the reducer, or the local stand-in for one - and that place
//!   refuses a stale fence, a stale generation, a released lease or an expired
//!   record. A design where only the caller checks is a design where an effect
//!   admitted while a lease was live can still land after the lease ended.
//! - **The numbers are not the identity.** A ticket a caller can reconstruct
//!   from what it observed is a ticket any caller can present. The reducer's
//!   identity is `ctx.sender`, which its transport authenticates; this port's
//!   equivalent is a [`Seal`], which the authority records and a caller cannot
//!   derive. Matching generation, resource, lease and fence authorise nothing
//!   on their own.
//! - **A refusal is not an undo.** Rejecting an effect's record does not
//!   retract its external action. A write that landed in a page is still in the
//!   page when the record of it is refused, so the destination records it as
//!   [`Ambiguous`] and quarantines the resource instead of pretending nothing
//!   happened.
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
//! Two shapes can carry these rules, and they are not interchangeable. A
//! **local stand-in** keeps the lease table in this process and decides here;
//! it is a test double, and in production it is a second authority however
//! correct its fencing. A **thin effect runner** keeps no lease state at all:
//! it takes a ticket the authority issued, runs the work, and presents the
//! ticket back. This crate is the runner. [`crate::Struct::StandIn`] is the
//! double, it is named as one, and [`crate::Struct::Authority::Authority`] is
//! the seam a reducer adapter replaces it through.
//!
//! One port shape changed in the port: `claim(resource, ttl, budget?)` and
//! `now()`/`reflect()` are on this one trait instead of a separate extension
//! trait, because Rust has no method overloading and an extension trait that
//! repeats `claim` with a different arity cannot exist.

use std::{
	fmt,
	hash::{BuildHasher, Hasher},
	sync::atomic::{AtomicU64, Ordering},
	time::{Duration, Instant},
};

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

/// The rendering of one admitted caller's identity.
///
/// The reducer's identity is `ctx.sender`: its transport authenticates the
/// caller, so a rule compares that value and never accepts it as an argument. A
/// single process has no transport, so the rendering here is 128 bits drawn
/// from a key the operating system seeds per process. A caller can observe
/// every number a ticket carries - generation, resource, lease id, fence,
/// expiry - and still not present a seal, because those numbers are not the
/// identity. That is the whole answer to "a caller that guesses the right
/// numbers is not the caller that was admitted".
///
/// It is not a cryptographic commitment and this doc does not claim one. It is
/// unguessable, which is the property the check needs, and it is redacted from
/// `Debug` because a ticket is a bearer token and a logged ticket is a leaked
/// one.
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct Seal([u8; 16]);

impl Seal {
	/// The seal a test presents when it has the numbers and no identity.
	pub const Empty:Seal = Seal([0; 16]);

	/// Mint a seal no other claim, in this process or after a restart, will
	/// produce.
	///
	/// Two halves from a hasher the standard library keys from the operating
	/// system's entropy, mixed with a per-process counter so two seals minted
	/// in the same nanosecond still differ. A restart produces new keys, so
	/// a seal from before a restart is not accepted by the authority after
	/// it - which is the honest answer to "can an identifier be reused
	/// across a restart": it cannot, because the authority that would
	/// recognise it did not survive.
	pub(crate) fn Mint() -> Seal {
		use std::collections::hash_map::RandomState;
		static Counter:AtomicU64 = AtomicU64::new(0);
		let State = RandomState::new();
		let Draw = Counter.fetch_add(1, Ordering::Relaxed);
		let mut Bytes = [0u8; 16];
		for (Index, Half) in Bytes.chunks_mut(8).enumerate() {
			let mut Hasher = State.build_hasher();
			Hasher.write_u64(Draw);
			Hasher.write_u64(Index as u64);
			Half.copy_from_slice(&Hasher.finish().to_le_bytes());
		}
		Seal(Bytes)
	}

	/// Read a seal back from the text an adapter stored.
	///
	/// This is the adapter's door, not the caller's: it rebuilds a seal that
	/// was issued, and confers nothing, because the authority accepts only
	/// the seals it recorded.
	pub fn Parse(Text:&str) -> Option<Seal> {
		if Text.len() != 32 {
			return None;
		}
		let mut Bytes = [0u8; 16];
		for (Index, Byte) in Bytes.iter_mut().enumerate() {
			*Byte = u8::from_str_radix(Text.get(Index * 2..Index * 2 + 2)?, 16).ok()?;
		}
		Some(Seal(Bytes))
	}

	/// The text an adapter stores. Never a log line.
	pub fn Hex(&self) -> String {
		use std::fmt::Write;
		let mut Text = String::with_capacity(32);
		for Byte in self.0 {
			let _ = write!(Text, "{Byte:02x}");
		}
		Text
	}
}

impl fmt::Debug for Seal {
	fn fmt(&self, Formatter:&mut fmt::Formatter<'_>) -> fmt::Result { Formatter.write_str("Seal(<redacted>)") }
}

/// One authority over one resource, granted for a bounded time.
///
/// The seal rides on the grant because the grant is what the holder presents
/// when it asks for an effect, and the holder is the only caller that has it.
#[derive(Clone, PartialEq, Eq)]
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
	/// The holder's identity, as the authority recorded it.
	pub Seal:Seal,
}

impl fmt::Debug for Lease {
	fn fmt(&self, Formatter:&mut fmt::Formatter<'_>) -> fmt::Result {
		Formatter
			.debug_struct("Lease")
			.field("Id", &self.Id)
			.field("Resource", &self.Resource)
			.field("Fence", &self.Fence)
			.field("ExpiresAt", &self.ExpiresAt)
			.field("Seal", &self.Seal)
			.finish()
	}
}

/// One effect's authority to be recorded.
///
/// Issued by the authority for exactly one admitted effect, and handed to the
/// work it admits. The work presents it to the destination, which re-checks it
/// there and refuses it when it is stale. That is the whole reason the type
/// exists: a closure admitted under a lease can outlive the lease, and the
/// numbers that admitted it are the only thing that can still be compared when
/// it arrives.
///
/// The fields are private on purpose. A caller can clone a ticket it was given
/// (that is what holding authority is) but it cannot build one out of numbers
/// it observed, because the numbers are not what the destination compares. Only
/// [`Ticket::Restore`] constructs one from outside, and that is the adapter's
/// door for a ticket read back out of the database, not a caller's.
#[derive(Clone, PartialEq, Eq)]
pub struct Ticket {
	Generation:u64,
	Resource:String,
	LeaseId:String,
	Fence:u64,
	ExpiresAt:Instant,
	Seal:Seal,
}

impl Ticket {
	/// Mint the ticket for one admitted effect, from the record rather than
	/// from the caller's copy of the lease.
	pub(crate) fn Mint(Generation:u64, Lease:&Lease, ExpiresAt:Instant) -> Ticket {
		Ticket {
			Generation,
			Resource:Lease.Resource.clone(),
			LeaseId:Lease.Id.clone(),
			Fence:Lease.Fence,
			ExpiresAt,
			Seal:Lease.Seal,
		}
	}

	/// Rebuild a ticket an authority issued, from the row it issued.
	///
	/// The adapter's constructor. It confers nothing: the destination accepts
	/// only the seals it recorded for the current epoch, so a restored ticket
	/// with the right numbers and the wrong seal is refused exactly as a
	/// guessed one is.
	pub fn Restore(
		Generation:u64,
		Resource:impl Into<String>,
		LeaseId:impl Into<String>,
		Fence:u64,
		ExpiresAt:Instant,
		Holder:Seal,
	) -> Ticket {
		Ticket {
			Generation,
			Resource:Resource.into(),
			LeaseId:LeaseId.into(),
			Fence,
			ExpiresAt,
			Seal:Holder,
		}
	}

	/// The generation the effect was admitted under.
	pub const fn Generation(&self) -> u64 { self.Generation }

	/// The resource the lease is over.
	pub fn Resource(&self) -> &str { &self.Resource }

	/// The id of the lease that admitted it.
	pub fn LeaseId(&self) -> &str { &self.LeaseId }

	/// The fence the resource was at when it was admitted.
	pub const fn Fence(&self) -> u64 { self.Fence }

	/// The expiry the record carried at admission. Diagnostic only: the
	/// destination compares *its own* record's expiry against *its own* clock,
	/// because the copy a caller presents is exactly the thing a lease that
	/// outlived its authority still has.
	pub const fn ExpiresAt(&self) -> Instant { self.ExpiresAt }

	/// The identity the effect was admitted as.
	pub const fn Seal(&self) -> Seal { self.Seal }
}

impl fmt::Debug for Ticket {
	fn fmt(&self, Formatter:&mut fmt::Formatter<'_>) -> fmt::Result {
		Formatter
			.debug_struct("Ticket")
			.field("Generation", &self.Generation)
			.field("Resource", &self.Resource)
			.field("LeaseId", &self.LeaseId)
			.field("Fence", &self.Fence)
			.field("ExpiresAt", &self.ExpiresAt)
			.field("Seal", &self.Seal)
			.finish()
	}
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

/// What the destination kept after accepting an effect.
///
/// The receipt is the effect's existence in the state: the fence and generation
/// it names are the authority it was recorded under, read from the table at the
/// instant it landed rather than copied from the ticket.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Recorded {
	/// The effect's own name for what it did.
	pub Name:String,
	/// The resource it happened under.
	pub Resource:String,
	/// The fence it was recorded at.
	pub Fence:u64,
	/// The generation it was recorded under.
	pub Generation:u64,
	/// The instant the destination accepted it, on the runtime's clock.
	pub At:Instant,
}

/// What a resolver says happened to an effect that was refused when it landed.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Resolution {
	/// The external action did happen. The effect is accounted for, not
	/// retracted: the state records that a write of unknown shape landed under
	/// an authority that had ended.
	Landed,
	/// The external action did not happen, or its absence was verified
	/// afterwards.
	Absent,
}

/// An effect that was admitted, was refused when it landed, and may therefore
/// have changed the world anyway.
///
/// This is the handover rule's record. Rejecting an effect's eventual result
/// does not undo its external effect, so the refusal is not allowed to be the
/// only trace of it: the effect is named, the epoch it belonged to is named,
/// and the resource stays quarantined until a resolver states which of the two
/// outcomes it was.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Ambiguous {
	/// The effect's own name for what it tried to do.
	pub Name:String,
	/// The resource it happened - or may have happened - under.
	pub Resource:String,
	/// The generation it was admitted under.
	pub Generation:u64,
	/// The fence it was admitted under.
	pub Fence:u64,
	/// The lease that admitted it.
	pub LeaseId:String,
	/// When the refusal was recorded.
	pub At:Instant,
	/// Why it was refused. One sentence, already in the vocabulary of the
	/// failure the caller saw.
	pub Reason:String,
	/// What a resolver said, or `None` while the outcome is unknown.
	pub Resolution:Option<Resolution>,
}

/// Where an effect is recorded: the one place that decides whether it may be.
///
/// One verb, because a destination that offered a second way to write would be
/// a destination with a second rule. It is a future rather than a call because
/// reaching the authority is not free: the production implementation is a
/// reducer call over a connection, and a trait that forced it to block would
/// have decided which transport the database gets.
///
/// The production implementation calls the reducer. `StandIn` implements it
/// over a local table so the port can be exercised without a database, and it
/// is a test double rather than the authority: a build with no database has no
/// decision to make and says so by having no implementation wired. The reads a
/// stand-in needs - which effects landed, which are ambiguous, which resources
/// are quarantined - are that stand-in's own surface, because a database
/// exposes them as rows and a subscription, not as a method on the port.
pub trait Destination: Send + Sync {
	/// Record one effect at the place it lands, or refuse it.
	///
	/// A refusal is a value. A panic would turn a lost lease into a lost
	/// process, and a lost lease is the expected outcome of a slow worker.
	///
	/// A refusal is also not an undo: an effect that ran before it presented
	/// may have changed the world, so the destination records it as
	/// [`Ambiguous`] and quarantines the resource rather than reporting
	/// nothing happened.
	fn Present<'a>(&'a self, Ticket:Ticket, Name:&'a str) -> FutureBox<'a, Result<Recorded, Error>>;
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
	/// Admit one effect under a lease, and hand it the authority it was
	/// admitted under.
	///
	/// The check here is an optimisation: work whose authority has already
	/// ended is refused before it starts, and the run is charged for the
	/// attempt. It is not the guarantee. A lease can expire while the work is
	/// running, and this call cannot revoke the effect at that point because
	/// this call is not where the effect lands. The work receives a [`Ticket`]
	/// and must present it to the [`Destination`] - the reducer, or the local
	/// stand-in for one - which refuses a stale fence, a stale generation, a
	/// released lease or an expired record.
	///
	/// An effect whose record is refused after it ran is not retracted: the
	/// destination records it as [`Ambiguous`] and quarantines the resource.
	fn WithLease<'a, T, Write>(&'a self, Lease:Lease, Write:Write) -> FutureBox<'a, Result<T, Error>>
	where
		T: Send + 'a,
		Write: FnOnce(Ticket) -> FutureBox<'a, T> + Send + 'a;
	/// Release, always. Never fails.
	fn Release<'a>(&'a self, Lease:Lease) -> FutureBox<'a, ()>;
	/// The runtime's clock, so a deadline can be expressed on it.
	fn Now(&self) -> Instant;
	/// The manifest this runtime enforces, and the generation it is bound to.
	fn Reflect(&self) -> Binding;
	/// The clock itself, for a caller that must express a deadline on it.
	fn Clock(&self) -> &dyn Clock;
}

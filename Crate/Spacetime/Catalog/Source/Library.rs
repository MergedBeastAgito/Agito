//! `agito.catalog.rules`: the database model as data and pure functions.
//!
//! A SpacetimeDB module compiles its tables into the published module, so a row
//! in a table-definition table cannot create one and no reducer can execute
//! DDL. What travels instead is the description - name, columns, types, keys
//! and whether a table is append-only - and the rules that read it. A module
//! renders its `table(...)` declarations from [`Tables`], satisfies
//! [`Context`], and registers [`Bind`], [`Claim`], [`Renew`], [`Record`],
//! [`Resolve`], [`Probe`] and [`RecordTrace`] as reducers; nothing here names
//! the server, so every rule is checkable before a module exists.
//!
//! # Why this is a capability and not a client
//!
//! The crate has **no dependencies at all**, and that is the property worth
//! keeping rather than an accident. A rule that imported `spacetimedb` could
//! not be tested without a published database, and would pin the rules to one
//! server version. The seam is [`Context`]: five members - who is calling, what
//! time it is, and one row read, inserted, updated or deleted by key - and a
//! module's adapter is the only code that has to be written for it.
//!
//! # What a rule answers
//!
//! Nothing here throws on a bad argument: a rule answers [`Answer`], and a
//! refusal is a value a caller handles and a test asserts. The three reasons
//! are [`Code`], which is the vocabulary `provides.errors` in `module.json`
//! declares, so a manifest cannot drift from the crate without the workspace's
//! readiness check failing.
//!
//! The five invariants, and where each is enforced:
//!
//! 1. **Coherence.** [`Bind`] appends a generation and refuses a manifest
//!    naming a capability the catalog does not declare. Every other rule
//!    refuses to write under a generation that is not the latest bound one
//!    ([`Latest`]), and a lease never adopts a later generation than the one it
//!    was granted under.
//! 2. **Fence.** [`Claim`] increments the fence; [`Renew`], [`Record`] and
//!    [`Probe`] refuse a fence that is not the lease's current one, so a worker
//!    whose lease expired cannot write as though it still held it. [`Record`]
//!    is where an effect is checked, and it checks the caller too:
//!    `lease.holder` is [`Context::Sender`], so a caller that reproduces every
//!    number on the row is still not the holder.
//! 3. **Disclosure.** [`Probe`] writes exactly one audience-scoped row and
//!    returns only that row. The private value is never an argument, so it is
//!    never a column: a reader cannot select what the database was never told.
//! 4. **Time.** Every timestamp a row carries is [`Context::Now`]. No function
//!    reads a wall clock, and the lags between times are differences the caller
//!    computes.
//! 5. **Handover.** An effect that was admitted under an epoch and refused when
//!    it lands is not retracted, because rejecting a result does not undo an
//!    effect. [`Record`] writes the refusal as an `effect-ambiguous` trace row
//!    and moves the lease to `quarantined`; [`Claim`] refuses a quarantined
//!    resource; [`Resolve`] states whether the effect landed or was absent and
//!    only then releases the resource for reassignment.

#![allow(non_snake_case)]
#![allow(non_upper_case_globals)]
#![allow(clippy::upper_case_acronyms)]

pub mod Fn;
pub mod Struct;

pub use crate::{
	Fn::{
		Bind::Bind,
		Claim::Claim,
		Latest::Latest,
		Probe::Probe,
		Record::Record,
		Renew::Renew,
		Resolve::Resolve,
		Trace::RecordTrace,
	},
	Struct::{
		Answer::{Answer, ProbeAnswer},
		Bind::{Bind as BindArgs, Manifest},
		Catalog::{ColumnTypes, PrimaryKey, TableNames, Tables},
		Claim::Claim as ClaimArgs,
		Code::Code,
		Column::{Column, ColumnType, Key},
		Context::Context,
		Effect::Effect,
		Probe::Probe as ProbeArgs,
		Renew::Renew as RenewArgs,
		Resolve::{Outcome, Resolve as ResolveArgs},
		Row::{Row, Value},
		Table::{Table, TableName},
		Trace::Trace,
	},
};

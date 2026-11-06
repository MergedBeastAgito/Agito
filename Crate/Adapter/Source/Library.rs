//! `agito.adapter`: the shape a third-party substrate implements.
//!
//! Agito ships capabilities and one concrete substrate. A third party with a
//! different substrate - another game, another protocol, another device - needs
//! one thing this workspace could not previously give it: a port it can
//! implement that a composition root can hold without naming the crate that
//! implements it. That is [`Struct::Port::Adapter`]. Until this crate existed a
//! third-party driver could only copy `agito-browser-cdp`'s plumbing and be
//! consumed by path, which is what the NFSU2 driver did.
//!
//! # Why a trait and not a registry
//!
//! A registry is a global, and a global makes load order observable: two
//! adapters claiming one port are then resolved by whoever registered first,
//! and the answer to "which one is running" depends on a side effect nobody
//! wrote down. A trait is a value the composition root already knows how to
//! pass. It constructs the adapter once, holds it as `Arc<dyn Adapter>`, and
//! hands that on - the shape `agito-browser-cdp`'s `Browse` already has, and
//! the reason both return a boxed future rather than declaring `async fn`.
//!
//! # What this crate does not decide
//!
//! An adapter's failures are its own. [`Struct::Error::Error`] carries the code
//! the adapter's own `module.json` declares, and this crate enumerates none of
//! them: a code names a substrate failure, and this crate has no substrate. That
//! is why this module's manifest lists no errors, and why the manifest of an
//! adapter - not this crate - is where its vocabulary is checked.
//!
//! # What it also does not do
//!
//! There is no `Adapter` implementation here. An implementation would be a
//! second substrate, and the workspace already has one; the crate ships the
//! shape, one function that makes the shape checkable, and a stand-in for a
//! consumer's own tests.

#![allow(non_snake_case)]
#![allow(non_upper_case_globals)]
#![allow(clippy::upper_case_acronyms)]

pub mod Fn;
pub mod Struct;

pub use crate::{
	Fn::Serves::Serves,
	Struct::{Error::Error, Port::Adapter, StandIn::StandIn},
};

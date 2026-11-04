//! The composition root: the only place that names concrete adapters.
//!
//! Everything below this crate is a port that somebody else implements. This
//! crate is where a browser is chosen, where a pace is chosen, and where the
//! two bindings that translate a consumer's port onto the substrate are
//! installed. It is also where the eight manifests live, embedded at compile
//! time, so the binary can report what the workspace ships without reading a
//! file.
//!
//! The one thing it must not do is leak an adapter downwards: no crate below
//! this one depends on another's concrete type, and the only crate that names
//! `chromiumoxide` is `agentic-browser-cdp`.

#![allow(non_snake_case)]
#![allow(non_upper_case_globals)]
#![allow(clippy::upper_case_acronyms)]

pub mod Fn;
pub mod Struct;

pub use crate::{
	Fn::{
		Composition::{Browser, Chat, Design},
		Run::{Drive, Run},
		VerifyModule::{Declared, Readiness, VerifyModule},
	},
	Struct::{
		Binary::Command::{Command, Verb},
		Manifest::{All, Find, Manifest},
	},
};

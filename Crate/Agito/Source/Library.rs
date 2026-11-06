//! The composition root: the only place that names concrete adapters.
//!
//! Everything below this crate is a port that somebody else implements. This
//! crate is where a browser is chosen, where a pace is chosen, and where the
//! two bindings that translate a consumer's port onto the substrate are
//! installed.
//!
//! It is also the executable, and the manifest table it prints is re-exported
//! from the facade rather than restated here: the answer to "what does this
//! workspace ship" is stated once. The only rows this binary adds to that answer
//! are the ones it read from an adapter root - a third-party module cannot be
//! compiled in, which is what `kind: "adapter"` means, so it is discovered at run
//! time and reported with the directory it was found in.
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
		Discover::Discover,
		Note::{Note, ThreadOf},
		Run::{Drive, Run},
		VerifyModule::{Declared, Readiness, VerifyModule},
	},
	Struct::{
		Binary::{
			Command::{Command, Verb},
			Shape::Shape,
		},
		External::External,
		Kind::{Kinds, Known, Names},
		Manifest::{All, Find, Manifest},
		Note::Given,
	},
};

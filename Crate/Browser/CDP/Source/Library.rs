//! `agentic.browse`: the concrete browser substrate.
//!
//! Attach and launch are separate operations, not a boolean hidden in a
//! constructor, because they differ in who owns the process: a session that
//! launched a browser may close it, and a session that attached to one may not.
//! [`Struct::Port::Ownership`] records which happened, and disposal reads it.
//!
//! Every classification rule lives in [`Fn::Classify`], in one place:
//!
//! - silence is a timeout. Nothing but the deadline can end a page that never
//!   answers, so a hang is reported as `timeout` and never as a dead transport;
//! - a closed transport is `browser-unavailable`, whatever error arrived with
//!   it, because the socket is gone rather than the page being slow;
//! - a live transport that could not be used is `navigation-failed`.
//!
//! Cancellation belongs to the caller: every operation races the caller's
//! signal and its own deadline through [`AgenticRuntime::Bound`], and both lose
//! to `timeout` rather than to a bare error, so a consumer can distinguish "I
//! gave up" from "the browser broke".
//!
//! `chromiumoxide` is behind [`Struct::Chrome`] and nowhere else. Nothing in
//! the session, the port or the transports mentions it: a different substrate
//! would replace one file.

#![allow(non_snake_case)]
#![allow(non_upper_case_globals)]
#![allow(clippy::upper_case_acronyms)]

pub mod Fn;
pub mod Struct;

pub use crate::{
	Fn::Classify::Classify,
	Struct::{
		Cancellation::Stop,
		Chrome::{Chrome, ChromeOptions},
		Error::{Code, Error},
		Pace::Tokio,
		Port::{Browse, OpenOptions, Ownership, Target},
		Session::{Create, Options, Report, Session},
		Transport::{Launcher, PageWant, Transport},
	},
};

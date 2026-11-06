//! Adapter-independent compositions, and the one place that knows both sides of
//! a port.
//!
//! This crate exists to hold the thing neither a capability nor an adapter may
//! hold: the translation between a consumer's own trait and the browser
//! substrate. `chat-driver` declares `Page` and must not know a browser;
//! `browser-cdp` exports `Browse` and must not know a chat product.
//! [`Struct::Binding`] implements the first in terms of the second, and that is
//! what a composition layer is for.
//!
//! The translation is deliberately small and deliberately not shared. A
//! "universal browser request" type that both capabilities could use would
//! quietly become the ports crate the whole design exists to avoid, so each
//! binding converts on its own and keeps its own conventions.

#![allow(non_snake_case)]
#![allow(non_upper_case_globals)]
#![allow(clippy::upper_case_acronyms)]

pub mod Fn;
pub mod Struct;

pub use crate::{
	Fn::{
		Measure::MeasurementExpression,
		Skill::Page::Verify::{Outcome, Verify},
	},
	Struct::Binding::{Measure::Measure, Page::Page},
};

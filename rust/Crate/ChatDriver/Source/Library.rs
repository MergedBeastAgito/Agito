//! `agentic.chat.ask`: ask a chat product one question, and return the answer
//! once the page has stopped changing it.
//!
//! The capability declares the port it needs. [`Struct::Port::Page`] is this
//! crate's own trait - not `agentic-browser-cdp`'s `Browse`, and deliberately
//! not a copy of it: the same three verbs, because a page is opened, asked and
//! released, and it stops there. No ownership, no attach, no transport, no
//! launcher. The composition root adapts a browser to this, and that is where
//! "a browser this session launched" and "a tab somebody else owns" is settled.
//!
//! The dependency list is the enforcement: no browser crate and no executor.
//! Time arrives as a [`AgenticRuntime::Call`] and a [`AgenticRuntime::Pace`],
//! so a page that hangs is bounded by the thing that supplied the deadline
//! rather than by a timer this crate reached for.

#![allow(non_snake_case)]
#![allow(non_upper_case_globals)]
#![allow(clippy::upper_case_acronyms)]

pub mod Fn;
pub mod Struct;

pub use crate::{
	Fn::{
		Expressions,
		Expressions::{
			PressMode,
			PressSubmit,
			ReadAnswers,
			ReadCollapse,
			ReadMode,
			ReadModel,
			ReadSources,
			ReadTrace,
			TypeQuestion,
		},
	},
	Struct::{
		Driver::{Create, Driver, Options},
		Error::{Code, Error},
		Port::{Answer, Chat, Control, Failure, Mode, Observation, Page, Question, Source, Step, Target},
		Selectors::{DefaultSelectors, Selectors},
		Settlement::{Confirmations, Settlement, StableReads},
	},
};

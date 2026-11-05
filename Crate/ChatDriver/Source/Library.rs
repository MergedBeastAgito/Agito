//! `agentic.chat.ask`: ask a chat product one question, and return the answer
//! once the page has stopped changing it.
//!
//! `agentic.chat.note`: turn a capture of that answer into the document a
//! reader opens. It is declared as a second verb beside `agentic.chat.ask`
//! rather than as a fifth error on it, because it needs nothing from a page -
//! no transport, no deadline, no failure - so a caller that only wants the
//! document does not have to hold a browser to get one. The formatting used to
//! live in a script beside this repository, which made the shape of a note a
//! fact about that script; it is [`crate::Struct::Note::Note`] and
//! [`crate::Fn::Note::Render`] now.
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
		Note::{Build, Render, Strip},
	},
	Struct::{
		Driver::{Create, Driver, Options},
		Error::{Code, Error},
		Format::Format,
		Note::{DefaultChrome, Fact, Meta, Note, Vocabulary},
		Port::{Answer, Chat, Control, Failure, Mode, Observation, Page, Question, Source, Step, Target},
		Selectors::{DefaultSelectors, Selectors},
		Settlement::{Confirmations, Settlement, StableReads},
	},
};

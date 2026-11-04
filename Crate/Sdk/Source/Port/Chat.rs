//! `agentic.chat.ask`, from `agentic-chat-driver`: one question, one send, and
//! the answer read once the page has stopped changing it.
//!
//! [`Page`] is this crate's own trait - opened, asked, released, and it stops
//! there - and it is deliberately not `agentic.browse`'s [`crate::Browse`]. A
//! chat page and a measured page are asked different things, and collapsing the
//! two into one universal request type is how a ports crate grows back.
//!
//! [`Driver`] is the composition: [`Create`] binds it to a [`Page`], a
//! [`Selectors`] and an [`Options`]. [`ReadAnswers`], [`ReadTrace`],
//! [`ReadSources`], [`ReadModel`], [`ReadMode`], [`ReadCollapse`],
//! [`TypeQuestion`] and [`PressSubmit`] are the expressions it sends, exposed
//! because a consumer measuring its own page wants the same ones.
//!
//! What is not here: the binding from a browser to [`Page`]. That is a
//! composition, it lives in the composition root, and a facade that shipped it
//! would be shipping a composition - the caller adapts its own session, which is
//! four methods and no ambiguity about who owns the tab.

pub use AgenticChatDriver::{
	Answer,
	Chat,
	Code,
	Confirmations,
	Control,
	Create,
	DefaultSelectors,
	Driver,
	Error,
	Failure,
	Format,
	Mode,
	Observation,
	Options,
	Page,
	PressMode,
	PressSubmit,
	Question,
	ReadAnswers,
	ReadCollapse,
	ReadMode,
	ReadModel,
	ReadSources,
	ReadTrace,
	Selectors,
	Settlement,
	Source,
	StableReads,
	Step,
	Target,
	TypeQuestion,
};

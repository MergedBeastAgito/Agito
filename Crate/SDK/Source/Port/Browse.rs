//! `agentic.browse`, from `agentic-browser-cdp`: attach or launch, evaluate,
//! dispose.
//!
//! [`Session`] is the port; [`Create`] binds it to a launcher and a pace, and
//! [`Chrome`] is the launcher the workspace ships. [`Tokio`] is the pace that
//! launcher requires, because it spawns the handler that keeps the page
//! answering - so a consumer must already be inside a tokio runtime before it
//! opens anything.
//!
//! What is not here: `Launcher`, `Transport` and `PageWant`. They are the seam
//! between a session and a mechanism, and a consumer that named one would be
//! writing an adapter rather than using one. `Classify` and `Stop` are the
//! crate's own machinery for the same reason: one decides which of the three
//! codes a failure carries, the other is how a transport stops listening, and
//! neither is a decision a caller makes.

pub use AgenticBrowserCDP::{
	Browse,
	Chrome,
	ChromeOptions,
	Code,
	Create,
	Error,
	OpenOptions,
	Options,
	Ownership,
	Report,
	Session,
	Target,
	Tokio,
};

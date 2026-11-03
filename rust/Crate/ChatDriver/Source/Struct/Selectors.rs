//! Where a chat product keeps the things this driver has to read.
//!
//! A driver written against one product's markup is not reusable, and a driver
//! written against nothing cannot apply its rules at all: the mode and the
//! model are preconditions because there is a toggle and a label to read, and
//! an answer is `truncated` because there is a control that hides the rest of
//! it. So the shape is fixed here and the strings are configuration.
//!
//! A selector may be a CSS selector list - `"textarea, [contenteditable]"` -
//! which is how one product is covered by one entry without the driver knowing
//! which of the two a given page used.

/// Where one chat product keeps the things this driver reads.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Selectors {
	/// The composer's mode toggles. Each carries its mode as its label and
	/// `aria-pressed="true"` on the one that is active, so the mode is the
	/// label of the single pressed toggle, and unknown when no toggle is
	/// pressed or several are.
	pub Mode:String,
	/// The element naming the model the composer will send to.
	pub Model:String,
	/// Where the question is typed: a `textarea`, an `input` or a
	/// contenteditable.
	pub Composer:String,
	/// The control that sends the question.
	pub Submit:String,
	/// Every answer in the thread, in document order; the last one is the
	/// newest.
	pub Answer:String,
	/// The control that collapses an answer, searched for *inside* the answer
	/// container. Not across the page: an earlier answer's control is not this
	/// answer's.
	pub Collapse:String,
	/// One step of the answer's trace, searched for *inside* the newest answer.
	pub Trace:String,
	/// One cited source: an anchor whose `href` is the citation, searched for
	/// *inside* the newest answer, for the same reason as `Trace`.
	pub Sources:String,
}

/// A chat product whose composer footer names its mode and its model.
pub fn DefaultSelectors() -> Selectors {
	Selectors {
		Mode:"[data-command-menu-control]".to_string(),
		Model:"[data-testid='model-selector'], [aria-label='Model']".to_string(),
		Composer:"textarea, [contenteditable='true']".to_string(),
		Submit:"[data-testid='submit-button'], button[aria-label='Submit']".to_string(),
		Answer:"[data-testid='answer'], [role='article']".to_string(),
		Collapse:"button".to_string(),
		Trace:"[data-testid='trace-step'], [role='listitem']".to_string(),
		Sources:"a[href^='http']".to_string(),
	}
}

impl Default for Selectors {
	fn default() -> Self { DefaultSelectors() }
}

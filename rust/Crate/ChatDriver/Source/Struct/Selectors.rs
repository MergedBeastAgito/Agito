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
	/// The control that means the page is still producing the answer - a stop
	/// or a cancel control.
	///
	/// Searched for across the whole page, not inside the answer, because a
	/// product keeps it in the composer: it is a fact about the turn, not about
	/// the text the turn has produced so far.
	pub Busy:String,
	/// The answer's own turn container, when the product has one.
	///
	/// Searched for as an ancestor of the answer, so a product that wraps a turn
	/// (body, steps, duration, source chips) can be asked what the *turn* says
	/// rather than what its body says.
	pub Turn:String,
	/// What the turn's text reads once the turn is over, as a pattern.
	///
	/// This is the positive half of completion: `Busy` says the page is still
	/// working, and this says the page has finished. They are not each other's
	/// negation - a cancelled turn is neither.
	///
	/// Empty by default, and deliberately so. A header that reads like a
	/// completion marker is often a duration label that is present while the
	/// answer streams, and a pattern that matches it makes every turn look
	/// finished from its first token - the exact false settle the check exists
	/// to prevent. Enable it only after watching a live turn and confirming the
	/// text appears at the end rather than the start.
	pub Done:String,
}

/// A chat product whose composer footer names its mode and its model.
pub fn DefaultSelectors() -> Selectors {
	Selectors {
		Mode:"[data-command-menu-control] button".to_string(),
		Model:"[class*='composer-row'] button[aria-haspopup='dialog'], [data-appearance='framed'] button[aria-haspopup='dialog']".to_string(),
		Composer:"textarea, [contenteditable='true']".to_string(),
		Submit:"[data-testid='submit-button'], button[aria-label='Submit']".to_string(),
		Answer:".prose[data-renderer='lm'], [data-testid='answer'], [role='article']".to_string(),
		Collapse:"button".to_string(),
		Trace:"[data-testid='trace-step'], [role='listitem']".to_string(),
		Sources:"a[href^='http']".to_string(),
		Busy:"button[data-testid='stop-generating-response-button'], button[aria-label*='Stop' i], button[aria-label*='Cancel' i]".to_string(),
		Turn:String::new(),
		Done:String::new(),
	}
}

/// The defaults, with whichever of these keys the caller stated replaced.
///
/// A merge rather than a whole object, because a caller changing one selector
/// should not have to restate the seven it is happy with, and because a key that
/// is absent must keep the default instead of becoming an empty selector that
/// matches everything.
///
/// The keys are `mode`, `model`, `composer`, `submit`, `answer`, `collapse`,
/// `trace`, `sources`, `busy`, `turn` and `done`.
pub fn Merged(Overrides:&serde_json::Value) -> Selectors {
	let mut Base = DefaultSelectors();
	let Read = |Key:&str, Field:&mut String| {
		if let Some(Text) = Overrides.get(Key).and_then(|Value| Value.as_str()) {
			*Field = Text.to_string();
		}
	};
	Read("mode", &mut Base.Mode);
	Read("model", &mut Base.Model);
	Read("composer", &mut Base.Composer);
	Read("submit", &mut Base.Submit);
	Read("answer", &mut Base.Answer);
	Read("collapse", &mut Base.Collapse);
	Read("trace", &mut Base.Trace);
	Read("sources", &mut Base.Sources);
	Read("busy", &mut Base.Busy);
	Read("turn", &mut Base.Turn);
	Read("done", &mut Base.Done);
	Base
}

impl Default for Selectors {
	fn default() -> Self { DefaultSelectors() }
}

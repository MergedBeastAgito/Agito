/**
 * Where a chat product keeps the things this driver has to read.
 *
 * A driver written against one product's markup is not reusable, and a driver
 * written against nothing cannot apply its rules at all: the mode and the model
 * are preconditions because there is a toggle and a label to read, and an answer
 * is `truncated` because there is a control that hides the rest of it. So the
 * shape is fixed here and the strings are configuration.
 *
 * The defaults are a starting point for a chat product whose composer footer
 * names its mode and its model - `Search` beside `Computer`, with the active one
 * pressed, a model name next to them, answers appended to one thread, and a
 * `Read more` control inside a long answer. A product that differs passes its own
 * selectors, and the driver's rules do not change: which element is read is
 * configuration, what is done with what came back is not.
 *
 * A selector may be a CSS selector list - `"textarea, [contenteditable]"` - which
 * is how one product is covered by one entry without the driver knowing which of
 * the two a given page used.
 */
export type Selectors = {
	/**
	 * The composer's mode toggles.
	 *
	 * Each carries its mode as its label - `aria-label` first, its text second -
	 * and `aria-pressed="true"` on the one that is active. The mode is therefore
	 * the label of the single pressed toggle, and it is unknown when no toggle is
	 * pressed or several are, which reads as "not search" and is refused.
	 */
	mode: string;
	/** The element naming the model the composer will send to. */
	model: string;
	/** Where the question is typed: a `textarea`, an `input` or a contenteditable. */
	composer: string;
	/** The control that sends the question. */
	submit: string;
	/** Every answer in the thread, in document order; the last one is the newest. */
	answer: string;
	/**
	 * The control that collapses an answer, searched for *inside* the answer
	 * container. Not across the page: an earlier answer's control is not this
	 * answer's, and treating it as one would refuse a complete answer.
	 */
	collapse: string;
	/**
	 * One step of the answer's trace, searched for *inside* the newest answer.
	 * `data-depth` is its nesting. An earlier turn's steps are that turn's, so a
	 * document-wide read would report work this answer never did.
	 */
	trace: string;
	/**
	 * One cited source: an anchor whose `href` is the citation, searched for
	 * *inside* the newest answer, for the same reason as `trace`.
	 */
	sources: string;
};

/** A chat product whose composer footer names its mode and its model. */
export const DefaultSelectors: Selectors = {
	mode: "[data-command-menu-control]",
	model: "[data-testid='model-selector'], [aria-label='Model']",
	composer: "textarea, [contenteditable='true']",
	submit: "[data-testid='submit-button'], button[aria-label='Submit']",
	answer: "[data-testid='answer'], [role='article']",
	collapse: "button",
	trace: "[data-testid='trace-step'], [role='listitem']",
	sources: "a[href^='http']",
};

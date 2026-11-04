/**
 * The expressions the driver sends to a page, and the values it expects back.
 *
 * These are the whole protocol between the capability and the browser. They are
 * deliberately small and literal: each one reads one thing, returns a plain value,
 * and touches nothing but `document`. That is what lets the same driver run
 * against a real tab and against the fake in `Test/`, which evaluates these
 * expressions for real against a page-shaped object rather than matching strings
 * this package happened to send.
 *
 * Two conventions hold across all of them:
 *
 * - a value the page does not have is `null`, never a guess. A missing toggle is
 *   not "search", a missing label is not the model the caller asked for, and a
 *   missing answer is not an empty answer;
 * - the text they return is trimmed, because a page's own whitespace is not part
 *   of what it said, and an answer that differs only in trailing spaces has not
 *   changed.
 */
import type { Selectors } from "./Selectors.ts";

/** What one read of the thread answers. */
export type Answers = {
	/** How many answers the thread holds, so a new one can be told apart. */
	count: number;
	/** The last answer's text, trimmed; empty when there is none. */
	text: string;
};

/** What pressing the send control did. */
export type SendOutcome = "sent" | "missing" | "disabled";

/** A string as a JavaScript literal, so a selector cannot end the expression. */
const literal = (value: string): string => JSON.stringify(value);

/** A label reader, repeated in the expressions that read one. */
const Label =
	'(Element) => (Element.getAttribute("aria-label") || Element.innerText || "").trim().toLowerCase()';

/**
 * The composer's mode: the label of the single pressed toggle.
 *
 * `null` when there is no toggle, when none is pressed, or when several are - a
 * page that cannot say which mode it is in has not said it is in search mode.
 */
export const ReadMode = (selectors: Selectors): string => `(() => {
	const Label = ${Label};
	const Pressed = [...document.querySelectorAll(${literal(selectors.mode)})].filter((Toggle) => Toggle.getAttribute("aria-pressed") === "true");
	return Pressed.length === 1 ? Label(Pressed[0]) : null;
})()`;

/**
 * Press the mode toggle.
 *
 * The toggle that reads `search` is preferred, so a product that renders one
 * button per mode is switched rather than cycled; a single cycling toggle is the
 * fallback. Pressing is not verifying - the caller reads the mode again and
 * decides - so this returns whether anything was there to press.
 */
export const PressMode = (selectors: Selectors): string => `(() => {
	const Label = ${Label};
	const Toggles = [...document.querySelectorAll(${literal(selectors.mode)})];
	const Wanted = Toggles.find((Toggle) => Label(Toggle) === "search");
	const Target = Wanted === undefined ? Toggles[0] : Wanted;
	if (Target === undefined) return false;
	Target.click();
	return true;
})()`;

/** The model label the composer will send to, as the page shows it. */
export const ReadModel = (selectors: Selectors): string => `(() => {
	const Label = document.querySelector(${literal(selectors.model)});
	if (Label === null) return null;
	return (Label.innerText || Label.getAttribute("aria-label") || "").trim();
})()`;

/**
 * Put the question in the composer.
 *
 * A `textarea` or an `input` takes it as a value; anything else is treated as a
 * contenteditable and takes it as text. Both paths dispatch `input`, because a
 * composer that never saw an input event keeps its send control disabled and the
 * question would sit there looking sent.
 */
export const TypeQuestion = (selectors: Selectors, text: string): string => `(() => {
	const Composer = document.querySelector(${literal(selectors.composer)});
	if (Composer === null) return false;
	const Text = ${literal(text)};
	if (Composer.tagName === "TEXTAREA" || Composer.tagName === "INPUT") Composer.value = Text;
	else Composer.innerText = Text;
	Composer.dispatchEvent(new Event("input", { bubbles: true }));
	return true;
})()`;

/**
 * Press the send control, and say what stopped it when it could not be pressed.
 *
 * A disabled control is not clicked: the click would land on nothing, the page
 * would look sent, and the caller would wait out its whole deadline for an answer
 * that was never asked for.
 */
export const PressSubmit = (selectors: Selectors): string => `(() => {
	const Submit = document.querySelector(${literal(selectors.submit)});
	if (Submit === null) return "missing";
	if (Submit.disabled === true) return "disabled";
	Submit.click();
	return "sent";
})()`;

/** How many answers the thread holds, and the text of the newest one. */
export const ReadAnswers = (selectors: Selectors): string => `(() => {
	const Answers = [...document.querySelectorAll(${literal(selectors.answer)})];
	if (Answers.length === 0) return { count: 0, text: "" };
	const Last = Answers[Answers.length - 1];
	return { count: Answers.length, text: (Last.innerText || "").trim() };
})()`;

/**
 * The label of the control that collapses the newest answer, or `null`.
 *
 * Only `Read more` counts. `Read less` is the label of the control that expands
 * an answer again, so an answer already open is complete and its text is what the
 * caller asked for.
 */
export const ReadCollapse = (selectors: Selectors): string => `(() => {
	const Answers = [...document.querySelectorAll(${literal(selectors.answer)})];
	if (Answers.length === 0) return null;
	const Last = Answers[Answers.length - 1];
	for (const Control of [...Last.querySelectorAll(${literal(selectors.collapse)})]) {
		const Label = (Control.getAttribute("aria-label") || Control.innerText || "").trim();
		if (/^read more$/i.test(Label)) return Label;
	}
	return null;
})()`;

/**
 * The work the newest answer reports having done, in the order it did it.
 *
 * Scoped to the newest answer for the same reason the collapse control is: a
 * thread holds every turn's steps, and a document-wide read would hand back the
 * trace of a question the caller did not ask. No answer yet is no steps.
 */
export const ReadTrace = (selectors: Selectors): string => `(() => {
	const Answers = [...document.querySelectorAll(${literal(selectors.answer)})];
	if (Answers.length === 0) return [];
	const Last = Answers[Answers.length - 1];
	return [...Last.querySelectorAll(${literal(selectors.trace)})].map((Step) => ({
		depth: Number(Step.getAttribute("data-depth") || 0),
		label: (Step.innerText || "").trim(),
		detail: (Step.getAttribute("data-detail") || Step.getAttribute("aria-label") || "").trim(),
	}));
})()`;

/**
 * The sources the newest answer cites.
 *
 * Scoped to the newest answer, because the sources of an earlier turn are that
 * turn's: an answer that cites nothing is a different fact from an answer whose
 * thread happens to contain citations elsewhere.
 */
export const ReadSources = (selectors: Selectors): string => `(() => {
	const Answers = [...document.querySelectorAll(${literal(selectors.answer)})];
	if (Answers.length === 0) return [];
	const Last = Answers[Answers.length - 1];
	return [...Last.querySelectorAll(${literal(selectors.sources)})].map((Source) => ({
		url: Source.getAttribute("href") || "",
		label: (Source.innerText || "").trim(),
	}));
})()`;

//! The expressions the driver sends to a page.
//!
//! These are the whole protocol between the capability and the browser. They
//! are deliberately small and literal: each one reads one thing, returns a
//! plain value, and touches nothing but `document`. That is what lets the same
//! driver run against a real tab and against a fake, which answers by
//! recognising the expression it was handed rather than by matching a string
//! this crate happened to send.
//!
//! Two conventions hold across all of them:
//!
//! - a value the page does not have is `null`, never a guess. A missing toggle
//!   is not "search", a missing label is not the model the caller asked for,
//!   and a missing answer is not an empty answer;
//! - the text they return is trimmed, because a page's own whitespace is not
//!   part of what it said, and an answer that differs only in trailing spaces
//!   has not changed.

use crate::Struct::Selectors::Selectors;

/// A string as a JavaScript literal, so a selector cannot end the expression.
fn Literal(Value:&str) -> String { serde_json::to_string(Value).unwrap_or_else(|_| "\"\"".to_string()) }

/// A label reader, repeated in the expressions that read one.
fn Label() -> &'static str {
	"(Element) => (Element.getAttribute(\"aria-label\") || Element.innerText || \"\").trim().toLowerCase()"
}

/// The composer's mode: the label of the single pressed toggle.
///
/// `null` when there is no toggle, when none is pressed, or when several are -
/// a page that cannot say which mode it is in has not said it is in search
/// mode.
pub fn ReadMode(Selectors:&Selectors) -> String {
	format!(
		"(() => {{\n\tconst Label = {};\n\tconst Pressed = [...document.querySelectorAll({})].filter((Toggle) => \
		 Toggle.getAttribute(\"aria-pressed\") === \"true\");\n\treturn Pressed.length === 1 ? Label(Pressed[0]) : \
		 null;\n}})()",
		Label(),
		Literal(&Selectors.Mode)
	)
}

/// Press the mode toggle.
///
/// The toggle that reads `search` is preferred, so a product that renders one
/// button per mode is switched rather than cycled; a single cycling toggle is
/// the fallback. Pressing is not verifying - the caller reads the mode again
/// and decides - so this returns whether anything was there to press.
pub fn PressMode(Selectors:&Selectors) -> String {
	format!(
		"(() => {{\n\tconst Label = {};\n\tconst Toggles = [...document.querySelectorAll({})];\n\tconst Wanted = \
		 Toggles.find((Toggle) => Label(Toggle) === \"search\");\n\tconst Target = Wanted === undefined ? Toggles[0] \
		 : Wanted;\n\tif (Target === undefined) return false;\n\tTarget.click();\n\treturn true;\n}})()",
		Label(),
		Literal(&Selectors.Mode)
	)
}

/// The model label the composer will send to, as the page shows it.
pub fn ReadModel(Selectors:&Selectors) -> String {
	format!(
		"(() => {{\n\tconst Label = document.querySelector({});\n\tif (Label === null) return null;\n\treturn \
		 (Label.innerText || Label.getAttribute(\"aria-label\") || \"\").trim();\n}})()",
		Literal(&Selectors.Model)
	)
}

/// Put the question in the composer.
///
/// A `textarea` or an `input` takes it through the property's own setter, so a
/// framework that tracks the value sees the change; anything else is treated as
/// a contenteditable and takes a real edit - a selection plus `insertText` -
/// because an editor that keeps its own document model ignores a written
/// `innerText` and leaves its send control disabled. Both paths dispatch
/// `input`, because a composer that never saw one keeps that control disabled
/// and the question would sit there looking sent.
pub fn TypeQuestion(Selectors:&Selectors, Text:&str) -> String {
	format!(
		"(() => {{\n	const Composer = document.querySelector({});\n	if (Composer === null) return false;\n	const \
		 Text = {};\n	Composer.focus();\n	if (Composer.tagName === \"TEXTAREA\" || Composer.tagName === \"INPUT\") \
		 {{\n		const Proto = Composer.tagName === \"TEXTAREA\" ? HTMLTextAreaElement.prototype : \
		 HTMLInputElement.prototype;\n		const Set = Object.getOwnPropertyDescriptor(Proto, \"value\").set;\n		\
		 Set.call(Composer, Text);\n		Composer.dispatchEvent(new Event(\"input\", {{ bubbles: true }}));\n		\
		 Composer.dispatchEvent(new Event(\"change\", {{ bubbles: true }}));\n		return true;\n	}}\n	const \
		 Range = document.createRange();\n	Range.selectNodeContents(Composer);\n	const Selection = \
		 window.getSelection();\n	Selection.removeAllRanges();\n	Selection.addRange(Range);\n	if \
		 (((Composer.innerText || \"\").trim() === \"\" || document.execCommand(\"delete\")) && document.execCommand(\"insertText\", false, Text)) return true;\n	Composer.innerText = Text;\n	\
		 Composer.dispatchEvent(new InputEvent(\"input\", {{ bubbles: true, inputType: \"insertText\", data: Text \
		 }}));\n	return true;\n}})()",
		Literal(&Selectors.Composer),
		Literal(Text)
	)
}

/// Press the send control, and say what stopped it when it could not be
/// pressed.
///
/// A disabled control is not clicked: the click would land on nothing, the page
/// would look sent, and the caller would wait out its whole deadline for an
/// answer that was never asked for.
///
/// The composer is read back first, because an editor can keep its own model
/// while the DOM shows something else - an insert that reported success over a
/// draft still holding the previous question. Sending that is worse than sending
/// nothing: the answer would be to a question nobody asked, and nothing in the
/// result would say so. So a composer that does not hold the question is
/// `mismatch`, and no click happens.
pub fn PressSubmit(Selectors:&Selectors, Expected:&str) -> String {
	format!(
		"(() => {{\n	const Composer = document.querySelector({});\n	const Strip = (Text) => (Text || \"\").replace(/\\s+/g, \" \").trim();\n	const \
		 Wanted = Strip({});\n	const Held = Composer === null ? null : Strip(Composer.innerText || Composer.value || \"\");\n	if (Held !== null && Held !== Wanted) \
		 return \"mismatch\";\n	const Submit = document.querySelector({});\n	if (Submit === null) return \"missing\";\n	if (Submit.disabled === true) return \
		 \"disabled\";\n	Submit.click();\n	return \"sent\";\n}})()",
		Literal(&Selectors.Composer),
		Literal(Expected),
		Literal(&Selectors.Submit)
	)
}

/// How many answers the thread holds, the text of the newest one, and whether
/// the page says it is still producing it.
///
/// `busy` is read here rather than in a probe of its own because it is a fact
/// about the same moment as the text: a separate read would describe a
/// different instant, and the two would disagree exactly when it matters.
///
/// It has two halves, and both are the page's own statements. A visible stop
/// control means the page is working now. A turn whose text does not yet read
/// as finished means the page has not said it is done - which covers the window
/// before a stop control appears, where a turn is demonstrably not finished and
/// nothing on the page says so.
pub fn ReadAnswers(Selectors:&Selectors) -> String {
	// An empty `done` is not a pattern that matches nothing: it is the caller
	// saying the product has no terminal marker, which turns the check off
	// rather than making every turn look unfinished.
	let Pattern = if Selectors.Done.trim().is_empty() {
		"null".to_string()
	} else {
		format!("new RegExp({})", Literal(&Selectors.Done))
	};
	format!(
		"(() => {{\n	// A virtualised thread does not mount what is off-screen, so a read that\n	// does not return to the end measures whichever turns happen to be\n	// mounted - which is how an old answer gets read as a new one. The page's\n	// own control is tried first; scrolling the last answer into view is the\n	// fallback for a page that has no such control.\n	const End = document.querySelector({});\n	if (End !== null && End.getClientRects().length > 0) End.click();\n	const Answers = [...document.querySelectorAll({})];\n	const Tail = Answers[Answers.length - 1];\n	if (Tail !== undefined && Tail.getBoundingClientRect().bottom > window.innerHeight) \
		 Tail.scrollIntoView({{block: \"end\", behavior: \"instant\" }});\n	const Stopping = [...document.querySelectorAll({})].some((Control) => \
		 Control.getClientRects().length > 0 && Control.disabled !== true);\n	const Pattern = {};\n	const Unfinished = (Answer) => {{\n		if \
		 (Pattern === null) return false;\n		const Turn = {} === '' ? Answer : Answer.closest({});\n		if (Turn === null) return true;\n		return !Pattern.test((Turn.innerText || \
		 \"\").trim());\n	}};\n	const Seen = (Answer) => (Answer.innerText || \"\").trim();\n	const Mark = (Text) => [...Text].length + \":\" + \
		 [...Text].slice(0, 48).join(\"\");\n	if (Answers.length === 0) return {{ count: 0, text: \"\", marks: [], busy: Stopping }};\n	const Last = \
		 Answers[Answers.length - 1];\n	return {{ count: Answers.length, text: Seen(Last), marks: Answers.map((Answer) => Mark(Seen(Answer))), busy: \
		 Stopping || Unfinished(Last) }};\n}})()",
		Literal(&Selectors.Scroll),
		Literal(&Selectors.Answer),
		Literal(&Selectors.Busy),
		Pattern,
		Literal(&Selectors.Turn),
		Literal(&Selectors.Turn)
	)
}

/// The label of the control that collapses the newest answer, or `null`.
///
/// Only `Read more` counts. `Read less` is the label of the control that
/// expands an answer again, so an answer already open is complete and its text
/// is what the caller asked for.
pub fn ReadCollapse(Selectors:&Selectors) -> String {
	format!(
		"(() => {{\n\tconst Answers = [...document.querySelectorAll({})];\n\tif (Answers.length === 0) return \
		 null;\n\tconst Last = Answers[Answers.length - 1];\n\tfor (const Control of [...Last.querySelectorAll({})]) \
		 {{\n\t\tconst Label = (Control.getAttribute(\"aria-label\") || Control.innerText || \"\").trim();\n\t\tif \
		 (/^read more$/i.test(Label)) return Label;\n\t}}\n\treturn null;\n}})()",
		Literal(&Selectors.Answer),
		Literal(&Selectors.Collapse)
	)
}

/// The work the newest answer reports having done, in the order it did it.
///
/// Scoped to the newest answer for the same reason the collapse control is: a
/// thread holds every turn's steps, and a document-wide read would hand back
/// the trace of a question the caller did not ask. No answer yet is no steps.
pub fn ReadTrace(Selectors:&Selectors) -> String {
	format!(
		"(() => {{\n\tconst Answers = [...document.querySelectorAll({})];\n\tif (Answers.length === 0) return \
		 [];\n\tconst Last = Answers[Answers.length - 1];\n\treturn [...Last.querySelectorAll({})].map((Step) => \
		 ({{\n\t\tdepth: Number(Step.getAttribute(\"data-depth\") || 0),\n\t\tlabel: (Step.innerText || \
		 \"\").trim(),\n\t\tdetail: (Step.getAttribute(\"data-detail\") || Step.getAttribute(\"aria-label\") || \
		 \"\").trim(),\n\t}}));\n}})()",
		Literal(&Selectors.Answer),
		Literal(&Selectors.Trace)
	)
}

/// The sources the newest answer cites.
///
/// Scoped to the newest answer, because the sources of an earlier turn are that
/// turn's: an answer that cites nothing is a different fact from an answer
/// whose thread happens to contain citations elsewhere.
pub fn ReadSources(Selectors:&Selectors) -> String {
	format!(
		"(() => {{\n\tconst Answers = [...document.querySelectorAll({})];\n\tif (Answers.length === 0) return \
		 [];\n\tconst Last = Answers[Answers.length - 1];\n\treturn [...Last.querySelectorAll({})].map((Source) => \
		 ({{\n\t\turl: Source.getAttribute(\"href\") || \"\",\n\t\tlabel: (Source.innerText || \
		 \"\").trim(),\n\t}}));\n}})()",
		Literal(&Selectors.Answer),
		Literal(&Selectors.Sources)
	)
}

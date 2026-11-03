//! The expressions the driver sends to a page.
//!
//! These are the whole protocol between the capability and the browser. They are
//! deliberately small and literal: each one reads one thing, returns a plain value,
//! and touches nothing but `document`. That is what lets the same driver run
//! against a real tab and against a fake, which answers by recognising the
//! expression it was handed rather than by matching a string this crate happened
//! to send.
//!
//! Two conventions hold across all of them:
//!
//! - a value the page does not have is `null`, never a guess. A missing toggle is
//!   not "search", a missing label is not the model the caller asked for, and a
//!   missing answer is not an empty answer;
//! - the text they return is trimmed, because a page's own whitespace is not part
//!   of what it said, and an answer that differs only in trailing spaces has not
//!   changed.

use crate::Struct::Selectors::Selectors;

/// A string as a JavaScript literal, so a selector cannot end the expression.
fn Literal(Value: &str) -> String {
	serde_json::to_string(Value).unwrap_or_else(|_| "\"\"".to_string())
}

/// A label reader, repeated in the expressions that read one.
fn Label() -> &'static str {
	"(Element) => (Element.getAttribute(\"aria-label\") || Element.innerText || \"\").trim().toLowerCase()"
}

/// The composer's mode: the label of the single pressed toggle.
///
/// `null` when there is no toggle, when none is pressed, or when several are - a
/// page that cannot say which mode it is in has not said it is in search mode.
pub fn ReadMode(Selectors: &Selectors) -> String {
	format!(
		"(() => {{\n\tconst Label = {};\n\tconst Pressed = [...document.querySelectorAll({})].filter((Toggle) => Toggle.getAttribute(\"aria-pressed\") === \"true\");\n\treturn Pressed.length === 1 ? Label(Pressed[0]) : null;\n}})()",
		Label(),
		Literal(&Selectors.Mode)
	)
}

/// Press the mode toggle.
///
/// The toggle that reads `search` is preferred, so a product that renders one
/// button per mode is switched rather than cycled; a single cycling toggle is the
/// fallback. Pressing is not verifying - the caller reads the mode again and
/// decides - so this returns whether anything was there to press.
pub fn PressMode(Selectors: &Selectors) -> String {
	format!(
		"(() => {{\n\tconst Label = {};\n\tconst Toggles = [...document.querySelectorAll({})];\n\tconst Wanted = Toggles.find((Toggle) => Label(Toggle) === \"search\");\n\tconst Target = Wanted === undefined ? Toggles[0] : Wanted;\n\tif (Target === undefined) return false;\n\tTarget.click();\n\treturn true;\n}})()",
		Label(),
		Literal(&Selectors.Mode)
	)
}

/// The model label the composer will send to, as the page shows it.
pub fn ReadModel(Selectors: &Selectors) -> String {
	format!(
		"(() => {{\n\tconst Label = document.querySelector({});\n\tif (Label === null) return null;\n\treturn (Label.innerText || Label.getAttribute(\"aria-label\") || \"\").trim();\n}})()",
		Literal(&Selectors.Model)
	)
}

/// Put the question in the composer.
///
/// A `textarea` or an `input` takes it as a value; anything else is treated as a
/// contenteditable and takes it as text. Both paths dispatch `input`, because a
/// composer that never saw an input event keeps its send control disabled and the
/// question would sit there looking sent.
pub fn TypeQuestion(Selectors: &Selectors, Text: &str) -> String {
	format!(
		"(() => {{\n\tconst Composer = document.querySelector({});\n\tif (Composer === null) return false;\n\tconst Text = {};\n\tif (Composer.tagName === \"TEXTAREA\" || Composer.tagName === \"INPUT\") Composer.value = Text;\n\telse Composer.innerText = Text;\n\tComposer.dispatchEvent(new Event(\"input\", {{ bubbles: true }}));\n\treturn true;\n}})()",
		Literal(&Selectors.Composer),
		Literal(Text)
	)
}

/// Press the send control, and say what stopped it when it could not be pressed.
///
/// A disabled control is not clicked: the click would land on nothing, the page
/// would look sent, and the caller would wait out its whole deadline for an answer
/// that was never asked for.
pub fn PressSubmit(Selectors: &Selectors) -> String {
	format!(
		"(() => {{\n\tconst Submit = document.querySelector({});\n\tif (Submit === null) return \"missing\";\n\tif (Submit.disabled === true) return \"disabled\";\n\tSubmit.click();\n\treturn \"sent\";\n}})()",
		Literal(&Selectors.Submit)
	)
}

/// How many answers the thread holds, and the text of the newest one.
pub fn ReadAnswers(Selectors: &Selectors) -> String {
	format!(
		"(() => {{\n\tconst Answers = [...document.querySelectorAll({})];\n\tif (Answers.length === 0) return {{ count: 0, text: \"\" }};\n\tconst Last = Answers[Answers.length - 1];\n\treturn {{ count: Answers.length, text: (Last.innerText || \"\").trim() }};\n}})()",
		Literal(&Selectors.Answer)
	)
}

/// The label of the control that collapses the newest answer, or `null`.
///
/// Only `Read more` counts. `Read less` is the label of the control that expands
/// an answer again, so an answer already open is complete and its text is what the
/// caller asked for.
pub fn ReadCollapse(Selectors: &Selectors) -> String {
	format!(
		"(() => {{\n\tconst Answers = [...document.querySelectorAll({})];\n\tif (Answers.length === 0) return null;\n\tconst Last = Answers[Answers.length - 1];\n\tfor (const Control of [...Last.querySelectorAll({})]) {{\n\t\tconst Label = (Control.getAttribute(\"aria-label\") || Control.innerText || \"\").trim();\n\t\tif (/^read more$/i.test(Label)) return Label;\n\t}}\n\treturn null;\n}})()",
		Literal(&Selectors.Answer),
		Literal(&Selectors.Collapse)
	)
}

/// The work the newest answer reports having done, in the order it did it.
///
/// Scoped to the newest answer for the same reason the collapse control is: a
/// thread holds every turn's steps, and a document-wide read would hand back the
/// trace of a question the caller did not ask. No answer yet is no steps.
pub fn ReadTrace(Selectors: &Selectors) -> String {
	format!(
		"(() => {{\n\tconst Answers = [...document.querySelectorAll({})];\n\tif (Answers.length === 0) return [];\n\tconst Last = Answers[Answers.length - 1];\n\treturn [...Last.querySelectorAll({})].map((Step) => ({{\n\t\tdepth: Number(Step.getAttribute(\"data-depth\") || 0),\n\t\tlabel: (Step.innerText || \"\").trim(),\n\t\tdetail: (Step.getAttribute(\"data-detail\") || Step.getAttribute(\"aria-label\") || \"\").trim(),\n\t}}));\n}})()",
		Literal(&Selectors.Answer),
		Literal(&Selectors.Trace)
	)
}

/// The sources the newest answer cites.
///
/// Scoped to the newest answer, because the sources of an earlier turn are that
/// turn's: an answer that cites nothing is a different fact from an answer whose
/// thread happens to contain citations elsewhere.
pub fn ReadSources(Selectors: &Selectors) -> String {
	format!(
		"(() => {{\n\tconst Answers = [...document.querySelectorAll({})];\n\tif (Answers.length === 0) return [];\n\tconst Last = Answers[Answers.length - 1];\n\treturn [...Last.querySelectorAll({})].map((Source) => ({{\n\t\turl: Source.getAttribute(\"href\") || \"\",\n\t\tlabel: (Source.innerText || \"\").trim(),\n\t}}));\n}})()",
		Literal(&Selectors.Answer),
		Literal(&Selectors.Sources)
	)
}

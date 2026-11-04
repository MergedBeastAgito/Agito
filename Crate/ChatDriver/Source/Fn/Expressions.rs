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
//!
//! # Where the JavaScript is
//!
//! Not in this file. Each expression is one file under `Source/Script/`, taken
//! whole by `include_str!` and sent as it stands, so an editor, a linter and a
//! compiler can see it - which a `format!` never let them do. A value the
//! caller supplies is written where it belongs as a `{{name}}` placeholder and
//! filled by `AgenticRuntime::Fill`; the parameters a function declares are the
//! parameters its file names, and the test at the bottom of this module holds
//! that agreement to the bytes.
//!
//! What did not move: the functions, their names and their signatures. This
//! module is still the list of what the driver can say, and the script behind
//! each one is still one expression that reads one thing.

use AgenticRuntime::Fill;

use crate::Struct::{Format::Format, Selectors::Selectors};

/// A string as a JavaScript literal, so a selector cannot end the expression.
fn Literal(Value:&str) -> String { serde_json::to_string(Value).unwrap_or_else(|_| "\"\"".to_string()) }

/// The script behind [`ReadMode`].
const ReadModeScript:&str = include_str!("../Script/ReadMode.js");

/// The script behind [`PressMode`].
const PressModeScript:&str = include_str!("../Script/PressMode.js");

/// The script behind [`ReadModel`].
const ReadModelScript:&str = include_str!("../Script/ReadModel.js");

/// The script behind [`TypeQuestion`].
const TypeQuestionScript:&str = include_str!("../Script/TypeQuestion.js");

/// The script behind [`PressSubmit`].
const PressSubmitScript:&str = include_str!("../Script/PressSubmit.js");

/// The script behind [`ReadAnswers`].
const ReadAnswersScript:&str = include_str!("../Script/ReadAnswers.js");

/// The serializer [`ReadAnswers`] splices into that script, for
/// [`Format::Markdown`] only.
const SerialiseScript:&str = include_str!("../Script/Extract.js");

/// The script behind [`ReadCollapse`].
const ReadCollapseScript:&str = include_str!("../Script/ReadCollapse.js");

/// The script behind [`ReadTrace`].
const ReadTraceScript:&str = include_str!("../Script/ReadTrace.js");

/// The script behind [`ReadSources`].
const ReadSourcesScript:&str = include_str!("../Script/ReadSources.js");

/// The composer's mode: the label of the single pressed toggle.
///
/// `null` when there is no toggle, when none is pressed, or when several are -
/// a page that cannot say which mode it is in has not said it is in search
/// mode.
pub fn ReadMode(Selectors:&Selectors) -> String { Fill(ReadModeScript, &[("mode", &Literal(&Selectors.Mode))]) }

/// Press the mode toggle.
///
/// The toggle that reads `search` is preferred, so a product that renders one
/// button per mode is switched rather than cycled; a single cycling toggle is
/// the fallback. Pressing is not verifying - the caller reads the mode again
/// and decides - so this returns whether anything was there to press.
pub fn PressMode(Selectors:&Selectors) -> String { Fill(PressModeScript, &[("mode", &Literal(&Selectors.Mode))]) }

/// The model label the composer will send to, as the page shows it.
pub fn ReadModel(Selectors:&Selectors) -> String { Fill(ReadModelScript, &[("model", &Literal(&Selectors.Model))]) }

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
	Fill(
		TypeQuestionScript,
		&[("composer", &Literal(&Selectors.Composer)), ("text", &Literal(Text))],
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
/// draft still holding the previous question. Sending that is worse than
/// sending nothing: the answer would be to a question nobody asked, and nothing
/// in the result would say so. So a composer that does not hold the question is
/// `mismatch`, and no click happens.
pub fn PressSubmit(Selectors:&Selectors, Expected:&str) -> String {
	Fill(
		PressSubmitScript,
		&[
			("composer", &Literal(&Selectors.Composer)),
			("expected", &Literal(Expected)),
			("submit", &Literal(&Selectors.Submit)),
		],
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
///
/// `Format` decides what one answer reads as, and it is a parameter rather than
/// a second function because the rest of the read is identical either way: the
/// same element, the same scroll, the same busy control, the same marks. Only
/// the closure that turns an element into text changes, which is why the
/// observation stream and the completion rule cannot tell the two apart - they
/// compare text to text, not text to a format.
pub fn ReadAnswers(Selectors:&Selectors, Format:Format) -> String {
	// An empty `done` is not a pattern that matches nothing: it is the caller
	// saying the product has no terminal marker, which turns the check off
	// rather than making every turn look unfinished.
	let Pattern = if Selectors.Done.trim().is_empty() {
		"null".to_string()
	} else {
		format!("new RegExp({})", Literal(&Selectors.Done))
	};
	// The serializer is spliced in only when it is asked for: it is thirteen
	// kilobytes the page would parse on every read of the thread, and the
	// recorded baselines were read without it.
	let Serialise = match Format {
		Format::Text => "",
		Format::Markdown => SerialiseScript,
	};
	// What one answer reads as. The same closure produces the newest answer's
	// text and every answer's mark, so the driver's own mark and the page's
	// cannot be computed from two different readings of the same turn.
	let Seen = match Format {
		Format::Text => "(Answer) => (Answer.innerText || \"\").trim()",
		Format::Markdown => "(Answer) => Serialise(Answer)",
	};
	Fill(
		ReadAnswersScript,
		&[
			("serialise", Serialise),
			("scroll", &Literal(&Selectors.Scroll)),
			("answer", &Literal(&Selectors.Answer)),
			("busy", &Literal(&Selectors.Busy)),
			("pattern", &Pattern),
			("turn", &Literal(&Selectors.Turn)),
			("seen", Seen),
		],
	)
}

/// The label of the control that collapses the newest answer, or `null`.
///
/// Only `Read more` counts. `Read less` is the label of the control that
/// expands an answer again, so an answer already open is complete and its text
/// is what the caller asked for.
pub fn ReadCollapse(Selectors:&Selectors) -> String {
	Fill(
		ReadCollapseScript,
		&[
			("answer", &Literal(&Selectors.Answer)),
			("collapse", &Literal(&Selectors.Collapse)),
		],
	)
}

/// The work the newest answer reports having done, in the order it did it.
///
/// Scoped to the newest answer for the same reason the collapse control is: a
/// thread holds every turn's steps, and a document-wide read would hand back
/// the trace of a question the caller did not ask. No answer yet is no steps.
pub fn ReadTrace(Selectors:&Selectors) -> String {
	Fill(
		ReadTraceScript,
		&[("answer", &Literal(&Selectors.Answer)), ("trace", &Literal(&Selectors.Trace))],
	)
}

/// The sources the newest answer cites.
///
/// Scoped to the newest answer, because the sources of an earlier turn are that
/// turn's: an answer that cites nothing is a different fact from an answer
/// whose thread happens to contain citations elsewhere.
pub fn ReadSources(Selectors:&Selectors) -> String {
	Fill(
		ReadSourcesScript,
		&[
			("answer", &Literal(&Selectors.Answer)),
			("sources", &Literal(&Selectors.Sources)),
		],
	)
}

#[cfg(test)]
mod Tests {
	use AgenticRuntime::Unfilled;

	use super::*;
	use crate::Struct::Selectors::DefaultSelectors;

	/// Every script this module sends, and the placeholders its function
	/// declares.
	///
	/// The list is the contract between the two halves: a placeholder a file
	/// gains without the function declaring it reaches the page as `{{name}}`
	/// and throws there, and a parameter a function passes without the file
	/// naming it is a value that silently goes nowhere. Neither is caught
	/// by a test that only inspects the expression the driver builds - a
	/// placeholder left in place is still a string, and the fake answers by
	/// kind rather than by compiling the JavaScript.
	const Scripts:[(&str, &str, &[&str]); 9] = [
		("ReadMode", ReadModeScript, &["mode"]),
		("PressMode", PressModeScript, &["mode"]),
		("ReadModel", ReadModelScript, &["model"]),
		("TypeQuestion", TypeQuestionScript, &["composer", "text"]),
		("PressSubmit", PressSubmitScript, &["composer", "expected", "submit"]),
		(
			"ReadAnswers",
			ReadAnswersScript,
			&["serialise", "scroll", "answer", "busy", "pattern", "turn", "seen"],
		),
		("ReadCollapse", ReadCollapseScript, &["answer", "collapse"]),
		("ReadTrace", ReadTraceScript, &["answer", "trace"]),
		("ReadSources", ReadSourcesScript, &["answer", "sources"]),
	];

	/// Every expression the driver builds, with the two optional selector
	/// fields set, so no parameter is exercised only in its empty form.
	fn Built() -> Vec<(&'static str, String)> {
		let Selectors = Selectors {
			Done:"^\\s*done\\s*$".to_string(),
			Turn:"[data-testid='turn']".to_string(),
			..DefaultSelectors()
		};
		let Question = "a question";
		vec![
			("ReadMode", ReadMode(&Selectors)),
			("PressMode", PressMode(&Selectors)),
			("ReadModel", ReadModel(&Selectors)),
			("TypeQuestion", TypeQuestion(&Selectors, Question)),
			("PressSubmit", PressSubmit(&Selectors, Question)),
			("ReadAnswers/text", ReadAnswers(&Selectors, Format::Text)),
			("ReadAnswers/markdown", ReadAnswers(&Selectors, Format::Markdown)),
			("ReadCollapse", ReadCollapse(&Selectors)),
			("ReadTrace", ReadTrace(&Selectors)),
			("ReadSources", ReadSources(&Selectors)),
		]
	}

	/// A script is filled completely, and the check that says so bites.
	///
	/// The second half is the one that matters: `Unfilled` answering `None` for
	/// every expression proves nothing unless it answers `Some` for a string
	/// that still carries a placeholder, and the raw script is that string.
	#[test]
	fn every_expression_is_filled_and_no_script_is_left_with_a_placeholder() {
		for (Name, Raw, Parameters) in Scripts {
			for Parameter in Parameters {
				assert!(
					Raw.contains(&format!("{{{{{Parameter}}}}}")),
					"{Name} does not name {{{{{Parameter}}}}}, so the value would go nowhere"
				);
			}
			assert_eq!(
				Unfilled(Raw).as_deref(),
				Some(Parameters[0]),
				"{Name}'s script carries no placeholder, so nothing proves the fill ran"
			);
		}

		for (Name, Text) in Built() {
			assert_eq!(
				Unfilled(&Text),
				None,
				"{Name} left a placeholder unsubstituted in the expression it sends"
			);
		}
	}
}

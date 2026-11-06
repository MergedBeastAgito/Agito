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
//! Not in this file. Each expression is one TypeScript file under
//! `Source/Script/Expression/`, compiled by this crate's `build.rs` into
//! JavaScript and baked in by [`script!`] - so an editor, a linter and a
//! compiler see the expression that a `format!` used to hide, and the file the
//! compiler checks is the file the page runs.
//!
//! A value the caller supplies is a parameter the script declares:
//! `declare const __PARAM_Scroll__: string;` at the top of the file is both the
//! type a reader can see and the declaration the build reads, and `Scroll =
//! &Selectors.Scroll` at the call site is checked against it at compile time.
//! A parameter declared `string` is escaped into a JavaScript string literal;
//! anything else is code and is spliced as it stands. The compiled expression
//! is byte for byte the JavaScript this module used to build with `format!`.
//!
//! What did not move: the functions, their names and their signatures. This
//! module is still the list of what the driver can say, and the script behind
//! each one is still one expression that reads one thing.

use ::RunTime::{Escape, script};

use crate::Struct::{Format::Format, Selectors::Selectors};

/// The serializer [`ReadAnswers`] splices into that script, for
/// [`Format::Markdown`] only.
const SerialiseScript:&str = script!("Expression/Extract");

/// The composer's mode: the label of the single pressed toggle.
///
/// `null` when there is no toggle, when none is pressed, or when several are -
/// a page that cannot say which mode it is in has not said it is in search
/// mode.
pub fn ReadMode(Selectors:&Selectors) -> String { script!("Expression/ReadMode"; Mode = &Selectors.Mode) }

/// Press the mode toggle.
///
/// The toggle that reads `search` is preferred, so a product that renders one
/// button per mode is switched rather than cycled; a single cycling toggle is
/// the fallback. Pressing is not verifying - the caller reads the mode again
/// and decides - so this returns whether anything was there to press.
pub fn PressMode(Selectors:&Selectors) -> String { script!("Expression/PressMode"; Mode = &Selectors.Mode) }

/// The model label the composer will send to, as the page shows it.
pub fn ReadModel(Selectors:&Selectors) -> String { script!("Expression/ReadModel"; Model = &Selectors.Model) }

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
	script!("Expression/TypeQuestion"; Composer = &Selectors.Composer, Text = Text)
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
	script!(
		"Expression/PressSubmit";
		Composer = &Selectors.Composer,
		Expected = Expected,
		Submit = &Selectors.Submit,
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
///
/// [`Format::Both`] is the exception that proves the rule: it changes neither
/// the closure nor the thread read, only the object the expression returns. The
/// text half stays `innerText`, so the completion rule and the turn marks are
/// the ones a `Text` call produces, and the page's Markdown is added beside it
/// from the same `Last` element. One evaluate, one snapshot, two renderings -
/// which is the only way the pair can be trusted to describe the same answer,
/// because two evaluates are two moments.
///
/// It is built from a second script file, `Source/Script/Expression/ReadBoth.ts`,
/// and not from a third parameter of `ReadAnswers.ts`: an expression that gained
/// a field would not be the expression a page already received. The two files
/// carry one rule, and they are written so the thread read is the same text in
/// both - which is what keeps a text call and a both call settling on the same
/// evidence.
pub fn ReadAnswers(Selectors:&Selectors, Format:Format) -> String {
	// An empty `done` is not a pattern that matches nothing: it is the caller
	// saying the product has no terminal marker, which turns the check off
	// rather than making every turn look unfinished.
	let Pattern = if Selectors.Done.trim().is_empty() {
		"null".to_string()
	} else {
		format!("new RegExp({})", Escape(&Selectors.Done))
	};
	match Format {
		Format::Both => {
			script!(
				"Expression/ReadBoth";
				Serialise = SerialiseScript,
				Scroll = &Selectors.Scroll,
				Answer = &Selectors.Answer,
				Busy = &Selectors.Busy,
				Pattern = Pattern,
				Turn = &Selectors.Turn,
			)
		},
		Format::Text | Format::Markdown => {
			// The serializer is spliced in only when it is asked for: it is
			// thirteen kilobytes the page would parse on every read of the
			// thread, and the recorded baselines were read without it.
			let Serialise = if Format == Format::Text { "" } else { SerialiseScript };
			// What one answer reads as. The same closure produces the newest
			// answer's text and every answer's mark, so the driver's own mark
			// and the page's cannot be computed from two different readings of
			// the same turn.
			let Seen = if Format == Format::Text {
				"(Answer) => (Answer.innerText || \"\").trim()"
			} else {
				"(Answer) => Serialise(Answer)"
			};
			script!(
				"Expression/ReadAnswers";
				Serialise = Serialise,
				Scroll = &Selectors.Scroll,
				Answer = &Selectors.Answer,
				Busy = &Selectors.Busy,
				Pattern = Pattern,
				Turn = &Selectors.Turn,
				Seen = Seen,
			)
		},
	}
}

/// The label of the control that collapses the newest answer, or `null`.
///
/// Only `Read more` counts. `Read less` is the label of the control that
/// expands an answer again, so an answer already open is complete and its text
/// is what the caller asked for.
pub fn ReadCollapse(Selectors:&Selectors) -> String {
	script!("Expression/ReadCollapse"; Answer = &Selectors.Answer, Collapse = &Selectors.Collapse)
}

/// The work the newest answer reports having done, in the order it did it.
///
/// Scoped to the newest answer for the same reason the collapse control is: a
/// thread holds every turn's steps, and a document-wide read would hand back
/// the trace of a question the caller did not ask. No answer yet is no steps.
pub fn ReadTrace(Selectors:&Selectors) -> String {
	script!("Expression/ReadTrace"; Answer = &Selectors.Answer, Trace = &Selectors.Trace)
}

/// The sources the newest answer cites.
///
/// Scoped to the newest answer, because the sources of an earlier turn are that
/// turn's: an answer that cites nothing is a different fact from an answer
/// whose thread happens to contain citations elsewhere.
pub fn ReadSources(Selectors:&Selectors) -> String {
	script!("Expression/ReadSources"; Answer = &Selectors.Answer, Sources = &Selectors.Sources)
}

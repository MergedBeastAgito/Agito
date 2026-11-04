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
//! `Source/Script/Expressions/`, compiled by this crate's `build.rs` into
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

use AgenticRuntime::{Escape, script};

use crate::Struct::{Format::Format, Selectors::Selectors};

/// The serializer [`ReadAnswers`] splices into that script, for
/// [`Format::Markdown`] only.
const SerialiseScript:&str = script!("Expressions/Extract");

/// The composer's mode: the label of the single pressed toggle.
///
/// `null` when there is no toggle, when none is pressed, or when several are -
/// a page that cannot say which mode it is in has not said it is in search
/// mode.
pub fn ReadMode(Selectors:&Selectors) -> String { script!("Expressions/ReadMode"; Mode = &Selectors.Mode) }

/// Press the mode toggle.
///
/// The toggle that reads `search` is preferred, so a product that renders one
/// button per mode is switched rather than cycled; a single cycling toggle is
/// the fallback. Pressing is not verifying - the caller reads the mode again
/// and decides - so this returns whether anything was there to press.
pub fn PressMode(Selectors:&Selectors) -> String { script!("Expressions/PressMode"; Mode = &Selectors.Mode) }

/// The model label the composer will send to, as the page shows it.
pub fn ReadModel(Selectors:&Selectors) -> String { script!("Expressions/ReadModel"; Model = &Selectors.Model) }

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
	script!("Expressions/TypeQuestion"; Composer = &Selectors.Composer, Text = Text)
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
		"Expressions/PressSubmit";
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
pub fn ReadAnswers(Selectors:&Selectors, Format:Format) -> String {
	// An empty `done` is not a pattern that matches nothing: it is the caller
	// saying the product has no terminal marker, which turns the check off
	// rather than making every turn look unfinished.
	let Pattern = if Selectors.Done.trim().is_empty() {
		"null".to_string()
	} else {
		format!("new RegExp({})", Escape(&Selectors.Done))
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
	script!(
		"Expressions/ReadAnswers";
		Serialise = Serialise,
		Scroll = &Selectors.Scroll,
		Answer = &Selectors.Answer,
		Busy = &Selectors.Busy,
		Pattern = Pattern,
		Turn = &Selectors.Turn,
		Seen = Seen,
	)
}

/// The label of the control that collapses the newest answer, or `null`.
///
/// Only `Read more` counts. `Read less` is the label of the control that
/// expands an answer again, so an answer already open is complete and its text
/// is what the caller asked for.
pub fn ReadCollapse(Selectors:&Selectors) -> String {
	script!("Expressions/ReadCollapse"; Answer = &Selectors.Answer, Collapse = &Selectors.Collapse)
}

/// The work the newest answer reports having done, in the order it did it.
///
/// Scoped to the newest answer for the same reason the collapse control is: a
/// thread holds every turn's steps, and a document-wide read would hand back
/// the trace of a question the caller did not ask. No answer yet is no steps.
pub fn ReadTrace(Selectors:&Selectors) -> String {
	script!("Expressions/ReadTrace"; Answer = &Selectors.Answer, Trace = &Selectors.Trace)
}

/// The sources the newest answer cites.
///
/// Scoped to the newest answer, because the sources of an earlier turn are that
/// turn's: an answer that cites nothing is a different fact from an answer
/// whose thread happens to contain citations elsewhere.
pub fn ReadSources(Selectors:&Selectors) -> String {
	script!("Expressions/ReadSources"; Answer = &Selectors.Answer, Sources = &Selectors.Sources)
}

#[cfg(test)]
mod Tests {
	use AgenticRuntime::Unfilled;

	use super::*;
	use crate::Struct::Selectors::DefaultSelectors;

	/// Every script this module sends, as it was compiled.
	///
	/// The parameter contract itself is a build-time check - a call site that
	/// disagrees with a script's declarations fails `cargo build` - so what is
	/// left to hold here is that the expressions are filled completely, and
	/// that the check which says so bites. The second half is the one that
	/// matters: `Unfilled` answering `None` for every expression proves nothing
	/// unless it answers `Some` for a string that still carries a marker, and
	/// the raw script is that string.
	const Raw:[(&str, &str); 10] = [
		("ReadMode", script!("Expressions/ReadMode")),
		("PressMode", script!("Expressions/PressMode")),
		("ReadModel", script!("Expressions/ReadModel")),
		("TypeQuestion", script!("Expressions/TypeQuestion")),
		("PressSubmit", script!("Expressions/PressSubmit")),
		("ReadAnswers", script!("Expressions/ReadAnswers")),
		("ReadCollapse", script!("Expressions/ReadCollapse")),
		("ReadTrace", script!("Expressions/ReadTrace")),
		("ReadSources", script!("Expressions/ReadSources")),
		("Extract", script!("Expressions/Extract")),
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
	#[test]
	fn every_expression_is_filled_and_no_script_is_left_with_a_marker() {
		for (Name, Text) in Built() {
			assert_eq!(
				Unfilled(&Text),
				None,
				"{Name} left a marker unsubstituted in the expression it sends"
			);
		}

		for (Name, Text) in Raw {
			let Carries = Unfilled(Text).is_some();
			assert!(
				Carries || Name == "Extract",
				"{Name}'s compiled script carries no marker, so nothing proves the fill ran"
			);
			if Name == "Extract" {
				// The serializer is a whole function spliced into another
				// script: it has no parameters of its own, which is why it is
				// the one script with nothing to fill.
				assert_eq!(Unfilled(Text), None, "Extract is spliced as it stands");
			}
		}
	}

	/// The compiled JavaScript is the JavaScript the page was already being
	/// sent, byte for byte.
	///
	/// The fixture is the *before* side of this card: every expression, built
	/// by the code at the commit before the compilation, dumped from that
	/// code rather than retyped. It is a baseline, not a snapshot to
	/// refresh - if a change to a script fails here, the change altered
	/// what a page receives, which is exactly the thing the compilation was
	/// not allowed to do.
	///
	/// This is the assertion that rules out a formatter: a compiler that
	/// re-indented, re-quoted, appended a semicolon or dropped a comment would
	/// pass a behavioural test and fail here.
	#[test]
	fn every_compiled_expression_is_byte_for_byte_the_one_the_page_received() {
		const Fixture:&str = include_str!("../../Test/Fixture/Expression.json");
		let Before:std::collections::BTreeMap<String, String> =
			serde_json::from_str(Fixture).expect("the fixture is a map of expression name to bytes");
		let Built = Built();
		assert_eq!(
			Built.len(),
			Before.len(),
			"the fixture covers a different set of expressions than the driver builds"
		);
		for (Name, Text) in Built {
			let Want = Before.get(Name).unwrap_or_else(|| panic!("{Name} is not in the fixture"));
			assert_eq!(&Text, Want, "{Name} is not the expression the page used to receive");
		}
	}
}

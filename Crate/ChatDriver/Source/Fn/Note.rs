//! Building a note out of an answer, and rendering it as the file a reader
//! opens.
//!
//! The render is two statements of one set of facts: a YAML block, because a
//! reader greps it, and a table, because a person reads it. Both come from
//! [`Note::Facts`], so a fact cannot appear in one and be missing from the
//! other - the table is not a second summary that can fall out of date, it is
//! the same list with the labels a human reads.
//!
//! # The page's furniture
//!
//! A capture is the page, and the page is mostly not the answer: a sidebar, an
//! account label, a mode switch, the project and a clock, the row of follow-up
//! prompts, a source count. All of it is a line, and none of it is a word
//! inside a sentence - which is the rule this module is built on:
//!
//! - **A line is dropped when every word on it belongs to the page.** A
//!   sidebar label, a clock, a duration, a count, a control. An answer that
//!   says `Computer mode runs tools on the caller's behalf` keeps its line,
//!   because `mode`, `runs` and `tools` are not the page's words.
//! - **A label may open a line.** A chat product writes the project and the
//!   account before the clock - `Example Project 1:42 AM` - so a known label is
//!   stripped from the front and the rest of the line is judged on its own.
//!   Longest label first, or `Example Project` would leave `Project` behind as
//!   a word the page never wrote.
//! - **The follow-up prompts go as a paragraph.** A chip's text is the page's
//!   suggestion and not the answer's, so no vocabulary can recognise it; what
//!   can be recognised is the prompt that closes the row, and the row is the
//!   block of lines that ends at it. The cost is stated rather than hidden: a
//!   capture that arrives without the blank line between the answer and the
//!   chips loses the answer's last paragraph, and a capture that arrives with
//!   one - which is what the page produces - loses exactly the chips.
//!
//! Nothing here is a substring match. `New` is furniture; `New York` is a line
//! the answer wrote, and the difference is the whole line rather than the word.
//!
//! # Nothing here can fail
//!
//! A note is derived from an answer the port has already refused to call
//! truncated, and every decision in this module has a total answer: a title
//! falls back to the thread, a timestamp that is not a date states no date, and
//! a line is either furniture or it is kept. So `agentic.chat.note` declares no
//! errors - a code this module cannot produce would be a comment in a manifest
//! rather than a claim about the code.

use crate::Struct::{
	Note::{Fact, Meta, Note, Vocabulary},
	Port::Answer,
};

/// The prompt a chat product puts under its follow-up chips.
const FollowUp:&str = "ask a follow-up";

/// Build the note a capture becomes.
///
/// The answer supplies the transcript, the counts and nothing else; the meta
/// supplies everything a page cannot say. The model and the mode the question
/// was asked in are deliberately not carried: they are facts about the asking,
/// and a note that recorded them would make two notes of one answer differ.
pub fn Build(Answer:Answer, Meta:Meta) -> Note {
	let Labels = LabelsOf(&Meta);
	// The transcript is the richest rendering the answer carries. A note built
	// from rendered text carries a formula as one glyph per line and a table as
	// a run of tabs; a call that read both renderings carries the page's own
	// Markdown beside the text, and that is the one a reader can follow. A call
	// that read one rendering carries that one in `Text`, so this picks
	// between two statements of the same answer rather than falling back to a
	// second reading of the page.
	let Rendering = if Answer.Markdown.is_empty() { &Answer.Text } else { &Answer.Markdown };
	let Transcript = Strip(Rendering, &Labels);
	Note {
		Title:Title(&Meta),
		Thread:Meta.Thread,
		Project:Meta.Project,
		Kind:Meta.Kind,
		Status:Meta.Status,
		Source:Meta.Source,
		Captured:Meta.Captured,
		Segments:Meta.Segments,
		Steps:Answer.Trace.len(),
		Sources:Answer.Sources.len(),
		Characters:Transcript.chars().count(),
		Transcript,
	}
}

/// Render a note as the file a reader opens: a frontmatter block, a heading, a
/// table of the same facts, then the transcript under its own heading.
///
/// The frontmatter keys are lowercase and their order is fixed, because a
/// reader greps them and a grep that has to allow for two spellings of a key is
/// a grep somebody writes twice. The heading repeats the title rather than a
/// row of the table doing it: a table that opens by repeating the heading above
/// it reads as two titles.
pub fn Render(Note:&Note) -> String {
	let Facts = Note.Facts();
	let mut Out = String::from("---\n");
	Out.push_str(&format!("title: {}\n", Quoted(&Note.Title)));
	for Fact in &Facts {
		if Fact.Key == "title" {
			continue;
		}
		Out.push_str(&format!("{}: {}\n", Fact.Key, Scalar(&Fact.Value)));
		// The date is derived from the capture and is in the block because the
		// catalogue greps it. It sits where the archived notes put it - after
		// the kind - so a reader diffing an old note against a new one sees the
		// facts that changed rather than a reordering.
		if Fact.Key == "kind"
			&& let Some(Date) = Date(&Note.Captured)
		{
			Out.push_str(&format!("date: {Date}\n"));
		}
	}
	Out.push_str("---\n\n");
	Out.push_str(&format!("# {}\n\n", Note.Title));
	Out.push_str(&Table(&Facts));
	Out.push_str("\n## Transcript\n\n");
	Out.push_str(&Note.Transcript);
	if !Note.Transcript.is_empty() {
		Out.push('\n');
	}
	Out
}

/// The transcript with the page's furniture taken out of it.
///
/// The text is a capture, so it is normalised to `\n` first: a page that
/// answered through a Windows clipboard and one that answered through a browser
/// must not produce two different notes.
///
/// Blank lines are kept as paragraph breaks and collapsed to one each, and the
/// text is trimmed at both ends but not at the end of its lines, because a
/// trailing space inside a line is not furniture - it is the answer's.
pub fn Strip(Text:&str, Labels:&[String]) -> String {
	let Normalised = Text.replace("\r\n", "\n").replace('\r', "\n");
	let Lines:Vec<&str> = Normalised.split('\n').collect();
	let mut Dropped = vec![false; Lines.len()];
	// The chips first, because they are only recognisable by where they sit.
	for (Index, Line) in Lines.iter().enumerate() {
		if !Line.trim().eq_ignore_ascii_case(FollowUp) {
			continue;
		}
		Dropped[Index] = true;
		let mut Back = Index;
		while Back > 0 {
			Back -= 1;
			if Lines[Back].trim().is_empty() {
				break;
			}
			Dropped[Back] = true;
		}
	}
	let mut Kept:Vec<&str> = Vec::new();
	for (Index, Line) in Lines.iter().enumerate() {
		if Dropped[Index] || Furniture(Line, Labels) {
			continue;
		}
		Kept.push(Line.trim_end());
	}
	let mut Out:Vec<&str> = Vec::new();
	for Line in Kept {
		let Blank = Line.trim().is_empty();
		if Blank && Out.last().map(|Last| Last.trim().is_empty()).unwrap_or(true) {
			continue;
		}
		Out.push(Line);
	}
	while Out.last().map(|Last| Last.trim().is_empty()).unwrap_or(false) {
		Out.pop();
	}
	Out.join("\n")
}

/// The labels a line may open with: the caller's vocabulary and the two words
/// the capture always carries - the project it was asked in and the account it
/// was asked from.
fn LabelsOf(Meta:&Meta) -> Vec<String> {
	let mut Labels:Vec<String> = Meta.Chrome.clone();
	for Label in [&Meta.Project, &Meta.Account] {
		let Label = Label.trim();
		if Label.is_empty() || Labels.iter().any(|Known| Known.eq_ignore_ascii_case(Label)) {
			continue;
		}
		Labels.push(Label.to_string());
	}
	Labels.sort_by_key(|Label| std::cmp::Reverse(Label.chars().count()));
	Labels
}

/// Whether the line is the page's furniture rather than the answer's text.
fn Furniture(Line:&str, Labels:&[String]) -> bool {
	let Trimmed = Line.trim();
	if Trimmed.is_empty() {
		return false;
	}
	let Rest = Unlabelled(Trimmed, Labels);
	if Rest.is_empty() {
		return true;
	}
	let mut Counted = false;
	let mut Unit = false;
	for Token in Rest.split_whitespace() {
		let Word = Token
			.trim_matches(|Character:char| !Character.is_alphanumeric() && !matches!(Character, ':' | '-' | '.'))
			.to_lowercase();
		if Word.is_empty() {
			// Punctuation the page did not put there as a control.
			return false;
		}
		if Vocabulary.contains(&Word.as_str()) {
			Unit |= matches!(Word.as_str(), "sources" | "source");
			continue;
		}
		if Duration(&Word) || Clock(&Word) {
			Unit = true;
			continue;
		}
		if Word.chars().all(|Character| Character.is_ascii_digit()) {
			Counted = true;
			continue;
		}
		// One word the page does not own is the answer's, and the line is kept.
		return false;
	}
	// A bare number is a count of nothing in particular: `35 sources` is the
	// page's, `42` alone is a line somebody wrote.
	!Counted || Unit
}

/// The line with any leading known label taken off it.
fn Unlabelled<'Line>(Line:&'Line str, Labels:&[String]) -> &'Line str {
	let mut Rest = Line.trim();
	'Strip: loop {
		for Label in Labels {
			let Label = Label.trim();
			let Some(Head) = Rest.get(..Label.len()) else {
				continue;
			};
			if Label.is_empty() || !Head.eq_ignore_ascii_case(Label) {
				continue;
			}
			let After = &Rest[Label.len()..];
			// A label ends at a word boundary: `DELO` does not open `DELOS`.
			if !After.is_empty() && !After.starts_with(char::is_whitespace) {
				continue;
			}
			Rest = After.trim_start();
			continue 'Strip;
		}
		return Rest;
	}
}

/// Whether the token is an elapsed time: `3s`, `12s`, `1.5s`, `250ms`.
fn Duration(Token:&str) -> bool {
	let Digits = Token.trim_end_matches(|Character:char| Character.is_ascii_alphabetic());
	if Digits.is_empty() || Digits.len() == Token.len() || Digits.parse::<f64>().is_err() {
		return false;
	}
	matches!(&Token[Digits.len()..], "ms" | "s" | "m" | "h")
}

/// Whether the token is a time of day: `1:42`, `07:02`.
fn Clock(Token:&str) -> bool {
	let Some((Hour, Minute)) = Token.split_once(':') else {
		return false;
	};
	!Hour.is_empty()
		&& Hour.len() <= 2
		&& Hour.chars().all(|Character| Character.is_ascii_digit())
		&& Minute.len() == 2
		&& Minute.chars().all(|Character| Character.is_ascii_digit())
}

/// The note's title: one line, and never empty.
fn Title(Meta:&Meta) -> String {
	let Folded = Meta.Title.split_whitespace().collect::<Vec<&str>>().join(" ");
	if Folded.is_empty() {
		// A note must have a heading, and the thread is the one fact that is
		// always there. An empty heading renders as a blank line, which a
		// reader takes for a mistake in the renderer rather than a note that
		// arrived without a title.
		return Meta.Thread.trim().to_string();
	}
	Folded
}

/// The date part of a capture timestamp, when it has one.
///
/// The catalogue greps `date`, and a note is filed by the day it was captured,
/// so the key is emitted when the timestamp is a date and omitted when it is
/// not - a capture that says `Today 4:18 PM` is not a date, and `date:` with
/// nothing after it teaches a grep nothing.
fn Date(Captured:&str) -> Option<&str> {
	let Trimmed = Captured.trim();
	let Date = Trimmed.get(..10)?;
	let Shape = Date.chars().enumerate().all(|(Index, Character)| {
		match Index {
			4 | 7 => Character == '-',
			_ => Character.is_ascii_digit(),
		}
	});
	Shape.then_some(Date)
}

/// A frontmatter value: plain when a YAML reader would read it back unchanged,
/// quoted when it would not.
fn Scalar(Value:&str) -> String {
	let Special = Value.is_empty()
		|| Value.trim() != Value
		|| Value.contains(": ")
		|| Value.ends_with(':')
		|| Value.contains(" #")
		|| Value.starts_with(['-', '[', '{', '*', '&', '!', '|', '>', '@', '`', '%', '"', '\'']);
	if Special { Quoted(Value) } else { Value.to_string() }
}

/// A value as a YAML double-quoted scalar.
fn Quoted(Value:&str) -> String { format!("\"{}\"", Value.replace('\\', "\\\\").replace('"', "\\\"")) }

/// The table: the same facts, with the labels a person reads.
///
/// Every column is as wide as its widest cell, header included, so a reader's
/// eye lands on the same edge down the table. Width is counted in characters
/// rather than bytes, because that is what the eye is counting.
fn Table(Facts:&[Fact]) -> String {
	let mut Rows:Vec<(&str, String)> = vec![("Field", "Value".to_string())];
	for Fact in Facts {
		if Fact.Key == "title" {
			continue;
		}
		Rows.push((Fact.Label, Cell(Fact)));
	}
	let Label = Rows.iter().map(|(Label, _)| Label.chars().count()).max().unwrap_or(0);
	let Value = Rows.iter().map(|(_, Value)| Value.chars().count()).max().unwrap_or(0);
	let mut Out = String::new();
	for (Index, (Name, Text)) in Rows.iter().enumerate() {
		if Index == 1 {
			Out.push_str(&format!("| {} | {} |\n", "-".repeat(Label), "-".repeat(Value)));
		}
		Out.push_str(&format!("| {} | {} |\n", Pad(Name, Label), Pad(Text, Value)));
	}
	Out
}

/// One cell: a value as a table can hold it.
fn Cell(Fact:&Fact) -> String {
	let Text = Fact.Value.replace('|', "\\|");
	if Fact.Code { format!("`{Text}`") } else { Text }
}

/// A cell padded to the column's width.
fn Pad(Text:&str, Width:usize) -> String { format!("{Text}{}", " ".repeat(Width.saturating_sub(Text.chars().count()))) }

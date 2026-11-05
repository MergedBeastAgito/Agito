//! The note as a value: what a capture becomes, and the bytes it becomes.
//!
//! `Fixture/Capture.md` is a page rather than an answer - a sidebar, an account
//! label, a mode switch, the project and a clock, an elapsed time, a source
//! count, a row of follow-up prompts - and `Fixture/Note.md` is what is left of
//! it. The pair is checked in rather than generated, so it is a baseline: a
//! change to the render that is not a deliberate change to the note fails here
//! instead of quietly rewriting every capture the workspace will ever take.
//!
//! The rest of the suite is the claims the pair cannot make on its own: that
//! the vocabulary is a whole-line rule rather than a substring one, that a note
//! with no sources says `0` instead of saying nothing, that the table and the
//! frontmatter state the same facts, and that the block is the one the archived
//! notes already carry.

use AgenticChatDriver::{Answer, Build, DefaultChrome, Meta, Note, Render, Source, Step, Strip};

const Capture:&str = include_str!("Fixture/Capture.md");
const Expected:&str = include_str!("Fixture/Note.md");

fn Meta() -> Meta {
	Meta::New(
		"1f0c3d9a-6b1e-4a52-9d3e-2f7a1c8b4e60",
		"Example Project",
		"https://www.example.com/search/1f0c3d9a-6b1e-4a52-9d3e-2f7a1c8b4e60",
		"2026-11-05T01:42:00",
	)
	.Titled("A capture as a note, not a folder")
	.Account("Alex Reader")
}

fn Answer() -> Answer {
	Answer {
		Text:Capture.to_string(),
		Trace:(0..3)
			.map(|Depth| Step { Depth, Label:format!("Step {Depth}"), Detail:"1s".to_string() })
			.collect(),
		Sources:(1..=35)
			.map(|Index| Source { Url:format!("https://example.com/{Index}"), Label:"example".to_string() })
			.collect(),
		Model:"example-model".to_string(),
		Mode:"search".to_string(),
	}
}

/// The frontmatter block of a rendered note, as its lines.
fn Block(Rendered:&str) -> Vec<&str> {
	let Body = Rendered.strip_prefix("---\n").expect("a note opens with its frontmatter");
	let End = Body.find("\n---\n").expect("the frontmatter block is closed");
	Body[..End].lines().collect()
}

/// The keys of a frontmatter block, in the order they are stated.
fn Keys(Rendered:&str) -> Vec<String> {
	Block(Rendered)
		.iter()
		.map(|Line| Line.split(':').next().unwrap_or_default().trim().to_string())
		.collect()
}

/// The rows of a note's table, header included.
fn Rows(Rendered:&str) -> Vec<&str> {
	Rendered
		.lines()
		.skip_while(|Line| !Line.starts_with("| Field"))
		.take_while(|Line| Line.starts_with('|'))
		.collect()
}

/// The note renders to the bytes that were checked in.
#[test]
fn the_capture_renders_byte_for_byte_to_the_note_that_was_checked_in() {
	assert_eq!(Render(&Build(Answer(), Meta())), Expected);
}

/// The page is gone, and a sentence that merely says `Computer` is not.
///
/// The two halves matter equally. A renderer that stripped the word rather than
/// the line would pass the first assertion and delete the answer, which is why
/// the third paragraph of the fixture is the one about `Computer mode`.
#[test]
fn the_page_is_gone_and_a_sentence_that_says_computer_stays() {
	let Note = Build(Answer(), Meta());
	let Transcript = Note.Transcript.clone();
	for Label in [
		"New",
		"Computer",
		"Automations",
		"Artifacts",
		"Plugins",
		"Projects",
		"Recents",
		"Search",
	] {
		assert!(
			!Transcript.lines().any(|Line| Line.trim().eq_ignore_ascii_case(Label)),
			"{Label} is a control, and it is still a line of the transcript"
		);
	}
	for Gone in [
		"Alex Reader",
		"1:42 AM",
		"Ask a follow-up",
		"35 sources",
		"Trace the failure modes",
		"Links",
	] {
		assert!(!Transcript.contains(Gone), "{Gone:?} is page chrome and it survived the strip");
	}
	assert!(
		Transcript.contains("Computer mode is a different capability"),
		"the answer's own sentence about Computer mode was deleted with the page's control"
	);

	let Rendered = Render(&Note);
	assert!(!Rendered.contains("Alex Reader"), "the account label reached the note");
	assert!(!Rendered.contains("1:42 AM"), "the clock reached the note");
}

/// A note that cited nothing says so.
///
/// The two claims are different and only one of them is true of a note: an
/// answer that cited nothing is a fact about the answer, while an absent key is
/// a fact about the renderer.
#[test]
fn a_note_with_no_sources_states_zero_rather_than_omitting_the_key() {
	let mut Answer = Answer();
	Answer.Sources.clear();
	let Note = Build(Answer, Meta());
	assert_eq!(Note.Sources, 0);
	let Rendered = Render(&Note);
	assert!(Rendered.contains("\nsources: 0\n"), "the frontmatter omits the count");
	assert!(
		Rows(&Rendered).iter().any(|Row| Row.starts_with("| Cited sources | 0 ")),
		"the table omits the count"
	);
}

/// The keys are lowercase, and their order is the order a reader greps them in.
#[test]
fn the_frontmatter_keys_are_lowercase_and_in_a_stable_order() {
	let Rendered = Render(&Build(Answer(), Meta()));
	let Keys = Keys(&Rendered);
	assert_eq!(
		Keys,
		vec![
			"title",
			"thread",
			"project",
			"kind",
			"date",
			"status",
			"source",
			"captured",
			"segments",
			"steps",
			"sources",
			"characters"
		]
	);
	for Key in &Keys {
		assert_eq!(Key.to_lowercase(), *Key, "{Key} is not lowercase, and a reader greps it");
	}
}

/// The table states the same facts as the frontmatter, from one list.
#[test]
fn the_table_states_every_fact_the_frontmatter_states() {
	let Note = Build(Answer(), Meta());
	let Rendered = Render(&Note);
	let Block = Block(&Rendered);
	let Rows = Rows(&Rendered);
	for Fact in Note.Facts() {
		if Fact.Key == "title" {
			// The heading is the title's statement in the body: a table row
			// repeating the heading above it reads as two titles.
			assert!(Rendered.contains(&format!("# {}\n", Note.Title)));
			continue;
		}
		let Line = Block
			.iter()
			.find(|Line| Line.starts_with(&format!("{}: ", Fact.Key)))
			.unwrap_or_else(|| panic!("{} is not in the frontmatter", Fact.Key));
		assert!(
			Line.ends_with(&Fact.Value),
			"{} states {:?} in the frontmatter",
			Fact.Key,
			Fact.Value
		);
		let Row = Rows
			.iter()
			.find(|Row| Row.starts_with(&format!("| {} ", Fact.Label)))
			.unwrap_or_else(|| panic!("{} is not a row of the table", Fact.Label));
		assert!(Row.contains(&Fact.Value), "{} states {:?} in the table", Fact.Key, Fact.Value);
	}
}

/// The block is the archived notes' block, with the two deliberate differences.
///
/// `collapsed` is gone because the port refuses a collapsed answer rather than
/// noting it, so the key had one value for every note that could exist; and
/// `characters` is stated because a note is compared with another note by how
/// much it holds. Everything else - the keys, their order, the heading, the
/// table and the transcript under its own heading - is what the workspace's
/// research directory already holds, and this is the assertion that keeps it
/// that way.
#[test]
fn the_block_is_the_archived_notes_block_with_the_two_deliberate_differences() {
	let Path = concat!(env!("CARGO_MANIFEST_DIR"), "/../../.hermes/research/4ffde6d5.md");
	let Archived = std::fs::read_to_string(Path)
		.unwrap_or_else(|Why| panic!("the archived note this render is modelled on could not be read: {Why}"));
	let Rendered = Render(&Build(Answer(), Meta()));
	let Wanted:Vec<String> = Keys(&Archived)
		.into_iter()
		.filter(|Key| Key != "collapsed")
		.chain(std::iter::once("characters".to_string()))
		.collect();
	assert_eq!(
		Keys(&Rendered),
		Wanted,
		"the note's keys are no longer the archived notes' keys"
	);
	for Section in ["\n# ", "\n| Field", "\n## Transcript\n"] {
		assert!(Archived.contains(Section), "the archived note no longer has {Section:?}");
		assert!(Rendered.contains(Section), "the render no longer has {Section:?}");
	}
}

/// A title that would change the meaning of the block is quoted, and escaped.
#[test]
fn a_title_that_would_change_the_block_is_quoted() {
	let Note = Build(Answer(), Meta().Titled("Why: a \"quoted\" title"));
	assert!(Render(&Note).contains("title: \"Why: a \\\"quoted\\\" title\"\n"));
}

/// A capture with no title is titled by its thread, because a note has a
/// heading and the thread is the fact that is always there.
#[test]
fn a_capture_without_a_title_is_titled_by_its_thread() {
	let Meta = Meta::New(
		"1f0c3d9a-6b1e-4a52-9d3e-2f7a1c8b4e60",
		"Example Project",
		"https://example.com/",
		"",
	);
	let Note = Build(Answer(), Meta);
	assert_eq!(Note.Title, "1f0c3d9a-6b1e-4a52-9d3e-2f7a1c8b4e60");
	assert!(Render(&Note).contains("\n# 1f0c3d9a-6b1e-4a52-9d3e-2f7a1c8b4e60\n"));
	// A timestamp that is not a date states no date, rather than an empty one.
	assert!(!Render(&Note).contains("date:"));
}

/// The rule is the whole line, and the vocabulary is the caller's to replace.
#[test]
fn a_line_is_furniture_only_when_every_word_on_it_is_the_pages() {
	let Chrome = DefaultChrome();
	assert_eq!(Strip("New", &Chrome), "");
	assert_eq!(Strip("New York is a city.", &Chrome), "New York is a city.");
	assert_eq!(Strip("Computer", &Chrome), "");
	assert_eq!(Strip("The Computer mode runs tools.", &Chrome), "The Computer mode runs tools.");
	assert_eq!(Strip("1:42 AM", &Chrome), "");
	assert_eq!(Strip("3s", &Chrome), "");
	assert_eq!(Strip("35 sources", &Chrome), "");
	assert_eq!(Strip("42", &Chrome), "42");
	// A caller reading a product whose sidebar says something else passes its
	// own words rather than editing the crate.
	let Own = vec!["Sidebar".to_string()];
	assert_eq!(Strip("Sidebar", &Own), "");
	assert_eq!(Strip("Sidebar", &Chrome), "Sidebar");
	// Prose with no furniture on it is the same prose: the strip is not a
	// normaliser that rewrites what it keeps.
	let Prose = "A line the answer wrote.\n\nAnother paragraph.";
	assert_eq!(Strip(Prose, &Chrome), Prose);
}

/// The counts are the answer's, and the transcript is not counted twice.
#[test]
fn the_counts_come_from_the_answer_and_the_characters_from_the_transcript() {
	let Note:Note = Build(Answer(), Meta());
	assert_eq!(Note.Steps, 3);
	assert_eq!(Note.Sources, 35);
	assert_eq!(Note.Segments, 1);
	assert_eq!(Note.Characters, Note.Transcript.chars().count());
	assert!(!Note.Transcript.contains('\r'));
	assert!(!Note.Transcript.ends_with('\n'));
}

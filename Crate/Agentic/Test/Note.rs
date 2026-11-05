//! The note as the command line produces it: which facts it states, where each
//! one came from, and what it refuses to invent.
//!
//! The renderer itself is `agentic-chat-driver`'s, and its own suite is where
//! the layout is pinned byte for byte. What is checked here is the composition
//! root's half - the half that decides whether a fact is the caller's, the
//! page's, or nobody's. So every case is about provenance: a thread read out of
//! a URL, a project that is empty because nobody named one, an account label
//! used to strip a line and never written down.
//!
//! No browser and no clock: the answer is built here and the timestamp is
//! supplied, which is what makes "the note says 2026-11-05" a statement about
//! the renderer rather than about the day this ran.

use Agentic::{
	Command,
	Fn::Note::{Now, Unfiled},
	Given,
	Note,
	Shape,
	ThreadOf,
	Verb,
};
use AgenticChatDriver::{Answer, Source, Step};

/// The thread the cases are filed under.
const ThreadId:&str = "1f0c3d9a-6b1e-4a52-9d3e-2f7a1c8b4e60";

/// A thread URL, as a page reports it: the identifier and a query the page put
/// there.
fn Url() -> String { format!("https://www.example.com/search/{ThreadId}?sm=d") }

/// A settled answer: a page's furniture, an answer, and a formula the page
/// wrote as TeX.
///
/// The blank lines are the page's and they matter: the row of follow-up chips
/// is recognised by the prompt that closes it, and the row is the block of
/// lines that ends at it - so a capture with no blank line before the chips
/// loses the paragraph above them. This fixture is the shape the page produces.
fn Answer() -> Answer {
	Answer {
		Url:Url(),
		Text:[
			"New",
			"Search",
			"Alex Reader 1:42 AM",
			"Example Project 1:42 AM",
			"",
			"How is a quadratic solved?",
			"",
			"The closed form is $x = \\frac{-b \\pm \\sqrt{b^2 - 4ac}}{2a}$, which needs no",
			"iteration.",
			"",
			"Computer mode is a different capability.",
			"",
			"3s",
			"35 sources",
			"Links",
			"",
			"Trace the failure modes when CDP execution races",
			"Ask a follow-up",
		]
		.join("\n"),
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

/// The frontmatter of a rendered note, as its lines.
fn Block(Rendered:&str) -> Vec<&str> {
	let Body = Rendered.strip_prefix("---\n").expect("a note opens with its frontmatter");
	let End = Body.find("\n---\n").expect("the frontmatter block is closed");
	Body[..End].lines().collect()
}

/// The keys of a note's frontmatter, in the order they are stated.
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

/// A note built from the answer with the given flags.
fn Rendered(Given:Given) -> String { Note(&Answer(), Given).expect("the note is written") }

/// The frontmatter states all eleven facts, in the order a reader greps them.
///
/// Eleven is the count the renderer is allowed and not a count to keep in step:
/// a twelfth key is a field nobody reads, and a missing one is a fact the note
/// silently dropped.
#[test]
fn a_note_states_every_fact_in_its_frontmatter_and_in_its_table() {
	let Note = Rendered(Given {
		Title:Some("A note from the command line".to_string()),
		Project:Some("Example Project".to_string()),
		Captured:Some("2026-11-05T01:42:00".to_string()),
		..Given::default()
	});
	assert!(Note.starts_with("---\n"), "the first line of the file is not `---`:\n{Note}");
	assert_eq!(
		Keys(&Note),
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
			"characters",
		],
		"the frontmatter does not state the eleven facts"
	);
	// The table is the same facts under the labels a person reads, so a note
	// cannot state one and drop the other.
	for Label in [
		"Thread",
		"Project",
		"Kind",
		"Status",
		"Source",
		"Captured",
		"Segments",
		"Trace steps",
		"Cited sources",
		"Characters",
	] {
		assert!(
			Rows(&Note).iter().any(|Row| Row.starts_with(&format!("| {Label}"))),
			"{Label} is in the frontmatter and missing from the table"
		);
	}
	assert!(Note.contains("\n## Transcript\n\n"), "the transcript has no heading");
}

/// The thread is the caller's, or the one the page's own URL names.
#[test]
fn the_thread_is_the_caller_s_or_the_one_in_the_page_s_url() {
	let Derived = Rendered(Given { Captured:Some("2026-11-05T01:42:00".to_string()), ..Given::default() });
	assert!(
		Derived.contains(&format!("thread: {ThreadId}\n")),
		"the thread in the page's URL was not the one the note was filed under:\n{Derived}"
	);
	let Named = Rendered(Given {
		Thread:Some("a-thread-the-caller-named".to_string()),
		Captured:Some("2026-11-05T01:42:00".to_string()),
		..Given::default()
	});
	assert!(
		Named.contains("thread: a-thread-the-caller-named\n"),
		"the flag lost to the page"
	);
}

/// A page whose URL names no thread is refused, and the refusal names the flag.
#[test]
fn a_page_url_with_no_thread_is_refused_rather_than_filed_under_a_guess() {
	let Answer = Answer { Url:"https://example.test/".to_string(), ..Answer() };
	let Why = Note(&Answer, Given::default()).expect_err("a note with no thread was written");
	assert_eq!(Why, Unfiled("https://example.test/"));
	assert!(Why.contains("--thread"), "the refusal does not say what to do: {Why}");
	// A bare host is not a thread, and neither is a path whose last segment is
	// a word: taking it would file the note under `intro`.
	assert_eq!(ThreadOf("https://example.test/"), None);
	assert_eq!(ThreadOf("https://example.test/docs/intro"), None);
	assert_eq!(ThreadOf(&Url()).as_deref(), Some(ThreadId));
	assert_eq!(
		ThreadOf(&format!("https://example.test/search/{ThreadId}/")).as_deref(),
		Some(ThreadId)
	);
	assert_eq!(ThreadOf("https://example.test/search/"), None);
}

/// A project nobody named is stated as empty rather than invented.
#[test]
fn an_unnamed_project_is_empty_rather_than_guessed() {
	let Note = Rendered(Given { Captured:Some("2026-11-05T01:42:00".to_string()), ..Given::default() });
	assert!(
		Note.contains("project: \"\"\n"),
		"an unnamed project was not stated as empty:\n{Note}"
	);
	assert!(!Note.contains("Example Project\n"), "a project was invented");
}

/// The account label strips its line and is never written down.
#[test]
fn the_account_label_strips_its_line_and_never_reaches_the_note() {
	let Note = Rendered(Given {
		Project:Some("Example Project".to_string()),
		Account:Some("Alex Reader".to_string()),
		Captured:Some("2026-11-05T01:42:00".to_string()),
		..Given::default()
	});
	assert!(!Note.contains("Alex Reader"), "the account label reached the note:\n{Note}");
	assert!(!Note.contains("1:42 AM"), "the clock reached the note:\n{Note}");
	assert!(
		!Note.lines().any(|Line| Line.trim() == "Search"),
		"a control survived the strip"
	);
	// The answer's own sentence about `Computer` mode is not a control, and a
	// renderer that deleted the word rather than the line would lose it.
	assert!(
		Note.contains("Computer mode is a different capability."),
		"the answer lost a sentence"
	);
	assert!(
		Note.contains("$x = \\frac{-b \\pm \\sqrt{b^2 - 4ac}}{2a}$"),
		"the answer lost its TeX"
	);
}

/// The timestamp is the caller's when it is given, and the date is derived.
#[test]
fn a_captured_stamp_is_the_caller_s_and_the_date_comes_from_it() {
	let Note = Rendered(Given { Captured:Some("2026-11-05T01:42:00".to_string()), ..Given::default() });
	assert!(
		Note.contains("captured: 2026-11-05T01:42:00\n"),
		"the caller's stamp was replaced"
	);
	assert!(Note.contains("date: 2026-11-05\n"), "the date was not derived from the stamp");
	// A stamp that is not a date states no date at all, rather than a `date:`
	// with nothing after it that teaches a grep nothing.
	let Undated = Rendered(Given { Captured:Some("Today 4:18 PM".to_string()), ..Given::default() });
	assert!(!Undated.contains("date:"), "a stamp that is not a date produced a date");
}

/// The clock this process reads is a stamp a note can be filed by.
#[test]
fn now_is_a_stamp_the_note_can_be_filed_by() {
	let Now = Now();
	assert_eq!(Now.len(), 19, "a capture stamp is not `YYYY-MM-DDTHH:MM:SS`: {Now}");
	let Shape:Vec<&str> = Now.split(['-', 'T', ':']).collect();
	assert_eq!(Shape.len(), 6, "a capture stamp is not `YYYY-MM-DDTHH:MM:SS`: {Now}");
	for (Index, Part) in Shape.iter().enumerate() {
		assert_eq!(Part.len(), if Index == 0 { 4 } else { 2 }, "{Now} is not a two-digit stamp");
		assert!(Part.chars().all(|Character| Character.is_ascii_digit()), "{Now} is not a stamp");
	}
}

/// The shapes are the three the flag takes, and only the note changes the
/// reading.
#[test]
fn the_shapes_are_text_markdown_and_note_and_note_reads_markdown() {
	assert_eq!(Shape::All().map(Shape::Name), ["text", "markdown", "note"]);
	assert_eq!(Shape::From("note"), Some(Shape::Note));
	assert_eq!(Shape::From(" Note "), Some(Shape::Note));
	assert_eq!(Shape::From("notebook"), None);
	// A reading format is a shape too, which is what a selector file states.
	assert_eq!(Shape::Of(AgenticChatDriver::Format::Markdown), Shape::Markdown);
	assert_eq!(Shape::Of(AgenticChatDriver::Format::Text), Shape::Text);
	// `note` is a document built from Markdown, and Markdown is unchanged.
	assert_eq!(Shape::Note.Reading(), AgenticChatDriver::Format::Markdown);
	assert_eq!(Shape::Markdown.Reading(), AgenticChatDriver::Format::Markdown);
	assert_eq!(Shape::Text.Reading(), AgenticChatDriver::Format::Text);
}

/// The command line offers the shape and every fact a note needs.
#[test]
fn the_command_line_offers_the_note_shape_and_the_facts_it_takes() {
	let Help = Command::ParseFrom(vec!["agentic".to_string(), "chat".to_string(), "--help".to_string()])
		.expect_err("--help is not a run")
		.to_string();
	for Word in ["note", "--thread", "--title", "--project", "--account", "--captured"] {
		assert!(Help.contains(Word), "{Word} is not in `agentic chat --help`:\n{Help}");
	}
	let Parsed = Command::ParseFrom(
		[
			"agentic",
			"chat",
			"--url",
			"https://example.test/search/x",
			"--model",
			"sol",
			"--question",
			"why",
			"--format",
			"note",
			"--thread",
			"t",
			"--title",
			"A title",
			"--project",
			"P",
			"--account",
			"A",
			"--captured",
			"2026-11-05T01:42:00",
		]
		.iter()
		.map(|Argument| Argument.to_string())
		.collect(),
	)
	.expect("a note call parses");
	let Verb::Chat { Format, Thread, Title, Project, Account, Captured, .. } = Parsed.Verb else {
		panic!("a chat call parsed as another verb");
	};
	assert_eq!(Format.as_deref(), Some("note"));
	assert_eq!(Thread.as_deref(), Some("t"));
	assert_eq!(Title.as_deref(), Some("A title"));
	assert_eq!(Project.as_deref(), Some("P"));
	assert_eq!(Account.as_deref(), Some("A"));
	assert_eq!(Captured.as_deref(), Some("2026-11-05T01:42:00"));
}

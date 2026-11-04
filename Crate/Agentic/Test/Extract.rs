//! The Markdown reader, exercised against a real page in a real browser.
//!
//! Ignored by default, and deliberately: the rest of this workspace's tests run
//! on a machine with no Chromium at all, and a browser-backed assertion in the
//! default suite would turn "this machine has no Brave" into "the reader is
//! broken". Run it when the reader changes:
//!
//! ```text
//! cargo test -p agentic --test Extract -- --ignored --nocapture
//! ```
//!
//! What it proves is the part a fake cannot: that the expression the driver
//! sends, evaluated in a page that renders a table, a fence, a formula and a
//! nested list, produces Markdown with those constructs in it. The fixture is
//! `Crate/ChatDriver/Test/Fixture/Answer.html` - the same file a person can open
//! to see what was read.
//!
//! The browser is named by `AGENTIC_BROWSER`, because this workspace does not
//! assume which one a machine has installed. When the variable is absent the
//! well-known Brave paths are tried, and a machine with neither is a failure
//! that says so rather than a test that quietly passes.

use std::{path::PathBuf, sync::Arc, time::Duration};

use AgenticBrowserCDP::{
	Browse, Chrome, ChromeOptions, Create as CreateSession, OpenOptions, Options, Report, Tokio,
};
use AgenticChatDriver::{DefaultSelectors, Format, ReadAnswers};
use AgenticRuntime::{Call, Never};

/// A call with a deadline, on the browser's own clock.
fn Budget(Seconds:u64) -> Call {
	Call::Within(Duration::from_secs(Seconds), &Tokio, Arc::new(Never))
}

/// The browser binary to drive.
fn Browser() -> PathBuf {
	if let Some(Named) = std::env::var_os("AGENTIC_BROWSER") {
		return PathBuf::from(Named);
	}
	let Candidates = [
		"/Applications/Brave Browser.app/Contents/MacOS/Brave Browser",
		"/Applications/Brave Browser Nightly.app/Contents/MacOS/Brave Browser Nightly",
		"/Applications/Google Chrome.app/Contents/MacOS/Google Chrome",
	];
	for Candidate in Candidates {
		let Path = PathBuf::from(Candidate);
		if Path.is_file() {
			return Path;
		}
	}
	panic!(
		"no browser to drive: set AGENTIC_BROWSER to one of {}",
		Candidates.join(", ")
	);
}

/// The fixture, as a `file:` URL the browser can open.
fn Fixture() -> String {
	let Path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
		.join("../ChatDriver/Test/Fixture/Answer.html");
	let Url = format!("file://{}", Path.to_string_lossy());
	assert!(Path.is_file(), "the fixture is not at {}", Path.display());
	Url
}

/// The answer's text in one format, read from the fixture page.
fn Captured(Format:Format) -> String {
	let Runtime = tokio::runtime::Builder::new_multi_thread()
		.enable_all()
		.build()
		.expect("a runtime");
	let Report:Report = Arc::new(|_Target, Why| eprintln!("dispose failed: {}", Why.Message()));
	let Browse = CreateSession(
		Chrome::New(ChromeOptions { Executable:Some(Browser()), Headless:true, NoSandbox:false }),
		Arc::new(Tokio),
		Options { Report, ..Options::default() },
	);
	let Selectors = DefaultSelectors();
	let Expression = ReadAnswers(&Selectors, Format);
	Runtime.block_on(async move {
		let Target = Browse
			.Open(&Fixture(), OpenOptions::Launch(Budget(30)))
			.await
			.expect("the fixture opens");
		let Value = Browse
			.Evaluate(&Target, &Expression, Budget(30))
			.await
			.expect("the expression runs");
		Browse.Dispose(Target).await;
		Value
			.get("text")
			.and_then(serde_json::Value::as_str)
			.unwrap_or_default()
			.to_string()
	})
}

#[test]
#[ignore = "needs a browser: cargo test -p agentic --test Extract -- --ignored"]
fn the_fixture_page_round_trips_as_markdown() {
	let Text = Captured(Format::Markdown);

	// A table: its head, a separator, and one line per row.
	let Rows:Vec<&str> = Text.lines().filter(|Line| Line.starts_with("| ")).collect();
	assert_eq!(Rows.len(), 5, "a head, a separator and three rows:\n{Text}");
	assert_eq!(Rows[0], "| Entity | Definition | Example |");
	assert_eq!(Rows[1], "| --- | --- | --- |");
	assert!(Rows[4].contains("a \\| b"), "a pipe in a cell is escaped:\n{Text}");

	// A fence, with the language the code declares.
	assert!(Text.contains("```rust\npub fn Read"), "a fenced block keeps its language:\n{Text}");
	// A fence that outgrows a fence inside it.
	assert!(Text.contains("````markdown\n```rust\nlet x = 1;\n```\n````"), "the fence grew:\n{Text}");

	// The formula is its annotation, not its glyphs.
	assert!(Text.contains("$E = mc^2$"), "inline maths:\n{Text}");
	assert!(Text.contains("$$a^2 + b^2 = c^2$$"), "display maths:\n{Text}");
	assert!(!Text.contains("mord"), "the rendered glyph tree is never read:\n{Text}");

	// Structure the text reader loses.
	assert!(Text.contains("- The driver asks exactly once\n  - one question, one send"), "a nested list:\n{Text}");
	assert!(Text.contains("3. Read the [the source](https://example.test/source)"), "an ordered list with a link:\n{Text}");
	assert!(Text.contains("## What the page says"), "a heading:\n{Text}");

	// A citation marker is the number the page showed, not a link.
	assert!(Text.contains("Keep the citation1 as it was shown"), "a citation is not a link:\n{Text}");
	assert!(!Text.contains("](https://example.test/cited)"), "a citation is not a link:\n{Text}");

	// Chrome inside the answer is not prose.
	assert!(!Text.contains("Copy"), "a control is not part of the answer:\n{Text}");
}

#[test]
#[ignore = "needs a browser: cargo test -p agentic --test Extract -- --ignored"]
fn the_text_format_is_the_unformatted_reading_the_card_replaces() {
	let Markdown = Captured(Format::Markdown);
	let Text = Captured(Format::Text);
	assert_ne!(Markdown, Text, "the two formats read the same page differently");
	assert!(!Text.contains("| Entity |"), "innerText keeps no pipes:\n{Text}");
	assert!(!Text.contains("```rust"), "innerText keeps no fence:\n{Text}");
	assert!(!Text.contains("$E = mc^2$"), "innerText keeps no TeX:\n{Text}");
	// Markdown is longer: the structure costs characters the text reader does
	// not spend.
	assert!(
		Markdown.chars().count() > Text.chars().count(),
		"markdown {} vs text {}",
		Markdown.chars().count(),
		Text.chars().count()
	);
}

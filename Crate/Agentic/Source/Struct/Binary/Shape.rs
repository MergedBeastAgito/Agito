//! What `agentic chat` prints, as opposed to how the page is read.
//!
//! One call makes two decisions and they are not the same decision.
//! `agentic-chat-driver`'s `Format` says how the answer's text is taken out of
//! the page - `innerText`, or the DOM serialised as Markdown - and it is a field
//! of the selector file because it is a fact about one product's markup. A shape
//! says what the process writes: the answer as the JSON a caller parses, or the
//! answer as the note a reader opens.
//!
//! `note` is a shape and deliberately not a reading format. A note is not a way
//! to read a page, it is a document built from what was read, so a reading enum
//! that carried it would let a caller ask for "the note reading" of a page that
//! never had one - and the driver, which owns that enum, would then have to know
//! what a note is.
//!
//! The two are combined rather than exclusive, and only in one direction: a note
//! needs the page's TeX, tables and fences, because a note built from
//! `innerText` carries a formula as one glyph per line and a table as a run of
//! tabs. [`Shape::Reading`] states that pairing once, so the flag cannot be
//! added to a call and then quietly read the wrong way.

use AgenticChatDriver::Format;

/// What the answer is printed as.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Shape {
	/// The answer as the JSON the driver returned, read as rendered text.
	Text,
	/// The same JSON, read as the page's own Markdown.
	Markdown,
	/// The research note: frontmatter, the same facts as a table, then the
	/// transcript.
	Note,
}

impl Shape {
	/// The word the command line uses.
	pub const fn Name(self) -> &'static str {
		match self {
			Shape::Text => "text",
			Shape::Markdown => "markdown",
			Shape::Note => "note",
		}
	}

	/// Every shape, in the order the command line lists them.
	pub const fn All() -> [Shape; 3] { [Shape::Text, Shape::Markdown, Shape::Note] }

	/// The shape a word names, or `None` when it names none of them.
	///
	/// Case is folded and nothing else is, for the reason the reading format
	/// folds nothing else: a shape that is nearly right is a caller's mistake,
	/// and a silent fallback would write a document they did not ask for.
	pub fn From(Word:&str) -> Option<Self> {
		let Word = Word.trim().to_lowercase();
		Self::All().into_iter().find(|Shape| Shape.Name() == Word)
	}

	/// The shape a reading format prints as, for a call that named no shape.
	///
	/// Total, because every reading format has an output: a caller who set
	/// `"format": "markdown"` in a selector file asked for Markdown and gets
	/// the JSON that carries it, which is what the file has always produced.
	pub const fn Of(Format:Format) -> Self {
		match Format {
			Format::Text => Shape::Text,
			Format::Markdown => Shape::Markdown,
		}
	}

	/// How the page must be read for this shape to be honest.
	///
	/// The reading is a consequence of the shape rather than a second choice,
	/// and it overrides the selector file: a file may describe a product, but
	/// it cannot make a note carry glyphs where the answer wrote TeX.
	pub const fn Reading(self) -> Format {
		match self {
			Shape::Text => Format::Text,
			Shape::Markdown | Shape::Note => Format::Markdown,
		}
	}
}

//! How the text of an answer is read out of the page.
//!
//! The page is the same either way; what changes is which of its statements is
//! taken as the answer. `Text` is `innerText`: what the product put on screen,
//! with a table's columns reduced to tabs, a list's structure reduced to line
//! breaks, and a formula reduced to its glyphs. `Markdown` serialises the same
//! DOM through `Source/Script/Expressions/Extract.ts`, so a table keeps its
//! columns, a code block keeps its fence and a formula is its TeX.
//!
//! `Both` is the two of them from one read. Each of the first two is a whole
//! read of the page on its own, so a caller that wants both renderings and
//! asks twice reads the page twice - and the second read can see a page that
//! has changed under it, which is how one answer comes to have two renderings
//! that disagree about what it said. `Both` is a single expression that takes
//! the rendered text and the page's own Markdown from the same subtree in the
//! same evaluate, so the pair describes one moment by construction rather than
//! by agreement between two.
//!
//! `Text` is the default, and deliberately so: the parity scenarios were
//! baselined against it, and a format that silently changed every recorded
//! answer would turn a baseline into a diff nobody asked for. A capture asks
//! for `Markdown`; anything that compares against a recorded text keeps
//! `Text`.
//!
//! It is a field of [`crate::Struct::Selectors::Selectors`] rather than of the
//! driver's options because it is a fact about the page: one product's answer
//! markup is worth serialising and another's is not, and the selector file is
//! where a product's reading is described.

/// How an answer's text is read.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Format {
	/// `innerText`, trimmed: what the page rendered, as text.
	#[default]
	Text,
	/// The DOM serialised as Markdown by the page itself.
	Markdown,
	/// Both readings, from one snapshot of the same answer.
	///
	/// One expression returns `text` - the rendered text - and `markdown` -
	/// the page's own serialisation - and both are taken from the answer the
	/// expression selected, in the same evaluate. Nothing is written down:
	/// the pair is handed to the caller, who decides what to do with it.
	///
	/// The completion rule runs on the text half, so a `Both` call settles
	/// exactly as a `Text` call does, and the turn marks are the text's - a
	/// mark computed from the serialised rendering would make every read of a
	/// thread look new to a driver correlating on text.
	Both,
}

impl Format {
	/// The word the command line and the selector file use.
	pub const fn Name(self) -> &'static str {
		match self {
			Format::Text => "text",
			Format::Markdown => "markdown",
			Format::Both => "both",
		}
	}

	/// Every format, in the order the command line lists them.
	pub const fn All() -> [Format; 3] { [Format::Text, Format::Markdown, Format::Both] }

	/// The format a word names, or `None` when it names none of them.
	///
	/// Case is folded and nothing else is: a format that is nearly right is a
	/// caller's mistake, and a silent fallback would capture a whole thread in
	/// the format they did not ask for.
	pub fn From(Word:&str) -> Option<Self> {
		let Word = Word.trim().to_lowercase();
		Self::All().into_iter().find(|Format| Format.Name() == Word)
	}
}

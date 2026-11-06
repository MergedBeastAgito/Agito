//! The note: what a capture becomes once the page has stopped being a page.
//!
//! A capture arrives as text - the accessibility tree, or the answer's own
//! markdown - and a note is that text after the page's furniture has been taken
//! out of it and the facts it only ever implied have been written down. The
//! type is the point: the formatting used to live in a script beside the
//! repository, so the shape of a note was a fact about that script rather than
//! about this capability, and anything that wanted a note had to run a second
//! program in a second language.
//!
//! Two nouns, because there are two kinds of fact:
//!
//! - [`Note`] is the document. Its fields are the facts a reader can grep, and
//!   the renderer is allowed no thirteenth: a field nobody reads is a field
//!   that drifts.
//! - [`Meta`] is what the caller knows and the page cannot say - which thread
//!   this was, which project asked it, when it was captured, whose page it was.
//!   A fact lives here rather than on [`Note`] when the answer's own text
//!   cannot produce it.
//!
//! The account label is in [`Meta`] and is deliberately not a field of
//! [`Note`]: it is read to recognise a line of the page's furniture and is then
//! dropped. A note that recorded who was signed in is a note that cannot be
//! published, and the label earns its place in the structure by being the only
//! way to strip a line that carries nothing but a name.
//!
//! Two facts are counts rather than copies. The answer's step trace and its
//! citations are evidence, and the caller still holds them in the [`Answer`]
//! this note was built from; a note that reproduced both tables would double
//! the size of every capture to restate what the page already said, so it
//! states how many there were and leaves the rows where they were read.
//!
//! [`Answer`]: crate::Struct::Port::Answer

/// One fact about a note, in the two forms it is read in.
///
/// The frontmatter key and the table label are one entry rather than two lists
/// because the renderer must not be able to state a fact in one place and drop
/// it from the other: a reader greps the key, a person reads the label, and a
/// note where those disagree is a note with two answers.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Fact {
	/// The frontmatter key: lowercase, because a reader greps it.
	pub Key:&'static str,
	/// The label the table shows: a reader's words, because a person reads it.
	pub Label:&'static str,
	/// The value, as text - the same in both places.
	pub Value:String,
	/// Whether the table shows the value as code.
	///
	/// An identifier is code to a reader's eye, and a backtick in the
	/// frontmatter is a character rather than a marker, so the flag exists for
	/// the one form that can carry it.
	pub Code:bool,
}

/// A captured answer as a document.
///
/// Every field is a fact the file states, and the renderer states all of them
/// in both halves of the file. Nothing here is derived at render time, so two
/// renders of one note cannot disagree.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Note {
	/// What the thread is about, in one line.
	///
	/// It is the heading and the `title` key at once, because a note is filed
	/// by what it was about: a filename is the thread id, and the title is the
	/// only thing in the document that says why anybody kept it.
	pub Title:String,
	/// The thread's identifier.
	///
	/// This is the join key between a note and the conversation it came from,
	/// which is why it is stated rather than assumed from the filename: a file
	/// can be renamed, and the thread cannot.
	pub Thread:String,
	/// The project the question was asked inside, as the page named it.
	///
	/// A chat product groups threads into projects, and the grouping is part of
	/// how the answer should be read - the same question in a general project
	/// and in a repository's project are answered against different material.
	pub Project:String,
	/// What kind of document this is.
	///
	/// `research` for a capture; the field exists because the directory holds
	/// more than one kind of note, and a reader that wants only the captures
	/// should be able to ask rather than guess from the shape.
	pub Kind:String,
	/// Where the note is in its life: `captured` for a note nobody has read
	/// yet.
	///
	/// It is the field a reviewer changes, so the note itself records whether
	/// what it holds has been checked rather than leaving that to a filename.
	pub Status:String,
	/// The URL the thread is at.
	///
	/// A note is evidence, and evidence that cannot be traced back to where it
	/// came from is a claim. It is the one field a reader can open.
	pub Source:String,
	/// When the page was read, as the caller's own timestamp.
	///
	/// Text rather than a clock reading, because this crate has no clock: time
	/// arrives from the runtime that supplied the deadline, and a renderer that
	/// reached for the system time would stamp the render rather than the
	/// capture - so a note written a week later would claim to be a week newer.
	pub Captured:String,
	/// How many captures the transcript was assembled from.
	///
	/// A long answer that scrolled or virtualised arrives in pieces, and a note
	/// that joined three of them says so rather than looking like one that
	/// arrived whole.
	pub Segments:usize,
	/// How many steps the answer reported having taken.
	///
	/// The count and not the trace: a reader comparing two answers wants to
	/// know whether one searched the web and the other did not, and that is
	/// visible in a number.
	pub Steps:usize,
	/// How many sources the answer cited.
	///
	/// Stated even when it is zero, because "this answer cited nothing" and
	/// "nobody recorded whether it cited anything" are different claims, and
	/// only one of them is true of a note.
	pub Sources:usize,
	/// How many characters the transcript holds.
	///
	/// Counted in characters rather than bytes: a transcript with an emoji in
	/// it is one character longer, not four, and a reader comparing the size of
	/// two notes is comparing what they read.
	pub Characters:usize,
	/// The answer's own words, with the page's furniture taken out.
	///
	/// This is the document. Every other field is a fact *about* it, which is
	/// why they are counts and labels rather than copies of the text.
	pub Transcript:String,
}

impl Note {
	/// The note's facts, in the order the frontmatter states them.
	///
	/// One list, two renderings. The order is the archived notes' order and it
	/// is stable on purpose: a diff between two notes should be the facts that
	/// changed, not a reordering of keys that did not.
	pub fn Facts(&self) -> Vec<Fact> {
		vec![
			Fact { Key:"title", Label:"Title", Value:self.Title.clone(), Code:false },
			Fact { Key:"thread", Label:"Thread", Value:self.Thread.clone(), Code:true },
			Fact { Key:"project", Label:"Project", Value:self.Project.clone(), Code:false },
			Fact { Key:"kind", Label:"Kind", Value:self.Kind.clone(), Code:false },
			Fact { Key:"status", Label:"Status", Value:self.Status.clone(), Code:false },
			Fact { Key:"source", Label:"Source", Value:self.Source.clone(), Code:false },
			Fact { Key:"captured", Label:"Captured", Value:self.Captured.clone(), Code:false },
			Fact { Key:"segments", Label:"Segments", Value:self.Segments.to_string(), Code:false },
			Fact { Key:"steps", Label:"Trace steps", Value:self.Steps.to_string(), Code:false },
			Fact { Key:"sources", Label:"Cited sources", Value:self.Sources.to_string(), Code:false },
			Fact {
				Key:"characters",
				Label:"Characters",
				Value:self.Characters.to_string(),
				Code:false,
			},
		]
	}
}

/// What the caller knows about a capture, and the page cannot say.
///
/// The split from [`Note`] is the rule the whole module rests on: a field is
/// here when the answer's text cannot produce it. The title, the thread, the
/// project and the moment all arrive from outside the page; the counts and the
/// transcript are read out of it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Meta {
	/// What the thread is about. Empty means "use the thread id", because a
	/// note must have a heading and the thread is the one fact always present.
	pub Title:String,
	/// The thread's identifier, as the caller read it from the URL.
	pub Thread:String,
	/// The project the thread lives in, as the page named it.
	pub Project:String,
	/// What kind of note this is: `research` unless the caller says otherwise.
	pub Kind:String,
	/// Where the note is in its life: `captured` unless the caller says
	/// otherwise.
	pub Status:String,
	/// The URL of the thread.
	pub Source:String,
	/// When the capture was taken, as the caller's own timestamp.
	pub Captured:String,
	/// How many captures the transcript was assembled from.
	pub Segments:usize,
	/// The label the page shows for the signed-in account.
	///
	/// Used to recognise one line of furniture and never written down. It is
	/// here rather than in a constant because a name is not this crate's to
	/// know: a driver that carried one would be a driver that only worked for
	/// one person.
	pub Account:String,
	/// The words this page uses for its own controls.
	///
	/// [`DefaultChrome`] when the caller has nothing better; a caller reading a
	/// product whose sidebar says something else passes its own list rather
	/// than editing this crate.
	pub Chrome:Vec<String>,
}

impl Meta {
	/// The facts a caller almost always has: the thread, the project, the URL
	/// and the moment. Everything else is a default that can be replaced.
	pub fn New(
		Thread:impl Into<String>,
		Project:impl Into<String>,
		Source:impl Into<String>,
		Captured:impl Into<String>,
	) -> Self {
		Self {
			Title:String::new(),
			Thread:Thread.into(),
			Project:Project.into(),
			Kind:"research".to_string(),
			Status:"captured".to_string(),
			Source:Source.into(),
			Captured:Captured.into(),
			Segments:1,
			Account:String::new(),
			Chrome:DefaultChrome(),
		}
	}

	/// The same meta, with a title.
	pub fn Titled(self, Title:impl Into<String>) -> Self { Self { Title:Title.into(), ..self } }

	/// The same meta, with the account label the page shows.
	pub fn Account(self, Account:impl Into<String>) -> Self { Self { Account:Account.into(), ..self } }

	/// The same meta, with a vocabulary of the page's own words.
	pub fn Chrome(self, Chrome:Vec<String>) -> Self { Self { Chrome, ..self } }
}

/// The words a chat product uses for its own furniture, as opposed to the words
/// it uses for an answer.
///
/// A list rather than a pattern, because the rule that reads it is exact: a
/// line is furniture when *every* word on it is one of these. That is what
/// makes the list safe to be imperfect - a word missing here costs a line of
/// chrome in the note, and a word added carelessly costs a line of the answer -
/// and it is why nothing here is a substring match. An answer may use the word
/// `Computer` in a sentence, and a renderer that deleted every occurrence would
/// be deleting the answer.
///
/// This is the one place the crate names a product's interface. A caller whose
/// product says something else passes its own list to `Meta::Chrome` instead of
/// editing this one.
pub const Vocabulary:&[&str] = &[
	// The sidebar.
	"new",
	"computer",
	"automations",
	"artifacts",
	"plugins",
	"projects",
	"recents",
	// The mode switch.
	"search",
	"research",
	"labs",
	// The controls around an answer.
	"share",
	"finished",
	"copy",
	"edit",
	"retry",
	"show",
	"hide",
	"more",
	"less",
	"expand",
	"collapse",
	// The footer of an answer.
	"links",
	"images",
	"sources",
	"source",
	"ask",
	"a",
	"follow-up",
	// The parts of a timestamp.
	"today",
	"yesterday",
	"am",
	"pm",
];

/// The vocabulary a capture is stripped with when the caller supplies none.
pub fn DefaultChrome() -> Vec<String> { Vocabulary.iter().map(|Word| (*Word).to_string()).collect() }

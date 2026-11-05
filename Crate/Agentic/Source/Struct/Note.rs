//! What the caller said about a capture, as opposed to what the page said.
//!
//! These are the facts a note states that no answer's text can produce, and
//! they arrive as flags rather than from anything this crate reads: the thread
//! a question was asked in, what it was about, which project asked it, whose
//! page it was, and when it was taken. The page supplies the rest - the
//! transcript, the counts, the URL it answered from.
//!
//! Every field is optional, and empty is not the same as absent: a field that
//! was not given falls back to something the call already knows (the thread in
//! the URL, the moment the answer settled), while a field that was given empty
//! is dropped as if it had not been given at all. Nothing here is defaulted to
//! a plausible value - a note that named a project nobody asked in would be
//! evidence of a conversation that did not happen.

/// The note's facts that the caller supplies.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Given {
	/// The thread the answer belongs to. Absent, it is read from the page's
	/// own URL.
	pub Thread:Option<String>,
	/// What the thread is about. Absent, the note is titled by its thread.
	pub Title:Option<String>,
	/// The project the question was asked inside. Absent, the note states an
	/// empty project rather than naming one.
	pub Project:Option<String>,
	/// The label the page shows for the signed-in account.
	///
	/// Read to recognise one line of the page's furniture and then dropped: a
	/// note that recorded who was signed in is a note that cannot be
	/// published.
	pub Account:Option<String>,
	/// When the capture was taken. Absent, the moment the answer settled is
	/// used, which is the same instant to the second and the only one this
	/// process can honestly claim.
	pub Captured:Option<String>,
}

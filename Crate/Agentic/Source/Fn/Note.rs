//! Assembling the note a `--format note` call prints.
//!
//! `agentic-chat-driver` renders a note from an answer and a `Meta`; this module
//! is the composition root's half of that. The driver decides what the page's
//! furniture is and how the document is laid out; this decides which facts the
//! caller supplied, which ones the page supplied, and the two that neither can:
//! the moment of the capture, and the fallback from a missing title to the
//! thread it was filed under.
//!
//! Nothing here guesses, because a note is evidence. The thread is the caller's
//! `--thread` or the identifier in the page's own URL; the title is the
//! caller's or the thread; the project is the caller's or empty; the source is
//! the URL the answer was read from. A note that invented any of them would be
//! a document about a conversation that did not happen, and it would look
//! exactly like one that did.

use AgenticChatDriver::{Answer, Build, Meta, Render};

use crate::Struct::Note::Given;

/// The note, rendered: the file a `--format note` call writes to stdout.
///
/// The answer supplies the transcript, the trace and the sources; the flags
/// supply everything a page cannot say. The refusal is the one decision that
/// can fail: a capture that cannot be filed under a thread is refused rather
/// than filed under an empty one.
pub fn Note(Answer:&Answer, Given:Given) -> Result<String, String> {
	let Given { Thread, Title, Project, Account, Captured } = Given;
	let Source = Answer.URL.trim().to_string();
	let Thread = match NonEmpty(Thread) {
		Some(Thread) => Thread,
		None => ThreadOf(&Source).ok_or_else(|| Unfiled(&Source))?,
	};
	let Project = NonEmpty(Project).unwrap_or_default();
	let Captured = NonEmpty(Captured).unwrap_or_else(Now);
	let mut Meta = Meta::New(Thread, Project, Source, Captured);
	if let Some(Title) = NonEmpty(Title) {
		Meta = Meta.Titled(Title);
	}
	if let Some(Account) = NonEmpty(Account) {
		Meta = Meta.Account(Account);
	}
	Ok(Render(&Build(Answer.clone(), Meta)))
}

/// The reason a note cannot be written, for a page whose URL names no thread.
///
/// It names the flag rather than only the problem, because the caller is the
/// one who knows which thread this was and the URL is not.
pub fn Unfiled(Source:&str) -> String {
	format!("a note is filed under the thread it came from, and {Source:?} names none: pass --thread <id>")
}

/// The thread identifier a thread URL carries, when it carries one.
///
/// A chat product files a thread under a URL whose path ends in the thread's
/// identifier - `…/search/<id>` - and a query string is the page's own
/// decoration rather than part of the id. This is the one place the composition
/// root names a product's URL shape, which is why anything else returns `None`
/// and `--thread` is how a caller says what its product does instead: a
/// renderer that took the last path segment of any URL would file
/// `https://example.test/docs/intro` under `intro`.
pub fn ThreadOf(URL:&str) -> Option<String> {
	let Path = URL.split(['?', '#']).next().unwrap_or_default();
	let (_, Thread) = Path.split_once("/search/")?;
	let Thread = Thread.trim_end_matches('/');
	(!Thread.is_empty() && !Thread.contains('/')).then(|| Thread.to_string())
}

/// The moment the note is written, in the shape the archived notes carry.
///
/// This is the one wall-clock read in the workspace - a deadline is supplied by
/// the runtime, but a capture's own instant is not a deadline and no port can
/// say it - and it happens in the composition root for that reason.
///
/// It is local time rather than UTC because a note is filed by the day the
/// person captured it, and a UTC reading files a capture taken at 1 a.m. under
/// yesterday. `libc` is here for `localtime_r` and nothing else: which offset a
/// zone carries is the operating system's fact, and a table of them in this
/// crate would be wrong the first time somebody travelled.
pub fn Now() -> String {
	let Seconds = std::time::SystemTime::now()
		.duration_since(std::time::UNIX_EPOCH)
		.map(|Elapsed| Elapsed.as_secs() as i64)
		.unwrap_or_default();
	#[cfg(unix)]
	{
		let Moment = Seconds as libc::time_t;
		let mut Broken = std::mem::MaybeUninit::<libc::tm>::uninit();
		// SAFETY: `localtime_r` writes one `tm` through the pointer it is
		// given and touches nothing else; the pointer is to a live, writable,
		// correctly sized `MaybeUninit<tm>`, and the value is read only after
		// the call reported success by returning that same pointer.
		let Filled = unsafe { libc::localtime_r(&Moment, Broken.as_mut_ptr()) };
		if !Filled.is_null() {
			// SAFETY: `localtime_r` returned this pointer, which is how it
			// says the `tm` it was given has been filled.
			let Broken = unsafe { Broken.assume_init() };
			return format!(
				"{:04}-{:02}-{:02}T{:02}:{:02}:{:02}",
				Broken.tm_year + 1900,
				Broken.tm_mon + 1,
				Broken.tm_mday,
				Broken.tm_hour,
				Broken.tm_min,
				Broken.tm_sec
			);
		}
	}
	Utc(Seconds)
}

/// The same instant in UTC, for a platform with no `localtime_r`.
fn Utc(Seconds:i64) -> String {
	let Days = Seconds.div_euclid(86_400);
	let Rest = Seconds.rem_euclid(86_400);
	let (Year, Month, Day) = Civil(Days);
	format!(
		"{Year:04}-{Month:02}-{Day:02}T{:02}:{:02}:{:02}",
		Rest / 3_600,
		(Rest % 3_600) / 60,
		Rest % 60
	)
}

/// The date a count of days from 1970-01-01 names.
///
/// Howard Hinnant's `civil_from_days`, which is arithmetic rather than a
/// calendar: a date library would be a dependency for one subtraction, and a
/// leap-year rule written by hand would be a bug waiting for 2100.
fn Civil(Days:i64) -> (i64, u32, u32) {
	let Days = Days + 719_468;
	let Era = Days.div_euclid(146_097);
	let Doe = Days.rem_euclid(146_097);
	let Yoe = (Doe - Doe / 1_460 + Doe / 36_524 - Doe / 146_096) / 365;
	let Doy = Doe - (365 * Yoe + Yoe / 4 - Yoe / 100);
	let Mp = (5 * Doy + 2) / 153;
	let Day = (Doy - (153 * Mp + 2) / 5 + 1) as u32;
	let Month = if Mp < 10 { Mp + 3 } else { Mp - 9 } as u32;
	let Year = Yoe + Era * 400 + i64::from(Month <= 2);
	(Year, Month, Day)
}

/// A flag's value, or nothing when it was absent or blank.
///
/// A flag given as the empty string is the same as a flag nobody typed: a shell
/// variable that expanded to nothing must not put an empty thread in a note's
/// frontmatter and look like a renderer that dropped a fact.
fn NonEmpty(Value:Option<String>) -> Option<String> {
	Value.map(|Text| Text.trim().to_string()).filter(|Text| !Text.is_empty())
}

//! The port `agito.chat.ask` promises, written the way its consumer asks for
//! it.
//!
//! `module.json` names four errors - `not-ready`, `wrong-mode`, `wrong-model`
//! and `truncated` - and [`crate::Struct::Error`] is where they live. A caller
//! narrows on the variant; anything a page does that is not one of those four
//! is mapped onto one of them at the boundary, so a consumer never has to guess
//! what a failure meant.
//!
//! The port states more than data shapes, because two drivers that agreed on
//! the shapes alone would still behave differently:
//!
//! - **A question is answered or it is not.** `Ask` returns an answer whose
//!   text the page had stopped changing, or it fails. A partial answer is never
//!   returned as a whole one, and an empty string is never an answer.
//! - **Mode and model are preconditions.** The composer is in search mode and
//!   set to the requested model before anything is typed, so a wrong answer is
//!   refused rather than produced.
//! - **The answer is the one this question caused.** A thread already holds the
//!   answers to earlier questions, so the turn is correlated by counting them
//!   before the send and reading only the one that appeared after it.
//! - **A collapsed answer is refused.** When the page hides the rest of an
//!   answer behind a control, the text in the DOM is not the answer, and
//!   returning it would be a quiet lie.
//! - **Cancellation is `not-ready`.** A signal that fires and a deadline that
//!   passes are the same event to a consumer - the answer did not arrive - so
//!   both are `not-ready`. `detail` says which one happened.

use ::RunTime::{Call, FutureBox};
use serde_json::Value;

/// One step of the work an answer reports having done.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Step {
	/// Nesting: a step inside a step is one level deeper than it.
	pub Depth:i64,
	/// What the step is called, as the page labels it.
	pub Label:String,
	/// The page's own note about it - a duration, a state - when it has one.
	pub Detail:String,
}

/// One cited source behind an answer.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Source {
	/// Where the citation points.
	pub URL:String,
	/// How the page labels the citation.
	pub Label:String,
}

/// The composer mode a question must be asked in.
///
/// The port is defined for search only. An agent mode answers by running tools
/// on the caller's behalf, which is a different capability with different costs
/// and a different budget, so a driver that silently used one would be spending
/// somebody else's allowance. [`Mode::Other`] exists so that asking for one is
/// a refusal with a code rather than a value the type system quietly forbids: a
/// caller migrating from another driver learns why from `wrong-mode`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Mode {
	/// The one mode this port asks in.
	Search,
	/// Some other mode, named as the caller named it.
	Other(String),
}

impl Mode {
	/// The mode's name, as a caller would write it.
	pub fn Name(&self) -> &str {
		match self {
			Mode::Search => "search",
			Mode::Other(Name) => Name,
		}
	}

	/// Whether this is the mode the port asks in.
	pub fn IsSearch(&self) -> bool { matches!(self, Mode::Search) }
}

/// What a caller asks for.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Question {
	/// The question, sent as written.
	pub Text:String,
	/// The composer mode the question must be asked in.
	pub Mode:Mode,
	/// The model the answer must come from, matched against the page's label.
	pub Model:String,
}

/// What a settled turn produced.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Answer {
	/// The URL of the page the answer was read from.
	///
	/// The page's own, not the one the caller asked for. A caller that binds a
	/// tab supplies a URL as a fallback only, and a note filed under that
	/// fallback would name a thread the answer did not come from.
	pub URL:String,
	/// The answer's text, read once the page stopped changing it.
	pub Text:String,
	/// The same answer read the other way, from the same snapshot.
	///
	/// Empty when the call read one rendering, which is most calls: a
	/// rendering that was never asked for is not a rendering that came back
	/// empty, and the one an answer does carry is in [`Answer::Text`].
	///
	/// A driver fills this from the read that produced `Text` and from no
	/// other, because a second pass over the page is a second moment: the pair
	/// would then be a claim about two snapshots, and the page never said the
	/// answer stood still between them.
	pub Markdown:String,
	/// The work the answer reports having done, in the order it did it.
	pub Trace:Vec<Step>,
	/// The sources the answer cites.
	pub Sources:Vec<Source>,
	/// The model label the page showed when the question was sent.
	pub Model:String,
	/// The composer mode the page was in when the question was sent.
	pub Mode:String,
}

/// The handle a page is bound to.
///
/// The driver passes it back to the `Page` that issued it and never reads it,
/// so a provider may hang whatever it needs there - a target id, a session, a
/// socket. `URL` is the one field the port keeps, because a failure report that
/// cannot say which page failed is not a report.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Target {
	/// The page this session is bound to.
	pub URL:String,
}

/// What a page operation reports when it fails.
///
/// A page is not this capability, so it does not speak this capability's four
/// codes. It says what it observed, and the driver maps that onto `not-ready`
/// with the observation in `detail` - which is the whole point of keeping the
/// detail apart from the sentence.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Failure {
	/// The sentence a human reads.
	pub Message:String,
	/// What was observed.
	pub Detail:String,
}

impl Failure {
	/// A failure with a sentence and nothing observed.
	pub fn Of(Message:impl Into<String>) -> Self { Self { Message:Message.into(), Detail:String::new() } }
}

/// The browser as this capability needs it: one page, asked one expression at a
/// time.
///
/// What the port demands of an implementation:
///
/// - `Evaluate` returns the value of the expression, not a wrapper around it,
///   and the expression is a complete JavaScript expression whose value is
///   JSON-serialisable. `document` is the only host object it may touch, so a
///   page that is a tab and a page that is a fixture answer the same driver.
/// - the [`Call`] is honoured: a page that never answers is failed, not
///   awaited. The driver checks the same deadline on its side, so an
///   implementation that ignores it cannot quietly turn a bounded call into an
///   unbounded one.
/// - `Dispose` never fails. A release that fails is a fact about the browser,
///   and a caller that already has its answer must not lose it to a failing
///   cleanup.
pub trait Page: Send + Sync {
	/// Open the page at `URL`.
	fn Open<'a>(&'a self, URL:&'a str, Call:Call) -> FutureBox<'a, Result<Target, Failure>>;
	/// Evaluate one expression in the page and return its value.
	fn Evaluate<'a>(
		&'a self,
		Target:&'a Target,
		Expression:&'a str,
		Call:Call,
	) -> FutureBox<'a, Result<Value, Failure>>;
	/// Release everything the page holds. Never fails.
	fn Dispose<'a>(&'a self, Target:Target) -> FutureBox<'a, ()>;
}

/// The one port this capability offers.
pub trait Chat: Send + Sync {
	/// Ask one question and return the answer once the page has settled.
	fn Ask<'a>(&'a self, Question:Question, Call:Call) -> FutureBox<'a, Result<Answer, crate::Struct::Error::Error>>;
	/// Ask one question, and report the thread on every read until it settles.
	///
	/// This is `Ask` with its inside exposed: the same preconditions, the same
	/// turn correlation and the same completion rule, plus one [`Observation`]
	/// pushed to the sink for every read the driver takes.
	///
	/// The sink is the observer and the [`Call`] is the abort: returning
	/// [`Control::Stop`] disconnects it the way a `MutationObserver`'s
	/// `disconnect()` does, and the call's cancellation ends the wait from the
	/// outside. Both end the observation, neither is an error - the answer so
	/// far comes back either way, so a caller that stopped early can still read
	/// what was there.
	fn Observe<'a>(
		&'a self,
		Question:Question,
		Call:Call,
		Sink:&'a mut (dyn FnMut(&Observation) -> Control + Send),
	) -> FutureBox<'a, Result<Answer, crate::Struct::Error::Error>>;
}

/// One look at the thread, as the driver saw it.
///
/// It carries what was read and not what to do about it: a consumer deciding
/// "still working" from `Busy` and `Settled` is the consumer's business, and a
/// driver that returned a verdict would be answering its own question.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Observation {
	/// How many answers the thread holds.
	pub Count:f64,
	/// The newest answer's text, trimmed.
	pub Text:String,
	/// Whether the page says it is still producing that answer.
	pub Busy:bool,
	/// Whether this answer was not in the thread before the question.
	pub Fresh:bool,
	/// Whether the text has held still long enough, with the page idle, to be
	/// read as settled.
	pub Settled:bool,
}

/// What an observer says back to the driver.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Control {
	/// Keep observing.
	Continue,
	/// Stop observing now; the answer so far is returned.
	Stop,
}

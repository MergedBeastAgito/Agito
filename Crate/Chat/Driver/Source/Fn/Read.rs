//! Turning what a page answered into the port's types, or into `None`.
//!
//! Every value the page returns crosses the port boundary as a
//! [`serde_json::Value`]: the expressions are the driver's own, but the provider is
//! not, and a provider that answers a number where a label was asked for must
//! not be able to make `ask` fail outside the four codes the manifest declares.
//! So a value is read as text when it is text and rendered as text when it is
//! not - the rules then refuse it on its merits, with a code the manifest
//! actually has.

use serde_json::Value;

use crate::Struct::Port::{Candidate, Composer, Inspection, Source, Step, Strip};

/// One read of the thread.
///
/// A thread with no answers is `{ Count: 0, Text: "", Busy: false }`, so "there
/// is no answer yet" and "the page did not answer the question I asked" are
/// different values. Folding them together would let a provider's wrong shape
/// look like an empty thread and cost the caller its whole deadline before
/// `not-ready`.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Answers {
	/// How many answers the thread holds.
	pub Count:f64,
	/// The last answer's text, trimmed; empty when there is none.
	pub Text:String,
	/// The last answer's second rendering, when the read asked for both.
	///
	/// `None` and not an empty string: a read that asked for one rendering
	/// has no second one, and folding "not asked for" into "came back empty"
	/// would let a page that dropped the field look like an answer that
	/// serialised to nothing.
	pub Markdown:Option<String>,
	/// Whether the page says it is still producing the answer.
	///
	/// This is the page's own statement, not an inference from its text: a
	/// pause between tokens, a reasoning step and a slow code block all leave
	/// the text unchanged while the answer is demonstrably unfinished. A page
	/// that offers no such signal answers `false`, which is the honest reading
	/// ("nothing says it is still working") and leaves the text rule in charge.
	pub Busy:bool,
	/// One identifier per answer in the thread, in document order.
	///
	/// An identifier is the text's length and its first characters, which is
	/// enough to tell one turn from another without carrying every turn's text
	/// through the port. It exists so "is this answer new" is answered by
	/// membership rather than by position: a virtualised thread can reorder or
	/// unmount its turns, and a newest-by-position read then hands back an
	/// answer that was already there.
	pub Marks:Vec<String>,
}

/// A value as text: as it is when it is text, rendered when it is not.
pub fn TextOf(Value:Option<&Value>) -> String {
	match Value {
		None | Some(Value::Null) => String::new(),
		Some(Value::String(Text)) => Text.clone(),
		Some(Other) => Other.to_string(),
	}
}

/// A label as it is compared: case and runs of whitespace are not part of a
/// model's name, so both sides are folded. Nothing else is folded: a different
/// model is a different model.
pub fn Normalise(Value:Option<&Value>) -> String { Fold(&TextOf(Value)) }

/// The same folding, over a string rather than a page's value.
pub fn Fold(Text:&str) -> String { Text.split_whitespace().collect::<Vec<&str>>().join(" ").to_lowercase() }

/// What was read, for a failure's detail, without pretending `null` is a value.
pub fn Describe(Value:Option<&Value>) -> String {
	match Value {
		None | Some(Value::Null) => "nothing".to_string(),
		Some(Value::String(Text)) if Text.trim().is_empty() => "nothing".to_string(),
		Some(Other) => Other.to_string(),
	}
}

/// What a text read, for a failure's detail.
pub fn DescribeText(Value:Option<&str>) -> String {
	match Value {
		None => "nothing".to_string(),
		Some(Text) if Text.trim().is_empty() => "nothing".to_string(),
		Some(Text) => serde_json::to_string(Text).unwrap_or_else(|_| format!("{Text:?}")),
	}
}

/// An answer's identifier, computed the way the page computes it: its length in
/// code points and its opening.
///
/// The two sides must agree, which is why the count is code points rather than
/// UTF-16 units: an answer with an emoji in it would otherwise mark differently
/// in the driver than in the page and every read would look new.
pub fn Mark(Text:&str) -> String { format!("{}:{}", Text.chars().count(), Text.chars().take(48).collect::<String>()) }

/// One read of the thread, or `None` when the page answered with something that
/// is not one.
pub fn AsAnswers(Value:&Value) -> Option<Answers> {
	let Object = Value.as_object()?;
	let Count = Object.get("count")?.as_f64()?;
	Some(Answers {
		Count,
		Text:TextOf(Object.get("text")),
		Markdown:Object.get("markdown").and_then(Value::as_str).map(|Text| Text.to_string()),
		Busy:Object.get("busy").and_then(Value::as_bool).unwrap_or(false),
		Marks:Object
			.get("marks")
			.and_then(Value::as_array)
			.map(|Marks| Marks.iter().map(|Mark| TextOf(Some(Mark))).collect())
			.unwrap_or_default(),
	})
}

/// The steps the page reported, or `None` when it reported something else.
pub fn AsSteps(Value:&Value) -> Option<Vec<Step>> {
	let Entries = Value.as_array()?;
	Some(
		Entries
			.iter()
			.map(|Entry| {
				let Object = Entry.as_object();
				Step {
					Depth:Object
						.and_then(|Object| Object.get("depth"))
						.and_then(Value::as_f64)
						.map(|Depth| Depth as i64)
						.unwrap_or(0),
					Label:TextOf(Object.and_then(|Object| Object.get("label"))),
					Detail:TextOf(Object.and_then(|Object| Object.get("detail"))),
				}
			})
			.collect(),
	)
}

/// The sources the page cited, or `None` when it cited something else.
pub fn AsSources(Value:&Value) -> Option<Vec<Source>> {
	let Entries = Value.as_array()?;
	Some(
		Entries
			.iter()
			.map(|Entry| {
				let Object = Entry.as_object();
				Source {
					URL:TextOf(Object.and_then(|Object| Object.get("url"))),
					Label:TextOf(Object.and_then(|Object| Object.get("label"))),
				}
			})
			.collect(),
	)
}

/// What the composer holds, or `None` when the page answered something else.
///
/// `null` is `None`: the selector matched nothing, which is not a composer that
/// holds nothing. The difference is the whole of the first precondition, so the
/// two are never folded together.
pub fn AsComposer(Value:&Value) -> Option<Composer> {
	let Object = Value.as_object()?;
	Some(Composer {
		Kind:TextOf(Object.get("kind")),
		// A model the page reports as `null` is an editor that cannot be
		// asked, not an editor that holds nothing: the empty case is the
		// empty string.
		Model:Object.get("model").and_then(Value::as_str).map(|Text| Text.to_string()),
		DOM:TextOf(Object.get("dom")),
		Source:TextOf(Object.get("source")),
	})
}

/// The composer row's popup controls and the model among them, or `None` when
/// the page answered something else.
pub fn AsInspection(Value:&Value) -> Option<Inspection> {
	let Object = Value.as_object()?;
	let Candidates = Object
		.get("candidates")
		.and_then(Value::as_array)
		.map(|Entries| {
			Entries
				.iter()
				.enumerate()
				.map(|(Index, Entry)| {
					let One = Entry.as_object();
					Candidate {
						Index:One
							.and_then(|One| One.get("index"))
							.and_then(Value::as_f64)
							.map(|At| At as usize)
							.unwrap_or(Index),
						Label:TextOf(One.and_then(|One| One.get("label"))),
						Text:TextOf(One.and_then(|One| One.get("text"))),
						Popup:TextOf(One.and_then(|One| One.get("popup"))),
						Reason:TextOf(One.and_then(|One| One.get("reason"))),
					}
				})
				.collect::<Vec<Candidate>>()
		})
		.unwrap_or_default();
	Some(Inspection {
		URL:String::new(),
		Chosen:Object.get("chosen").and_then(Value::as_f64).map(|At| At as usize),
		Model:Object.get("model").and_then(Value::as_str).map(|Text| Text.to_string()),
		Candidates,
		Composer:Composer::default(),
	})
}

/// Whether the page said its platform selects everything with Command.
pub fn AsMac(Value:&Value) -> bool { Value.get("mac").and_then(Value::as_bool).unwrap_or(false) }

/// What the composer holds against what was typed.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Comparison {
	/// Whether the composer holds what was typed.
	pub Matched:bool,
	/// The held text's length in characters.
	pub Held:usize,
	/// The typed text's length in characters.
	pub Want:usize,
	/// The first offset at which the two differ once whitespace is removed.
	pub Offset:usize,
	/// What the composer holds from that offset.
	pub HeldAt:String,
	/// What was typed from that offset.
	pub WantAt:String,
}

impl Comparison {
	/// The sentence a failure states: the two lengths and where they part.
	pub fn Detail(&self) -> String {
		format!(
			"held {} characters, wanted {}; first difference at {}: {:?} against {:?}",
			self.Held, self.Want, self.Offset, self.HeldAt, self.WantAt
		)
	}
}

/// Compare what the composer holds with what was typed.
///
/// # The rule, and why it is two rules
///
/// Whitespace is removed from both sides before they are compared, because an
/// editor is allowed to lose it. A paragraph break is a node in the editor's
/// document and a rendered block in the DOM, and the reference product's editor
/// renders one with no separator at all: the text reads `them.The` where the
/// caller wrote `them. The`. Collapsing runs to a single space - the rule this
/// replaced - calls that a mismatch and refuses a question that was typed
/// exactly as written.
///
/// A comparison that ignores whitespace entirely is too permissive on its own,
/// so it is paired with a length range:
///
/// - **the floor** is the typed text's length with whitespace removed. Every
///   character the caller typed has to be there, so a prefix or a truncated
///   insert falls below it and is refused;
/// - **the ceiling** is the typed length plus that floor, which is what an
///   editor holds when it keeps every break as two characters. A composer that
///   still holds the previous draft *and* this question is above it, and is
///   refused.
///
/// What this deliberately accepts: an editor that drops every paragraph break
/// while keeping every character. That is below the typed length and above the
/// floor, and it is not distinguishable from the same editor's own rendering -
/// measured on the reference product, the editor's model reports the same
/// separator-free text its DOM does. Refusing it would refuse every
/// multi-paragraph question, which is the defect this replaced.
pub fn Compare(Held:&str, Expected:&str) -> Comparison {
	let Wanted = Strip(Expected);
	let Actual = Strip(Held);
	let Floor = Wanted.chars().count();
	let Ceiling = Expected.chars().count() + Floor;
	let Held = Held.chars().count();
	let Want = Expected.chars().count();
	let Matched = Actual == Wanted && Held >= Floor && Held <= Ceiling;
	let Limit = Actual.chars().count().min(Wanted.chars().count());
	let Offset = Actual
		.chars()
		.zip(Wanted.chars())
		.take_while(|(Left, Right)| Left == Right)
		.count()
		.min(Limit);
	Comparison {
		Matched,
		Held,
		Want,
		Offset,
		HeldAt:Actual.chars().skip(Offset).take(24).collect(),
		WantAt:Wanted.chars().skip(Offset).take(24).collect(),
	}
}

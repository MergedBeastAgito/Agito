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

use crate::Struct::Port::{Source, Step};

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
pub fn Normalise(Value:Option<&Value>) -> String {
	TextOf(Value).split_whitespace().collect::<Vec<&str>>().join(" ").to_lowercase()
}

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
					Url:TextOf(Object.and_then(|Object| Object.get("url"))),
					Label:TextOf(Object.and_then(|Object| Object.get("label"))),
				}
			})
			.collect(),
	)
}

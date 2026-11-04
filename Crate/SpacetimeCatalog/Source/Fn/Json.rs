//! The JSON a row holds, written by the rules that build it.
//!
//! Two columns are JSON: `generation.capabilities` is an array of capability
//! ids and a trace's `payload` is an opaque document. A rule that produced one
//! had to render it, and the choice was to render it here rather than take a
//! dependency: the crate has none, so the rules stay a statement about the
//! model instead of a client for a serialiser. What that buys is the property
//! the port exists to keep - no server SDK, no runtime, nothing to install
//! before the rules can be checked.
//!
//! Only what the rules need is here, and it is the escaping JSON requires, not
//! an approximation of it: a capability id or an effect name carrying a quote,
//! a backslash or a control character produces the document `JSON.stringify`
//! produces, and the case beside this module proves it byte for byte.

use std::fmt::Write;

/// One string as a JSON string literal, quotes included.
pub(crate) fn Quoted(Text:&str) -> String {
	let mut Out = String::with_capacity(Text.len() + 2);
	Out.push('"');
	for One in Text.chars() {
		match One {
			'"' => Out.push_str("\\\""),
			'\\' => Out.push_str("\\\\"),
			'\u{8}' => Out.push_str("\\b"),
			'\u{c}' => Out.push_str("\\f"),
			'\n' => Out.push_str("\\n"),
			'\r' => Out.push_str("\\r"),
			'\t' => Out.push_str("\\t"),
			One if (One as u32) < 0x20 => {
				let _ = write!(Out, "\\u{:04x}", One as u32);
			},
			One => Out.push(One),
		}
	}
	Out.push('"');
	Out
}

/// A list of strings as a JSON array.
pub(crate) fn Array(Values:&[String]) -> String {
	let Items:Vec<String> = Values.iter().map(|One| Quoted(One)).collect();
	format!("[{}]", Items.join(","))
}

#[cfg(test)]
mod proof {
	use super::{Array, Quoted};

	#[test]
	fn a_string_is_quoted_and_escaped_the_way_json_requires() {
		assert_eq!(Quoted("high"), "\"high\"");
		assert_eq!(Quoted("a\"b\\c"), "\"a\\\"b\\\\c\"");
		assert_eq!(Quoted("a\nb\tc\rd"), "\"a\\nb\\tc\\rd\"");
		assert_eq!(Quoted("\u{8}\u{c}"), "\"\\b\\f\"");
		assert_eq!(Quoted("\u{1}"), "\"\\u0001\"");
		// Non-ASCII is not escaped: a JSON document is text, and the ids this
		// crate writes are capability names rather than bytes.
		assert_eq!(Quoted("каталог"), "\"каталог\"");
	}

	#[test]
	fn an_array_is_the_ids_in_the_order_it_was_given() {
		assert_eq!(Array(&["agentic.browse@0.1.0".to_string()]), "[\"agentic.browse@0.1.0\"]");
		assert_eq!(
			Array(&["agentic.browse@0.1.0".to_string(), "agentic.chat@0.1.0".to_string()]),
			"[\"agentic.browse@0.1.0\",\"agentic.chat@0.1.0\"]"
		);
		assert_eq!(Array(&[]), "[]");
	}
}

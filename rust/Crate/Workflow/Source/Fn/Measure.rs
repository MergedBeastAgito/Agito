//! The one expression that turns a page into measurements.
//!
//! This is a *workflow* decision, not a capability's: the verification package
//! must not know what an element is, so the convention for reading one lives
//! here, where a particular site can replace it.
//!
//! What it reads: `data-value` when the element carries one, parsed as a number
//! when it is numeric, and the element's trimmed text otherwise. A selector
//! that matches nothing is `present: false` with a `null` value, which is the
//! wire shape the capability's port asks for.

use serde_json::Value;

/// A JavaScript expression that answers one measurement per selector.
pub fn MeasurementExpression(Selectors:&[String]) -> String {
	let Selectors = serde_json::to_string(Selectors).unwrap_or_else(|_| "[]".to_string());
	format!(
		"(() => {{\n\tconst Read = (Selector) => {{\n\t\tconst Element = document.querySelector(Selector);\n\t\tif \
		 (Element === null) return {{ selector: Selector, present: false, value: null }};\n\t\tconst Raw = \
		 Element.getAttribute(\"data-value\");\n\t\tif (Raw !== null) {{\n\t\t\tconst Parsed = \
		 Number(Raw);\n\t\t\treturn {{ selector: Selector, present: true, value: Number.isNaN(Parsed) ? Raw : Parsed \
		 }};\n\t\t}}\n\t\treturn {{ selector: Selector, present: true, value: (Element.innerText || \"\").trim() \
		 }};\n\t}};\n\treturn {Selectors}.map(Read);\n}})()"
	)
}

/// The selectors a caller asked about, as a JSON array, for a harness that
/// wants to see what was sent without building a page.
pub fn MeasurementSelectors(Expression:&str) -> Option<Value> {
	let Start = Expression.rfind("return ")?;
	let Rest = &Expression[Start + "return ".len()..];
	let End = Rest.find(".map(Read)")?;
	serde_json::from_str(&Rest[..End]).ok()
}

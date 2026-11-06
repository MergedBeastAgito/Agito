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
//!
//! The JavaScript is `Source/Script/Measure.ts`, compiled by this crate's
//! `build.rs` and baked in by [`script!`]. The one value this function supplies
//! is the selector list, already a JSON array, and it is the parameter the
//! script declares as `declare const __PARAM_Selectors__: string[];`. So the
//! file is the expression and this function is only its parameter.

use ::RunTime::script;
use serde_json::Value;

/// A JavaScript expression that answers one measurement per selector.
pub fn MeasurementExpression(Selectors:&[String]) -> String {
	let Selectors = serde_json::to_string(Selectors).unwrap_or_else(|_| "[]".to_string());
	script!("Measure"; Selectors = Selectors)
}

/// The selectors a caller asked about, as a JSON array, for a harness that
/// wants to see what was sent without building a page.
pub fn MeasurementSelectors(Expression:&str) -> Option<Value> {
	let Start = Expression.rfind("return ")?;
	let Rest = &Expression[Start + "return ".len()..];
	let End = Rest.find(".map(Read)")?;
	serde_json::from_str(&Rest[..End]).ok()
}

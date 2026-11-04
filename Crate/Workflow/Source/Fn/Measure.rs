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

use AgenticRuntime::script;
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

#[cfg(test)]
mod Tests {
	use AgenticRuntime::Unfilled;

	use super::*;

	/// The script is filled completely, the check that says so bites, and what
	/// the expression was built from can still be read back out of it.
	///
	/// The read-back is the part that a rewrite would break silently:
	/// `MeasurementSelectors` finds `return ` and `.map(Read)` in the built
	/// string, so a template that moved either marker would leave every
	/// measurement test passing and every read-back answering `None`.
	#[test]
	fn the_measure_expression_is_filled_and_can_be_read_back() {
		let Selectors = vec!["#main".to_string(), "body > p".to_string()];
		let Built = MeasurementExpression(&Selectors);
		assert_eq!(Unfilled(&Built), None, "the expression still carries a marker");
		assert_eq!(
			Unfilled(script!("Measure")).as_deref(),
			Some("Selectors"),
			"the script carries no marker, so nothing proves the fill ran"
		);
		assert_eq!(MeasurementSelectors(&Built), Some(serde_json::json!(["#main", "body > p"])));
	}

	/// The compiled expression is the expression the page was already being
	/// sent, byte for byte.
	///
	/// The fixture is the *before* side of the compilation card: the script at
	/// the commit before it, with the one substitution the old
	/// `MeasurementExpression` performed. It is a baseline, not a snapshot to
	/// refresh - a reformatting compiler fails here.
	#[test]
	fn the_measure_expression_is_byte_for_byte_the_one_the_page_received() {
		const Held:&str = include_str!("../../Test/Fixture/Measure.json");
		let Fixture:serde_json::Value = serde_json::from_str(Held).expect("the fixture is a JSON object");
		let Selectors:Vec<String> = Fixture["selectors"]
			.as_array()
			.expect("the fixture names the selectors")
			.iter()
			.map(|Value| Value.as_str().expect("a selector is a string").to_string())
			.collect();
		assert_eq!(
			MeasurementExpression(&Selectors),
			Fixture["expression"].as_str().expect("the fixture holds the expression")
		);
	}
}

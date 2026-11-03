//! Reconciling what a transport answered with what the caller asked for.
//!
//! The answer is matched by selector rather than by position, so a provider
//! that batches or reorders cannot hand an invariant another element's number.
//! A selector the transport did not answer for is a broken transport, not an
//! absent element: absence is something the page was observed to have, and an
//! answer that never arrived was not observed at all. Reporting it as absent
//! would invent a page defect out of a provider's omission.

use serde_json::Value;

use crate::Struct::{Error::Error, Port::Measurement};

/// A value as a detail line.
///
/// A provider that answered with the wrong shape is reported by what it sent,
/// not by `[object Object]`, because the detail is the only thing that says
/// which part of the contract it broke.
pub fn Describe(Value:&Value) -> String {
	match Value {
		Value::Null => "null".to_string(),
		Value::String(Text) => serde_json::to_string(Text).unwrap_or_else(|_| format!("{Text:?}")),
		Value::Number(_) | Value::Bool(_) => Value.to_string(),
		Value::Array(Items) => format!("an array of {}", Items.len()),
		Value::Object(_) => "an object".to_string(),
	}
}

/// What a measurement's value may be.
///
/// A provider that answers an object has not finished serialising what it read,
/// and flattening that to `Null` would report "nothing to report" about an
/// element that did report something. The port refuses it instead, naming the
/// selector, so the provider is fixed rather than silently trusted.
fn AsValue(Value:&Value, Selector:&str) -> Result<Value, Error> {
	match Value {
		Value::Null | Value::Number(_) | Value::String(_) => Ok(Value.clone()),
		Other => {
			Err(Error::NotReady(
				"the transport answered with a value the port does not carry",
				format!(
					"{:?} read {}; a measurement is a number, a string or null",
					Selector,
					Describe(Other)
				),
			))
		},
	}
}

/// One entry of a transport's answer, as the port defines a measurement.
fn AsMeasurement(Value:&Value) -> Result<Measurement, Error> {
	let Some(Object) = Value.as_object() else {
		return Err(Error::NotReady(
			"the transport answered with a measurement that is not an object",
			format!("it answered {}", Describe(Value)),
		));
	};
	let Selector = match Object.get("selector") {
		Some(Value::String(Selector)) if !Selector.is_empty() => Selector.clone(),
		_ => {
			return Err(Error::NotReady(
				"the transport answered with a measurement for no selector",
				format!("it answered {}", Describe(Value)),
			));
		},
	};
	let Present = match Object.get("present") {
		Some(Value::Bool(Present)) => *Present,
		Other => {
			return Err(Error::NotReady(
				"the transport answered without saying whether the element was there",
				format!(
					"{:?} carried {} as present",
					Selector,
					Other.map(Describe).unwrap_or_else(|| "nothing".to_string())
				),
			));
		},
	};
	// An absent element cannot carry a value: a stale number travelling with
	// `present: false` is what an invariant that forgot to check would read.
	let Value = if Present {
		AsValue(Object.get("value").unwrap_or(&Value::Null), &Selector)?
	} else {
		Value::Null
	};
	Ok(Measurement { Selector, Present, Value })
}

/// One measurement per asked selector, in the order it was asked for.
pub fn Reconcile(Asked:&[String], Answered:&[Value]) -> Result<Vec<Measurement>, Error> {
	let mut BySelector:Vec<Measurement> = Vec::new();
	for Entry in Answered {
		let Measurement = AsMeasurement(Entry)?;
		BySelector.push(Measurement);
	}
	let mut Ordered = Vec::new();
	for Selector in Asked {
		let Some(Found) = BySelector.iter().find(|Measurement| &Measurement.Selector == Selector) else {
			return Err(Error::NotReady(
				"the transport did not answer for a selector it was asked about",
				format!("nothing was measured for {Selector:?}"),
			));
		};
		// A copy per position: two entries naming one selector must not be one
		// mutable object, or a caller editing one edits the other.
		Ordered.push(Found.clone());
	}
	Ok(Ordered)
}

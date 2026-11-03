//! Judging measurements against invariants. Pure, and it never fails.
//!
//! The absent list is built first and independently of the invariants, because a
//! selector that matched nothing fails the run even when every row that reads
//! through it happened to hold: the design named an element, and the page does not
//! have it.

use std::panic::{catch_unwind, AssertUnwindSafe};

use serde_json::Value;

use crate::Struct::Port::{Evidence, Invariant, Measurement, Outcome, Verdict, Want};

/// Whether one invariant held, without letting any part of it escape.
///
/// A reading that fails, a derivation that fails and a comparator that panics are
/// each recorded as a row that did not hold, because a page defect must not be able
/// to end a run as an exception.
pub fn Judge(Invariant: &Invariant, Measurements: &[Measurement]) -> Evidence {
	let Want = match &Invariant.Want {
		Want::Value(Value) => Outcome::Value(Value.clone()),
		Want::Derive(Derive) => match Derive(Measurements) {
			Ok(Value) => Outcome::Value(Value),
			Err(Why) => Outcome::Threw(Why),
		},
	};
	let Got = match (Invariant.Read)(Measurements) {
		Ok(Value) => Outcome::Value(Value),
		Err(Why) => Outcome::Threw(Why),
	};
	let Held = match (&Got, &Want) {
		(Outcome::Value(Got), Outcome::Value(Want)) => match &Invariant.Test {
			Some(Test) => catch_unwind(AssertUnwindSafe(|| Test(Got, Want))).unwrap_or(false),
			None => Got == Want,
		},
		// A comparison that could not be made has not been satisfied. The row still
		// carries what was read, which is what a reader needs.
		_ => false,
	};
	Evidence {
		Label: Invariant.Label.clone(),
		Got,
		Want,
		Held,
	}
}

/// Judge a set of measurements.
pub fn Check(Invariants: &[Invariant], Measurements: &[Measurement]) -> Verdict {
	let mut Absent: Vec<String> = Vec::new();
	for Measurement in Measurements {
		if Measurement.Present {
			continue;
		}
		if !Absent.iter().any(|Selector| Selector == &Measurement.Selector) {
			Absent.push(Measurement.Selector.clone());
		}
	}
	let Evidence: Vec<Evidence> = Invariants
		.iter()
		.map(|Invariant| Judge(Invariant, Measurements))
		.collect();
	Verdict {
		Held: Absent.is_empty() && Evidence.iter().all(|Row| Row.Held),
		Evidence,
		Absent,
	}
}

/// The measurements a selector names, for a caller writing a reading.
pub fn Measured<'a>(Measurements: &'a [Measurement], Selector: &str) -> Option<&'a Measurement> {
	Measurements
		.iter()
		.find(|Measurement| Measurement.Selector == Selector)
}

/// A measurement's value as a number, for a caller writing a reading.
pub fn Number(Measurements: &[Measurement], Selector: &str) -> Result<Value, String> {
	let Measurement = Measured(Measurements, Selector)
		.ok_or_else(|| format!("nothing was measured for {Selector}"))?;
	if !Measurement.Present {
		return Err(format!("{Selector} is not in the page"));
	}
	match &Measurement.Value {
		Value::Number(Number) => Ok(Value::Number(Number.clone())),
		Other => Err(format!("{Selector} read {Other}, which is not a number")),
	}
}

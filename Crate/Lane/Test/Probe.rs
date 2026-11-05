#![allow(non_snake_case, non_upper_case_globals)]

use AgenticLane::Corpus;
use serde_json::json;

#[test]
fn probe() {
	let C:Corpus = serde_json::from_value(json!({"version": 7, "fields": {"a": {"unit": "m"}}, "lanes": {"x": {"fields": ["a"], "templates": ["t"]}}, "order": ["x"]})).unwrap();
	println!(
		"version={} fields={} lanes={} order={:?}",
		C.Version,
		C.Fields.len(),
		C.Lanes.len(),
		C.Order
	);
	println!("write={}", serde_json::to_value(&C).unwrap());
}

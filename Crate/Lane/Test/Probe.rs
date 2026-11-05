#![allow(non_snake_case, non_upper_case_globals)]

use AgenticLane::{Corpus, Observation, ObservationField};
use serde_json::json;

#[test]
fn probe() {
	println!(
		"observation default = {}",
		serde_json::to_value(Observation::default()).unwrap()
	);
	let mut Obs = Observation::default();
	Obs.Fields.insert(
		"speed".to_string(),
		ObservationField { Unit:Some("km/h".to_string()), ..Default::default() },
	);
	println!("observation one field = {}", serde_json::to_value(&Obs).unwrap());
	println!("corpus default = {}", serde_json::to_value(Corpus::default()).unwrap());
	let Read:Corpus = serde_json::from_value(json!({"state": {"a": {}}})).unwrap();
	println!("read state -> fields {}", Read.Fields.len());
}

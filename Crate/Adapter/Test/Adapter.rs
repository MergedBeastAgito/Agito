//! The port as a consumer holds it, and as a third party implements it.
//!
//! Two of these cases are about the shape rather than about behaviour: a
//! foreign implementation - a struct defined here, outside the crate that
//! declares the trait - must satisfy it, and the result must be usable as
//! `Arc<dyn Adapter>` with no concrete type in sight. If either stops holding,
//! `kind: "adapter"` is a label with nothing behind it, because the whole point
//! of the manifest kind is that the value it describes can be passed on.

#![allow(non_snake_case, non_upper_case_globals)]

use std::{sync::Arc, time::Duration};

use AgenticAdapter::{Adapter, Error, Serves, StandIn};
use AgenticRuntime::{BlockOn, Call, FutureBox, Never, Wall};
use serde_json::{Value, json};

/// One call's budget, on the wall clock.
fn Budget() -> Call { Call::Within(Duration::from_secs(1), &Wall, Arc::new(Never)) }

/// A substrate this crate has never seen, written the way a third party would
/// write one: a struct, one port name, and the three methods.
struct Foreign {
	/// Whether it was released.
	Released:std::sync::atomic::AtomicBool,
}

impl Adapter for Foreign {
	fn Port(&self) -> &'static str { "agentic.foreign.drive" }

	fn Call<'a>(&'a self, Request:&'a str, _Call:Call) -> FutureBox<'a, Result<Value, Error>> {
		let Answer = if Request == "speed" {
			Ok(json!({ "speed": 128 }))
		} else {
			Err(Error::New(
				"navigation-failed",
				"the request named nothing this substrate reads",
			))
		};
		Box::pin(std::future::ready(Answer))
	}

	fn Dispose<'a>(&'a self) -> FutureBox<'a, ()> {
		self.Released.store(true, std::sync::atomic::Ordering::SeqCst);
		Box::pin(std::future::ready(()))
	}
}

#[test]
fn a_foreign_implementation_is_usable_as_a_port_and_nothing_else() {
	let Held:Arc<dyn Adapter> = Arc::new(Foreign { Released:std::sync::atomic::AtomicBool::new(false) });
	assert_eq!(Held.Port(), "agentic.foreign.drive");
	let Answer = BlockOn(Held.Call("speed", Budget())).expect("answered");
	assert_eq!(Answer, json!({ "speed": 128 }));
	let Refusal = BlockOn(Held.Call("reverse", Budget())).expect_err("refused");
	assert_eq!(Refusal.Code(), "navigation-failed");
	BlockOn(Held.Dispose());
}

#[test]
fn the_port_is_send_and_sync_so_it_can_be_passed_across_a_thread() {
	fn Assert<T:Send + Sync>() {}
	Assert::<Arc<dyn Adapter>>();
}

#[test]
fn a_stand_in_answers_in_the_order_it_was_given() {
	let Double = StandIn::New("agentic.probe.drive");
	Double.Queue(Ok(json!(1)));
	Double.Queue(Ok(json!(2)));
	assert_eq!(BlockOn(Double.Call("first", Budget())).expect("answered"), json!(1));
	assert_eq!(BlockOn(Double.Call("second", Budget())).expect("answered"), json!(2));
}

#[test]
fn a_stand_in_records_every_request_it_was_handed() {
	let Double = StandIn::New("agentic.probe.drive");
	BlockOn(Double.Call("{\"target\":120}", Budget())).expect("answered");
	BlockOn(Double.Call("{}", Budget())).expect("answered");
	assert_eq!(Double.Requests(), vec!["{\"target\":120}".to_string(), "{}".to_string()]);
}

#[test]
fn an_exhausted_stand_in_answers_null_rather_than_failing() {
	// A double that panicked or failed here would make a caller's own bug look
	// like a defect in the port, which is the one thing a stand-in must not do.
	let Double = StandIn::New("agentic.probe.drive");
	assert_eq!(BlockOn(Double.Call("anything", Budget())).expect("answered"), Value::Null);
	assert_eq!(Double.Requests().len(), 1);
}

#[test]
fn a_stand_in_that_was_released_says_so() {
	let Double = StandIn::New("agentic.probe.drive");
	assert!(!Double.Disposed());
	BlockOn(Double.Dispose());
	assert!(Double.Disposed());
}

#[test]
fn serves_agrees_with_the_manifest_it_was_registered_under() {
	let Double = StandIn::New("agentic.nfsu.drive");
	assert!(Serves(&Double, "agentic.nfsu.drive").is_ok());
	let Why = Serves(&Double, "agentic.browse").expect_err("the ports differ");
	assert!(Why.contains("agentic.nfsu.drive"), "{Why}");
	assert!(Why.contains("agentic.browse"), "{Why}");
}

#[test]
fn an_error_carries_the_code_its_manifest_declares() {
	let Failure = Error::New("timeout", "the page never answered").WithDetail("9000ms");
	assert_eq!(Failure.Code(), "timeout");
	assert_eq!(Failure.Message(), "the page never answered");
	assert_eq!(Failure.Detail(), "9000ms");
	assert_eq!(Failure.to_string(), "timeout: the page never answered (9000ms)");
	let Bare = Error::New("timeout", "the page never answered");
	assert_eq!(Bare.Detail(), "");
	assert_eq!(Bare.to_string(), "timeout: the page never answered");
}

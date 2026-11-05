//! The adapter port, seen from a consumer's position.
//!
//! A crate outside the workspace that implements `agentic.adapter` depends on
//! `agentic-sdk` and on nothing else of this workspace, so this file builds the
//! value the way that crate would: one import path, one trait object, one claim
//! check. `StandIn` stands in for the substrate, which is why the test needs no
//! page, no process and no dynamic loading - and why it says what the port is
//! for rather than how a particular adapter works.
//!
//! What it does not test is a real adapter: a consumer of the facade cannot see
//! one, and a test that reached for the NFSU2 driver would be naming a second
//! crate - which is the dependency this fixture exists to prove unnecessary.

#![allow(non_snake_case, non_upper_case_globals)]

use std::{sync::Arc, time::Duration};

use AgenticSdk::{Adapter, Call, Never, Serves, StandIn, Tokio};

/// One call's budget, on the browser's own clock.
fn Budget() -> Call { Call::Within(Duration::from_secs(1), &Tokio, Arc::new(Never)) }

#[tokio::test]
async fn a_consumer_holds_the_adapter_port_with_one_dependency() {
	// The composition root's act, written with the facade's names: construct the
	// adapter once, hold it as the port, hand it on.
	let Held:Arc<dyn Adapter> = Arc::new(StandIn::New("agentic.nfsu.drive"));
	assert_eq!(Held.Port(), "agentic.nfsu.drive");

	// The claim check a registry-free root performs, because it is the only
	// place the value and the manifest that named it are both in hand.
	assert!(Serves(&*Held, "agentic.nfsu.drive").is_ok());
	let Why = Serves(&*Held, "agentic.browse").expect_err("the ports differ");
	assert!(Why.contains("agentic.nfsu.drive"), "{Why}");
	assert!(Why.contains("agentic.browse"), "{Why}");

	// And the three methods are callable through the trait object: a port whose
	// methods could only be reached on the concrete type would not be a port.
	Held.Call("{}", Budget()).await.expect("the stand-in answers");
	Held.Dispose().await;
}

#[test]
fn the_adapter_error_is_reachable_at_its_own_path() {
	// The root's `Error` is the browse port's, so the adapter's failure
	// vocabulary is named through the port it belongs to - which is also the
	// path a consumer reports a failure from.
	let Failure = AgenticSdk::Port::Adapter::Error::New("timeout", "the substrate never answered");
	assert_eq!(Failure.Code(), "timeout");
	assert_eq!(Failure.Message(), "the substrate never answered");
}

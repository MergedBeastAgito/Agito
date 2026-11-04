//! Whether an adapter is the one its manifest says it is.
//!
//! With no registry there is nobody to ask "who serves `agentic.nfsu.drive`?",
//! and that is the point: the composition root constructed the value, so it
//! already knows which adapter it holds. What it cannot know from the value
//! alone is whether the value agrees with the manifest a harness read - and that
//! is the one question a value can answer about itself.
//!
//! `agentic verify --module <name>` compares a manifest against the code this
//! binary can see. For a third-party adapter it can see no code, so it checks
//! the manifest structurally and says so. This function is the other half, at
//! the only place both halves are in hand.

use crate::Struct::Port::Adapter;

/// Whether `Adapter` serves the port the manifest that registered it declares.
pub fn Serves(Adapter:&dyn Adapter, Port:&str) -> Result<(), String> {
	let Provided = Adapter.Port();
	if Provided == Port {
		return Ok(());
	}
	Err(format!(
		"the adapter reports the port {Provided:?} and the manifest that named it declares {Port:?}"
	))
}

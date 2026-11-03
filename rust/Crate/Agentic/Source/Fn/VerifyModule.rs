//! One module's readiness, from the manifest it ships.
//!
//! The check is the interesting part: a manifest that names four errors while
//! the crate's own type names three is a manifest that has drifted from its
//! code, and a harness reading it would prepare the wrong thing. Comparing the
//! two is what makes `provides.errors` a claim rather than a comment, which is
//! the job the TypeScript contract tests did and the job this verb does for the
//! Rust tree.

use AgenticBrowserCDP::Code as BrowserCode;
use AgenticChatDriver::Code as ChatCode;
use AgenticDesignVerification::Code as DesignCode;
use AgenticRuntime::Code as RuntimeCode;
use AgenticSkillContract::Rule as SkillRule;
use serde_json::{Value, json};

use crate::Struct::Manifest::{Find, Manifest};

/// What one module reports about itself.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Readiness {
	/// The module's logical name.
	pub Module:String,
	/// What kind of thing it is.
	pub Kind:String,
	/// The port it provides.
	pub Port:String,
	/// The retention class it declares.
	pub State:String,
	/// The errors the manifest names.
	pub Manifest:Vec<String>,
	/// The codes the crate's own type declares.
	pub Code:Vec<String>,
	/// Whether the two agree, the state class is one of the four, and the
	/// manifest parses at all.
	pub Ready:bool,
	/// Why it is not ready, when it is not.
	pub Reason:Option<String>,
}

/// The codes each crate's type declares, by module name.
///
/// A crate with no error vocabulary of its own - the workflow and the
/// composition root - declares an empty list, and its manifest must too.
pub fn Declared(Name:&str) -> Option<Vec<&'static str>> {
	Some(match Name {
		"@agentic/runtime" => RuntimeCode::All().into_iter().map(RuntimeCode::Name).collect(),
		"@agentic/skill-contract" => SkillRule::All().into_iter().map(SkillRule::Name).collect(),
		"@agentic/chat-driver" => ChatCode::All().into_iter().map(ChatCode::Name).collect(),
		"@agentic/browser-cdp" => BrowserCode::All().into_iter().map(BrowserCode::Name).collect(),
		"@agentic/design-verification" => DesignCode::All().into_iter().map(DesignCode::Name).collect(),
		"@agentic/workflow" | "@agentic/composition" => Vec::new(),
		_ => return None,
	})
}

/// The states the manifest may name.
fn States() -> Vec<&'static str> {
	AgenticRuntime::States
		.into_iter()
		.map(AgenticRuntime::StateClass::Name)
		.collect()
}

/// Report one module.
pub fn VerifyModule(Name:&str) -> Result<Readiness, String> {
	let Some(Manifest) = Find(Name) else {
		return Err(format!(
			"{Name} is not a module this workspace ships; try one of: {}",
			crate::Struct::Manifest::All
				.iter()
				.map(|Manifest| Manifest.Name)
				.collect::<Vec<&str>>()
				.join(", ")
		));
	};
	let Some(Code) = Declared(Name) else {
		return Err(format!("{Name} has no code vocabulary in this binary"));
	};
	let Parsed:Value = serde_json::from_str(Manifest.JSON)
		.map_err(|Why| format!("{} ships a manifest that does not parse: {Why}", Manifest.Name))?;
	let ManifestErrors:Vec<String> = Parsed
		.get("provides")
		.and_then(|Provides| Provides.get("errors"))
		.and_then(Value::as_array)
		.map(|Errors| Errors.iter().filter_map(Value::as_str).map(str::to_string).collect())
		.unwrap_or_default();
	let Declared:Vec<String> = Code.iter().map(|Code| (*Code).to_string()).collect();
	let State = Parsed.get("state").and_then(Value::as_str).unwrap_or_default().to_string();
	let Reason = if ManifestErrors != Declared {
		Some(format!(
			"the manifest names [{}] and the crate names [{}]",
			ManifestErrors.join(", "),
			Declared.join(", ")
		))
	} else if !States().contains(&State.as_str()) {
		Some(format!("{State:?} is not one of the four state classes"))
	} else {
		None
	};
	Ok(Readiness {
		Module:Manifest.Name.to_string(),
		Kind:Manifest.Kind.to_string(),
		Port:Manifest.Port.to_string(),
		State,
		Manifest:ManifestErrors,
		Code:Declared,
		Ready:Reason.is_none(),
		Reason,
	})
}

/// A readiness report as the JSON a harness reads.
pub fn ReadinessJSON(Readiness:&Readiness) -> Value {
	json!({
		"module": Readiness.Module,
		"kind": Readiness.Kind,
		"port": Readiness.Port,
		"state": Readiness.State,
		"ready": Readiness.Ready,
		"errors": { "manifest": Readiness.Manifest, "code": Readiness.Code },
		"reason": Readiness.Reason,
	})
}

/// Every manifest, as the summary `agentic manifests` prints.
pub fn Catalogue() -> Vec<Manifest> { crate::Struct::Manifest::All.to_vec() }

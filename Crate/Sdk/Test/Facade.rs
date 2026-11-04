//! The facade, checked as a facade rather than as a crate that happens to build.
//!
//! Three claims, none of which the compiler makes on its own:
//!
//! - the crate depends on ports and never on a substrate, so a consumer that
//!   names it cannot inherit the CDP client's choices;
//! - a session is built, bounded and named with nothing but this crate's paths,
//!   which is the flow a consumer performs;
//! - the manifest table it carries is the table the modules actually ship, byte
//!   for byte, rather than a copy that drifted.
//!
//! Nothing here starts a browser. A suite that needed Chromium would turn "this
//! machine has no Brave" into "the facade is broken", and the claim under test
//! is about names, not about pixels.

#![allow(non_snake_case, non_upper_case_globals)]

use std::{fs, path::Path, sync::Arc, time::Duration};

use AgenticSdk::{
	All,
	Browse,
	Call,
	Chrome,
	ChromeOptions,
	Create,
	Find,
	Manifest,
	Never,
	OpenOptions,
	Options,
	Ownership,
	Session,
	Target,
	Tokio,
};

/// The dependency names one section of a manifest declares.
///
/// Read line by line rather than with a TOML parser, because the question is
/// "which keys does this section hold" and a parser would answer a larger one -
/// with a dev-dependency of its own, which is the thing the first test below is
/// trying not to smuggle in.
fn dependencies(Text:&str, Section:&str) -> Vec<String> {
	let mut Inside = false;
	let mut Found = Vec::new();
	for Line in Text.lines() {
		let Trimmed = Line.trim();
		if let Some(Name) = Trimmed.strip_prefix('[').and_then(|Rest| Rest.strip_suffix(']')) {
			Inside = Name == Section;
			continue;
		}
		if !Inside || Trimmed.is_empty() || Trimmed.starts_with('#') {
			continue;
		}
		if let Some((Key, _)) = Trimmed.split_once('=') {
			Found.push(Key.trim().trim_matches('"').to_string());
		}
	}
	Found
}

/// This crate's own manifest, which is the one a consumer's dependency line
/// resolves to.
fn manifest() -> String {
	fs::read_to_string(Path::new(env!("CARGO_MANIFEST_DIR")).join("Cargo.toml")).expect("Cargo.toml is readable")
}

/// This crate's `module.json`.
fn module() -> serde_json::Value {
	let Text =
		fs::read_to_string(Path::new(env!("CARGO_MANIFEST_DIR")).join("module.json")).expect("module.json is readable");
	serde_json::from_str(&Text).expect("module.json is JSON")
}

#[test]
fn the_facade_depends_on_ports_and_never_on_a_substrate() {
	let Declared = dependencies(&manifest(), "dependencies");
	assert_eq!(Declared.len(), 5, "the five ports, and nothing else: {Declared:?}");
	for Name in &Declared {
		assert!(Name.starts_with("agentic-"), "{Name} is not a port of this workspace");
	}
	assert!(
		!Declared.iter().any(|Name| Name == "chromiumoxide"),
		"the substrate is named in the facade's dependency list: {Declared:?}"
	);
}

#[test]
fn a_session_is_built_and_bounded_from_the_facade_alone() {
	// Every name in this body comes from `agentic-sdk`. The session is built but
	// never opened: construction spawns nothing, so this runs with no browser
	// and no runtime.
	let Bounded = Call::Within(Duration::from_secs(30), &Tokio, Arc::new(Never));
	let Port:Session = Create(
		Chrome::New(ChromeOptions { Headless:true, ..ChromeOptions::default() }),
		Arc::new(Tokio),
		Options::default(),
	);
	// The port is the facade's trait, and it is object-safe - which is what
	// makes a substituted fake possible at all.
	let _:&dyn Browse = &Port;

	// `OpenOptions` records who would own the page, and `Target` is the handle
	// the same names describe. A fabricated one proves both are readable
	// through this crate without opening anything.
	let Wanted = OpenOptions::Launch(Bounded);
	assert_eq!(Wanted.Ownership(), Ownership::Launched);
	let Handle = Target {
		Url:"https://example.test/".to_string(),
		Ws:"ws://127.0.0.1:1/devtools/page/0".to_string(),
		Ownership:Ownership::Attached,
	};
	assert_eq!(Handle.Url, "https://example.test/");
	assert_eq!(Handle.Ownership.Name(), "attached");
}

#[test]
fn the_table_agrees_with_the_bytes_each_manifest_ships() {
	assert_eq!(All.len(), 8, "the workspace ships eight modules with a port");
	for One in All {
		let Parsed:serde_json::Value = serde_json::from_str(One.JSON)
			.unwrap_or_else(|Why| panic!("{} ships a manifest that is not JSON: {Why}", One.Name));
		assert_eq!(Parsed["name"], One.Name, "{} names itself", One.Name);
		assert_eq!(Parsed["kind"], One.Kind, "{} declares its kind", One.Name);
		assert_eq!(Parsed["provides"]["port"], One.Port, "{} declares its port", One.Name);
		assert_eq!(Parsed["state"], One.State, "{} declares its state", One.Name);
	}
}

#[test]
fn every_module_the_table_lists_is_reachable_by_name() {
	for One in All {
		let Found:&Manifest = Find(One.Name).unwrap_or_else(|| panic!("{} cannot be found", One.Name));
		assert_eq!(Found, &One);
	}
	assert!(Find("@agentic/not-a-module").is_none());
}

#[test]
fn the_facade_declares_its_own_kind_and_is_not_an_entry_in_the_table() {
	let Module = module();
	assert_eq!(Module["kind"], "facade");
	assert_eq!(Module["name"], "@agentic/sdk");
	// A facade provides no port, so it has nothing a harness would prepare for -
	// which is why it is not one of the eight and why `agentic verify --module`
	// does not answer for it.
	assert!(Module["provides"]["port"].is_null(), "{Module}");
	assert!(!All.iter().any(|One| One.Name == "@agentic/sdk"));
	// What it does declare is the ports it re-exports, which is the statement
	// this crate is: a path over five of them.
	let Required:Vec<&str> = Module["requires"]["ports"]
		.as_array()
		.map(|Ports| Ports.iter().filter_map(serde_json::Value::as_str).collect())
		.unwrap_or_default();
	for Port in [
		"agentic.runtime",
		"agentic.browse",
		"agentic.chat.ask",
		"agentic.verify.design",
		"agentic.skill.validate",
	] {
		assert!(Required.contains(&Port), "{Port} is not declared: {Required:?}");
	}
}

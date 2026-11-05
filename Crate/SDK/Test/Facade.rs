//! The facade, checked as a facade rather than as a crate that happens to build.
//!
//! Four claims, none of which the compiler makes on its own:
//!
//! - the crate depends on ports and never on a substrate, so a consumer that
//!   names it cannot inherit the CDP client's choices;
//! - a session is built, bounded and named with nothing but this crate's paths,
//!   which is the flow a consumer performs;
//! - the adapter port is reachable and implementable through this crate alone,
//!   which is the whole reason a crate outside the tree depends on it;
//! - the manifest table it carries is the table the modules actually ship, byte
//!   for byte, rather than a copy that drifted.
//!
//! Nothing here starts a browser. A suite that needed Chromium would turn "this
//! machine has no Brave" into "the facade is broken", and the claim under test
//! is about names, not about pixels.

#![allow(non_snake_case, non_upper_case_globals)]

use std::{fs, path::Path, sync::Arc, time::Duration};

use AgenticSdk::{
	Adapter,
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
	Serves,
	Session,
	StandIn,
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
	// The five ports a consumer would otherwise name, plus the adapter port a
	// substrate outside this tree implements. Nothing else: a sixth entry that
	// was not a port would be a substrate or a utility arriving through the
	// facade's own dependency line.
	assert_eq!(Declared.len(), 6, "six ports, and nothing else: {Declared:?}");
	assert!(
		Declared.iter().any(|Name| Name == "agentic-adapter"),
		"the adapter port is what a third-party substrate implements: {Declared:?}"
	);
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
fn a_consumer_holds_the_adapter_port_through_the_facade_alone() {
	// The value a composition root passes on, built and held with nothing but
	// this crate's names. `StandIn` stands in for a substrate, so this runs with
	// no page, no process and no dynamic loading - and the point of the whole
	// port is that the root constructs the adapter and hands it on.
	let Held:Arc<dyn Adapter> = Arc::new(StandIn::New("agentic.nfsu.drive"));
	assert_eq!(Held.Port(), "agentic.nfsu.drive");
	assert!(Serves(&*Held, "agentic.nfsu.drive").is_ok());
	// A manifest naming a different port is refused, and the refusal names both
	// halves: the value and the file, because either could be the wrong one.
	let Why = Serves(&*Held, "agentic.browse").expect_err("the ports differ");
	assert!(Why.contains("agentic.nfsu.drive"), "{Why}");
	assert!(Why.contains("agentic.browse"), "{Why}");
}

#[test]
fn the_table_agrees_with_the_bytes_each_manifest_ships() {
	// Deliberately not a count. The table grows when the workspace does, and a
	// number here would fail on the card that adds the next module rather than
	// on a defect - which is the wrong reason for a test to be red.
	assert!(!All.is_empty(), "the table carries the workspace's modules");
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
fn every_port_is_reachable_at_its_own_path() {
	// The three ports the crate root does not carry, and the runtime's own
	// failure vocabulary, each named through the path a consumer would use. A
	// re-export that pointed at a crate nobody can see would still compile here
	// and fail there; these resolve it.
	assert!(std::any::type_name::<AgenticSdk::Port::Browse::Session>().starts_with("AgenticBrowserCDP"));
	assert!(std::any::type_name::<AgenticSdk::Port::Chat::Driver>().starts_with("AgenticChatDriver"));
	assert!(std::any::type_name::<AgenticSdk::Port::Design::Verifier>().starts_with("AgenticDesignVerification"));
	assert!(std::any::type_name::<AgenticSdk::Port::Skill::Contract>().starts_with("AgenticSkillContract"));
	assert!(std::any::type_name::<AgenticSdk::Port::Runtime::Error>().starts_with("AgenticRuntime"));
	assert!(std::any::type_name::<AgenticSdk::Port::Adapter::StandIn>().starts_with("AgenticAdapter"));
	// And the choice the crate documents: the root's `Error` is the browse
	// port's, because open, evaluate and dispose are the calls a consumer makes
	// first.
	assert_eq!(
		std::any::type_name::<AgenticSdk::Error>(),
		std::any::type_name::<AgenticSdk::Port::Browse::Error>()
	);
}

#[test]
fn the_facade_declares_its_own_kind_and_is_not_an_entry_in_the_table() {
	let Module = module();
	assert_eq!(Module["kind"], "facade");
	assert_eq!(Module["name"], "@agentic/sdk");
	// A facade provides no port, so it has nothing a harness would prepare for -
	// which is why it is not one of the ten and why `agentic verify --module`
	// does not answer for it.
	assert!(Module["provides"]["port"].is_null(), "{Module}");
	assert!(!All.iter().any(|One| One.Name == "@agentic/sdk"));
	// What it does declare is the ports it re-exports, which is the statement
	// this crate is: one path over six ports.
	let Required:Vec<&str> = Module["requires"]["ports"]
		.as_array()
		.map(|Ports| Ports.iter().filter_map(serde_json::Value::as_str).collect())
		.unwrap_or_default();
	// The set, not a subset. The facade is a curated surface - the flow a
	// consumer performs - so a seventh port arriving here by accident would be a
	// dependency the consumer did not ask for. Widening it is an edit to this
	// list, which is the point of listing it.
	let mut Declared = Required.clone();
	Declared.sort_unstable();
	let mut Expected = vec![
		"agentic.adapter",
		"agentic.browse",
		"agentic.chat.ask",
		"agentic.runtime",
		"agentic.skill.validate",
		"agentic.verify.design",
	];
	Expected.sort_unstable();
	assert_eq!(Declared, Expected, "the facade re-exports the consumer's flow and nothing else");
	// The four it leaves out are ports the workspace really ships, so each
	// omission is a decision about a module rather than a name that never
	// existed: the composition root is what carries them.
	for Port in [
		"agentic.lane.decide",
		"agentic.catalog.rules",
		"agentic.workflow",
		"agentic.compose",
	] {
		assert!(
			All.iter().any(|One| One.Port == Port),
			"{Port} is outside the facade but no module in the table provides it"
		);
		assert!(!Required.contains(&Port), "{Port} is outside the consumer's flow");
	}
	// A port this crate says it re-exports must be a port some module in the
	// table provides, or the surface promises something nothing answers for.
	for Port in &Required {
		assert!(
			All.iter().any(|One| One.Port == *Port),
			"{Port} is declared by the facade but no module in the table provides it"
		);
	}
}

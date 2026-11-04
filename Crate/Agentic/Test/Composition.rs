//! The composition root: the manifest table, the readiness check, and the
//! command line.
//!
//! Nothing here starts a browser: the two verbs that need one are exercised by
//! hand, and the two that do not are the ones a machine without Chromium still
//! has to be able to run.

use std::{
	fs,
	path::{Path, PathBuf},
};

use Agentic::{All, Command, Declared, Drive, Find, Run, Verb, VerifyModule};
use serde_json::{Value, json};

#[test]
fn the_workspace_ships_nine_manifests_each_with_a_name_a_port_and_a_state() {
	assert_eq!(All.len(), 9);
	for Manifest in All {
		assert!(!Manifest.Name.is_empty());
		assert!(Manifest.Name.starts_with("@agentic/"));
		assert!(Manifest.Port.starts_with("agentic."), "{}", Manifest.Name);
		assert!(!Manifest.Kind.is_empty());
		assert!(!Manifest.JSON.is_empty());
		assert!(Find(Manifest.Name).is_some(), "{} cannot be found by name", Manifest.Name);
	}
	assert!(Find("@agentic/not-a-module").is_none());
}

#[test]
fn every_manifest_names_exactly_the_errors_its_crate_declares() {
	// The check the TypeScript contract tests did, in one place: a manifest
	// that has drifted from its code is a manifest a harness would prepare
	// the wrong thing for.
	for Manifest in All {
		let Readiness = VerifyModule(Manifest.Name, &[]).expect("the module is known");
		let Code = Readiness.Code.clone().expect("a module this binary carries has code");
		assert_eq!(
			Readiness.Manifest,
			Code,
			"{}: manifest [{}] vs code [{}]",
			Manifest.Name,
			Readiness.Manifest.join(", "),
			Code.join(", ")
		);
		assert!(Readiness.Ready, "{}: {:?}", Manifest.Name, Readiness.Reason);
		assert!(!Readiness.External, "{} is not a third-party module", Manifest.Name);
	}
}

#[test]
fn the_error_vocabulary_of_each_crate_is_the_one_the_manifest_promises() {
	assert_eq!(
		Declared("@agentic/runtime").unwrap(),
		vec!["admission-refused", "lease-held", "budget-exhausted"]
	);
	assert_eq!(
		Declared("@agentic/spacetime-catalog").unwrap(),
		vec!["admission-refused", "lease-held", "budget-exhausted"]
	);
	assert_eq!(
		Declared("@agentic/skill-contract").unwrap(),
		vec!["unparsable", "unlisted-reference", "uncited"]
	);
	assert_eq!(
		Declared("@agentic/chat-driver").unwrap(),
		vec!["not-ready", "wrong-mode", "wrong-model", "truncated"]
	);
	assert_eq!(
		Declared("@agentic/browser-cdp").unwrap(),
		vec!["browser-unavailable", "navigation-failed", "timeout"]
	);
	assert_eq!(
		Declared("@agentic/design-verification").unwrap(),
		vec!["absent-element", "invariant-broke", "not-ready"]
	);
	assert_eq!(Declared("@agentic/workflow").unwrap(), Vec::<&str>::new());
	assert_eq!(
		Declared("@agentic/adapter").unwrap(),
		Vec::<&str>::new(),
		"the port shape names no codes: a code belongs to the adapter that reports it"
	);
	assert!(Declared("@agentic/not-a-module").is_none());
}

#[test]
fn an_unknown_module_is_refused_with_the_list_of_the_ones_that_exist() {
	let Why = VerifyModule("@agentic/nope", &[]).expect_err("refused");
	assert!(Why.contains("@agentic/nope"));
	assert!(Why.contains("@agentic/runtime"));
}

#[test]
fn the_readiness_report_is_the_json_a_harness_reads() {
	let Readiness = VerifyModule("@agentic/chat-driver", &[]).expect("the module is known");
	let JSON = Agentic::Fn::VerifyModule::ReadinessJSON(&Readiness);
	assert_eq!(JSON["module"], "@agentic/chat-driver");
	assert_eq!(JSON["ready"], true);
	assert_eq!(JSON["external"], false);
	assert_eq!(JSON["state"], "holds-transport");
	assert_eq!(JSON["errors"]["manifest"], JSON["errors"]["code"]);
}

#[test]
fn the_manifests_verb_lists_every_module() {
	let Text = Drive(Run(Verb::Manifests { Adapter:Vec::new() })).expect("listed");
	let Lines:Vec<&str> = Text.lines().collect();
	assert_eq!(Lines.len(), 9);
	assert!(Text.contains("@agentic/runtime"));
	assert!(Text.contains("@agentic/adapter"));
	assert!(Text.contains("agentic.browse"));
	// Every row says where it came from, and the workspace's own rows are the
	// ones this binary carries.
	assert_eq!(Lines.iter().filter(|Line| Line.ends_with("workspace")).count(), 9);
}

#[test]
fn the_verify_verb_answers_ready_for_every_module() {
	for Manifest in All {
		let Text = Drive(Run(Verb::Verify { Module:Manifest.Name.to_string(), Adapter:Vec::new() }))
			.expect("ready");
		assert!(Text.contains("\"ready\": true"), "{}", Manifest.Name);
	}
}

#[test]
fn the_skills_verb_reports_a_corpus_without_a_browser() {
	let Root = Path::new(env!("CARGO_TARGET_TMPDIR")).join("cli-corpus");
	let _ = fs::remove_dir_all(&Root);
	let Skill = Root.join("Alpha");
	fs::create_dir_all(&Skill).expect("the corpus is writable");
	fs::write(
		Skill.join("SKILL.md"),
		"---\nname: alpha\ndescription: One honest skill.\n---\n",
	)
	.expect("SKILL.md");
	fs::write(Root.join("Index.md"), "- [Alpha](Alpha/SKILL.md)\n").expect("the index");
	let Text = Drive(Run(Verb::Skills { Root:Root.to_string_lossy().into_owned(), Json:false })).expect("swept");
	assert!(Text.contains("1 skill(s)"));
	assert!(Text.contains("0 finding(s)"));
	assert!(Text.contains("sha256:"));
}

#[test]
fn the_skills_verb_can_answer_json() {
	let Root = Path::new(env!("CARGO_TARGET_TMPDIR")).join("cli-corpus-json");
	let _ = fs::remove_dir_all(&Root);
	let Skill = Root.join("Alpha");
	fs::create_dir_all(&Skill).expect("the corpus is writable");
	fs::write(
		Skill.join("SKILL.md"),
		"---\nname: alpha\ndescription: One honest skill.\n---\n",
	)
	.expect("SKILL.md");
	fs::write(Root.join("Index.md"), "- [Alpha](Alpha/SKILL.md)\n").expect("the index");
	let Text = Drive(Run(Verb::Skills { Root:Root.to_string_lossy().into_owned(), Json:true })).expect("swept");
	let Parsed:serde_json::Value = serde_json::from_str(&Text).expect("parsed");
	assert_eq!(Parsed["skills"][0]["Name"], "Alpha");
	assert_eq!(Parsed["findings"], serde_json::json!([]));
	assert!(Parsed["digest"].as_str().unwrap_or_default().starts_with("sha256:"));
}

#[test]
fn the_command_line_parses_every_verb() {
	let Cases = vec![
		vec!["agentic", "manifests"],
		vec!["agentic", "verify", "--module", "@agentic/runtime"],
		vec!["agentic", "skills", "/tmp"],
		vec!["agentic", "skills", "/tmp", "--json"],
		vec!["agentic", "browse", "--url", "https://example.test/"],
		vec![
			"agentic",
			"chat",
			"--url",
			"https://chat.example/",
			"--model",
			"sol",
			"--question",
			"why",
		],
		vec![
			"agentic",
			"chat",
			"--url",
			"https://chat.example/",
			"--model",
			"sol",
			"--question",
			"why",
			"--format",
			"markdown",
		],
		vec![
			"agentic",
			"design",
			"--url",
			"https://example.test/",
			"--selector",
			"[data-rail]",
		],
	];
	for Case in Cases {
		let Arguments:Vec<String> = Case.iter().map(|Argument| Argument.to_string()).collect();
		assert!(Command::ParseFrom(Arguments.clone()).is_ok(), "{Arguments:?} did not parse");
	}
}

#[test]
fn the_help_lists_every_command() {
	let Error = Command::ParseFrom(vec!["agentic".to_string(), "--help".to_string()]).expect_err("--help is not a run");
	let Text = Error.to_string();
	for Verb in ["manifests", "verify", "skills", "browse", "chat", "design"] {
		assert!(Text.contains(Verb), "{Verb} is not in --help:\n{Text}");
	}
}

#[test]
fn an_unknown_verb_is_a_parse_error_and_not_a_run() {
	assert!(Command::ParseFrom(vec!["agentic".to_string(), "frobnicate".to_string()]).is_err());
}

// ---------------------------------------------------------------------------
// Third-party modules: the `adapter` kind and the roots it is found through.
//
// The fixture is the shape the NFSU driver's `ADAPTER.md` describes - a name, a
// port, a required port, an artefact - because that file is the specification
// this half of the workspace was written against. Nothing here starts a browser
// or a driver: a third-party module is read, not run.
// ---------------------------------------------------------------------------

/// A fresh scratch root for one case, so two cases cannot see each other's
/// manifests.
fn scratch(Name:&str) -> PathBuf {
	let Root = Path::new(env!("CARGO_TARGET_TMPDIR")).join(Name);
	let _ = fs::remove_dir_all(&Root);
	fs::create_dir_all(&Root).expect("the scratch root is writable");
	Root
}

/// One third-party crate directory: a `module.json` and nothing else.
fn manifest(Directory:&Path, Name:&str, Kind:&str, Port:&str, Requires:&[&str]) {
	fs::create_dir_all(Directory).expect("the crate directory is writable");
	let JSON = json!({
		"name": Name,
		"kind": Kind,
		"version": "0.1.0",
		"provides": { "port": Port, "errors": ["browser-unavailable", "timeout"] },
		"requires": { "ports": Requires },
		"artefact": { "entry": "Target/release/probe", "hash": null, "runtime": "rust>=1.88" },
		"state": "holds-transport",
		"cost": { "coldMs": 1, "warmMs": 1, "network": false },
		"verify": "cargo run --release"
	});
	fs::write(Directory.join("module.json"), JSON.to_string()).expect("module.json");
}

/// The adapter root as the verbs take it.
fn roots(Root:&Path) -> Vec<String> { vec![Root.to_string_lossy().into_owned()] }

#[test]
fn an_adapter_under_a_root_is_listed_with_its_port() {
	let Root = scratch("adapter-listing");
	manifest(
		&Root.join("nfsu"),
		"@agentic/nfsu-driver",
		"adapter",
		"agentic.nfsu.drive",
		&["agentic.browse"],
	);
	let Text = Drive(Run(Verb::Manifests { Adapter:roots(&Root) })).expect("listed");
	let Lines:Vec<&str> = Text.lines().collect();
	assert_eq!(Lines.len(), 10, "nine shipped and one read:\n{Text}");
	let Row = Lines
		.iter()
		.find(|Line| Line.contains("@agentic/nfsu-driver"))
		.unwrap_or_else(|| panic!("the adapter is not listed:\n{Text}"));
	assert!(Row.contains("adapter"), "{Row}");
	assert!(Row.contains("agentic.nfsu.drive"), "{Row}");
	assert!(
		Row.contains("adapter-listing"),
		"the row names the directory it was read from: {Row}"
	);
}

#[test]
fn an_adapter_under_a_root_verifies_ready_without_its_code() {
	let Root = scratch("adapter-verify");
	manifest(
		&Root.join("nfsu"),
		"@agentic/nfsu-driver",
		"adapter",
		"agentic.nfsu.drive",
		&["agentic.browse"],
	);
	let Text = Drive(Run(Verb::Verify {
		Module:"@agentic/nfsu-driver".to_string(),
		Adapter:roots(&Root),
	}))
	.expect("ready");
	let Parsed:Value = serde_json::from_str(&Text).expect("parsed");
	assert_eq!(Parsed["ready"], true, "{Text}");
	assert_eq!(Parsed["kind"], "adapter");
	assert_eq!(Parsed["port"], "agentic.nfsu.drive");
	assert_eq!(Parsed["external"], true);
	assert_eq!(
		Parsed["errors"]["code"],
		Value::Null,
		"this binary saw no code, and the report says so rather than echoing the manifest"
	);
	assert_eq!(Parsed["errors"]["manifest"], json!(["browser-unavailable", "timeout"]));
}

#[test]
fn an_adapter_that_requires_a_port_nothing_provides_is_not_ready() {
	let Root = scratch("adapter-unsatisfied");
	manifest(
		&Root.join("probe"),
		"@agentic/probe-driver",
		"adapter",
		"agentic.probe.drive",
		&["agentic.nowhere"],
	);
	let Text = Drive(Run(Verb::Verify {
		Module:"@agentic/probe-driver".to_string(),
		Adapter:roots(&Root),
	}))
	.expect_err("not ready");
	assert!(Text.contains("\"ready\": false"), "{Text}");
	assert!(Text.contains("agentic.nowhere"), "{Text}");
}

#[test]
fn a_manifest_that_declares_a_kind_the_reader_does_not_know_is_refused() {
	let Root = scratch("adapter-unknown-kind");
	manifest(&Root.join("plugin"), "@agentic/plugin", "plugin", "agentic.plugin.drive", &[]);
	let Why = Drive(Run(Verb::Manifests { Adapter:roots(&Root) })).expect_err("refused");
	assert!(Why.contains("plugin"), "{Why}");
	// The refusal names the vocabulary rather than only the offending word.
	assert!(Why.contains("adapter"), "{Why}");
	assert!(Why.contains("capability"), "{Why}");
}

#[test]
fn a_third_party_module_cannot_claim_a_name_the_workspace_ships() {
	let Root = scratch("adapter-impostor");
	manifest(&Root.join("impostor"), "@agentic/runtime", "adapter", "agentic.impostor", &[]);
	let Why = Drive(Run(Verb::Manifests { Adapter:roots(&Root) })).expect_err("refused");
	assert!(Why.contains("@agentic/runtime"), "{Why}");
	assert!(Why.contains("already ships"), "{Why}");
}

#[test]
fn two_modules_cannot_claim_one_name() {
	let Root = scratch("adapter-twins");
	manifest(&Root.join("one"), "@agentic/twin", "adapter", "agentic.twin.one", &[]);
	manifest(&Root.join("two"), "@agentic/twin", "adapter", "agentic.twin.two", &[]);
	let Why = Drive(Run(Verb::Manifests { Adapter:roots(&Root) })).expect_err("refused");
	assert!(Why.contains("@agentic/twin"), "{Why}");
	assert!(Why.contains("both declare"), "{Why}");
}

#[test]
fn an_adapter_root_that_does_not_exist_is_skipped_without_breaking_the_listing() {
	// A stale entry in a path list is not a claim about anything. The listing it
	// was named for still has to work.
	let Missing = Path::new(env!("CARGO_TARGET_TMPDIR")).join("adapter-absent");
	let _ = fs::remove_dir_all(&Missing);
	let Text = Drive(Run(Verb::Manifests { Adapter:roots(&Missing) })).expect("listed");
	assert_eq!(Text.lines().count(), 9);
}

#[test]
fn the_adapter_path_value_is_split_the_way_a_path_is() {
	use Agentic::Fn::Discover;

	assert_eq!(Discover::FromValue(Some("/one:/two")).len(), 2);
	assert!(Discover::FromValue(None).is_empty());
	assert!(Discover::FromValue(Some("")).is_empty());
}

#[test]
fn the_composition_root_holds_an_adapter_as_a_port_and_the_two_agree() {
	// The constraint, in code: the adapter is constructed once, held as a port,
	// and the port it reports is the one its manifest declared. A registry would
	// answer the same question by load order; this answers it by identity.
	use std::sync::Arc;

	use AgenticAdapter::{Adapter, Serves, StandIn};

	let Root = scratch("adapter-claim");
	manifest(
		&Root.join("nfsu"),
		"@agentic/nfsu-driver",
		"adapter",
		"agentic.nfsu.drive",
		&["agentic.browse"],
	);
	let Readiness = VerifyModule("@agentic/nfsu-driver", &roots(&Root).iter().map(PathBuf::from).collect::<Vec<_>>())
		.expect("the adapter is known");
	let Held:Arc<dyn Adapter> = Arc::new(StandIn::New("agentic.nfsu.drive"));
	assert!(Serves(Held.as_ref(), &Readiness.Port).is_ok());
	let Wrong:Arc<dyn Adapter> = Arc::new(StandIn::New("agentic.browse"));
	assert!(
		Serves(Wrong.as_ref(), &Readiness.Port).is_err(),
		"a value that serves another port is not the adapter the manifest named"
	);
}

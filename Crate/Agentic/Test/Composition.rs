//! The composition root: the manifest table, the readiness check, and the
//! command line.
//!
//! Nothing here starts a browser: the two verbs that need one are exercised by
//! hand, and the two that do not are the ones a machine without Chromium still
//! has to be able to run.

use std::{fs, path::Path};

use Agentic::{All, Command, Declared, Drive, Find, Run, Verb, VerifyModule};

#[test]
fn the_workspace_ships_seven_manifests_each_with_a_name_a_port_and_a_state() {
	assert_eq!(All.len(), 7);
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
		let Readiness = VerifyModule(Manifest.Name).expect("the module is known");
		assert_eq!(
			Readiness.Manifest,
			Readiness.Code,
			"{}: manifest [{}] vs code [{}]",
			Manifest.Name,
			Readiness.Manifest.join(", "),
			Readiness.Code.join(", ")
		);
		assert!(Readiness.Ready, "{}: {:?}", Manifest.Name, Readiness.Reason);
	}
}

#[test]
fn the_error_vocabulary_of_each_crate_is_the_one_the_manifest_promises() {
	assert_eq!(
		Declared("@agentic/runtime").unwrap(),
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
	assert!(Declared("@agentic/not-a-module").is_none());
}

#[test]
fn an_unknown_module_is_refused_with_the_list_of_the_ones_that_exist() {
	let Why = VerifyModule("@agentic/nope").expect_err("refused");
	assert!(Why.contains("@agentic/nope"));
	assert!(Why.contains("@agentic/runtime"));
}

#[test]
fn the_readiness_report_is_the_json_a_harness_reads() {
	let Readiness = VerifyModule("@agentic/chat-driver").expect("the module is known");
	let JSON = Agentic::Fn::VerifyModule::ReadinessJSON(&Readiness);
	assert_eq!(JSON["module"], "@agentic/chat-driver");
	assert_eq!(JSON["ready"], true);
	assert_eq!(JSON["state"], "holds-transport");
	assert_eq!(JSON["errors"]["manifest"], JSON["errors"]["code"]);
}

#[test]
fn the_manifests_verb_lists_every_module() {
	let Text = Drive(Run(Verb::Manifests)).expect("listed");
	let Lines:Vec<&str> = Text.lines().collect();
	assert_eq!(Lines.len(), 7);
	assert!(Text.contains("@agentic/runtime"));
	assert!(Text.contains("agentic.browse"));
}

#[test]
fn the_verify_verb_answers_ready_for_every_module() {
	for Manifest in All {
		let Text = Drive(Run(Verb::Verify { Module:Manifest.Name.to_string() })).expect("ready");
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

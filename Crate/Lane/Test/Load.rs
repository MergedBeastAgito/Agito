//! The corpus loader, exercised against directories on disk.
//!
//! Every claim here is one [`crate::Fn::Load`] makes in prose, stated as an
//! assertion: the layout, the discovery of lane files, the resolution of lane
//! references, and the refusal that names the path or the id it was about.
//!
//! # Why a second test binary
//!
//! `Test/Lane.rs` exercises the round trip - select, compose, post, parse, meter
//! - and this file exercises the one step before any of that: reading the
//! directory. They are separate binaries because the loader needs corpora that
//! are deliberately broken, and a suite that has to hold both a working round
//! trip and nine refusals in one file is a file nobody can read.
//!
//! # The real corpus
//!
//! [`the_real_corpus_declares_every_lane_and_resolves_every_template`] reads a
//! corpus pointed at by `AGENTIC_CORPUS` and skips, out loud, when the variable
//! is unset: this crate must not name a game, and the composition root is where
//! a real caller's vocabulary belongs
//! (`Crate/Agentic/Test/CorpusLanes.rs` is the permanent gate on it).

#![allow(non_snake_case, non_upper_case_globals)]

use std::{
	env,
	fs,
	path::{Path, PathBuf},
	sync::atomic::{AtomicU32, Ordering},
};

use AgenticLane::{Code, CorpusError, Error, Load, Read};

/// One scratch corpus directory, unique per call.
///
/// Written under the process's temporary directory and removed by the test that
/// made it: a suite that leaves corpora behind fills a disk quietly.
fn Scratch(Name:&str) -> PathBuf {
	static Next:AtomicU32 = AtomicU32::new(0);
	let Root = env::temp_dir().join(format!(
		"agentic-lane-{Name}-{}-{}",
		std::process::id(),
		Next.fetch_add(1, Ordering::Relaxed)
	));
	let _ = fs::remove_dir_all(&Root);
	fs::create_dir_all(&Root).expect("the scratch directory is creatable");
	Root
}

/// Write every `(relative path, text)` under one root.
fn Write(Root:&Path, Files:&[(&str, &str)]) {
	for (Path, Text) in Files {
		let Full = Root.join(Path);
		if let Some(Parent) = Full.parent() {
			fs::create_dir_all(Parent).expect("a parent directory is creatable");
		}
		fs::write(&Full, Text).expect("a fixture file is writable");
	}
}

/// The fixture corpus this crate ships beside its tests.
fn Fixture() -> PathBuf { Path::new(env!("CARGO_MANIFEST_DIR")).join("Test/Fixture/Corpus") }

/// The real corpus, when this run was pointed at one.
fn Real() -> Option<PathBuf> { env::var_os("AGENTIC_CORPUS").map(PathBuf::from) }

/// One corpus's observation, minimal but shaped like a real one.
const Observation:&str = r#"{
	"version": 1,
	"state": {
		"speed": { "type": "number", "unit": "m/s" },
		"heading": { "type": "number", "unit": "deg" }
	},
	"derived": {
		"verdict": "moving, creeping or blocked - computed over a window, not read"
	}
}"#;

/// One corpus's templates and lanes, in the single-file shape.
const Questions:&str = r#"{
	"version": 3,
	"templates": {
		"cruise": {
			"when": "no cue is visible",
			"questions": {
				"action": { "type": "choice", "criteria": { "go": "hold the line", "stop": "slow down" } }
			}
		}
	},
	"lanes": {
		"plain": { "fields": ["speed"], "templates": ["cruise"], "cadence": "every tick" }
	}
}"#;

#[test]
fn the_fixture_corpus_loads_through_its_templates_directory() {
	let Corpus = Load(&Fixture()).expect("the fixture corpus loads");
	assert_eq!(Corpus.Lanes().count(), 2, "two lanes");
	assert_eq!(Corpus.Templates().count(), 2, "two templates");
	assert!(Corpus.Observed("depth"), "a field the observation reads");
	assert!(Corpus.Observed("steady"), "and one it derives");
	assert!(!Corpus.Observed("unobserved"), "and nothing it never named");
	assert_eq!(
		Corpus.Root().map(PathBuf::as_path),
		Some(Fixture().as_path()),
		"the corpus remembers the directory it came from"
	);
}

#[test]
fn a_templates_directory_loads_as_its_own_root() {
	// The same corpus, pointed at one level lower: a corpus copied without its
	// wrapper is not a different corpus.
	let Wrapped = Load(&Fixture()).expect("the wrapped corpus loads");
	let Flat = Load(&Fixture().join("Templates")).expect("the templates directory loads");
	assert_eq!(Flat.Lanes().count(), Wrapped.Lanes().count());
	assert_eq!(Flat.Templates().count(), Wrapped.Templates().count());
	assert_eq!(Flat.Version(), Wrapped.Version());
}

#[test]
fn the_real_corpus_declares_every_lane_and_resolves_every_template() {
	let Some(Root) = Real() else {
		println!("no AGENTIC_CORPUS: the real corpus is not named by this run, so it is not read");
		return;
	};
	let Corpus = Load(&Root).unwrap_or_else(|Why| panic!("{} does not load: {Why} ({})", Root.display(), Why.Detail()));
	let Names:Vec<&str> = Corpus.Lanes().map(|Lane| Lane.Name.as_str()).collect();
	println!(
		"corpus {} · version {} · {} lanes · {} templates · {} read fields",
		Root.display(),
		Corpus.Version(),
		Names.len(),
		Corpus.Templates().count(),
		Corpus.Fields().count()
	);
	for Lane in Corpus.Lanes() {
		println!(
			"lane {:<8} {:2} fields · {:<24} · {}",
			Lane.Name,
			Lane.Fields.len(),
			Lane.Cadence,
			Lane.Templates.join(", ")
		);
		for One in &Lane.Templates {
			assert!(
				Corpus.Template(One).is_some(),
				"lane {} serves {One}, which the corpus does not carry",
				Lane.Name
			);
		}
		for Field in &Lane.Fields {
			// A lane sends a reading or a derived sentence; both are the
			// observation's to declare.
			assert!(
				Corpus.Observed(Field),
				"lane {} sends {Field}, which the observation does not name",
				Lane.Name
			);
		}
	}
	assert_eq!(Names.len(), 5, "the corpus declares five lanes: {Names:?}");
}

#[test]
fn a_missing_file_is_reported_with_the_path_it_expected() {
	let Root = Scratch("missing");
	Write(&Root, &[("Templates/Questions.json", Questions)]);
	let Why = Load(&Root).expect_err("a corpus with no observation is not readable");
	let CorpusError::MissingFile { path } = &Why else {
		panic!("a missing file is MissingFile, not {Why:?}");
	};
	assert!(path.ends_with("Templates/Observation.json"), "{}", path.display());
	assert!(Why.Unreadable(), "a missing file is an unreadable corpus");
	assert!(Why.Message().contains("Observation.json"), "{}", Why.Message());
	assert!(
		Why.Message().contains(&Root.display().to_string()),
		"the message names the path: {}",
		Why.Message()
	);
	// The port's own code, for a caller that only knows the port.
	assert_eq!(Error::from(Why).Code(), Code::CorpusUnreadable);
	let _ = fs::remove_dir_all(&Root);
}

#[test]
fn a_file_that_is_not_json_is_reported_with_its_path_and_the_parser_position() {
	let Root = Scratch("malformed");
	Write(
		&Root,
		&[("Templates/Observation.json", Observation), ("Templates/Questions.json", "{")],
	);
	let Why = Load(&Root).expect_err("a corpus whose question file is not JSON is not readable");
	let CorpusError::MalformedJson { path, .. } = &Why else {
		panic!("a file that is not JSON is MalformedJson, not {Why:?}");
	};
	assert!(path.ends_with("Templates/Questions.json"), "{}", path.display());
	assert!(Why.Message().contains("Questions.json"), "{}", Why.Message());
	assert!(!Why.Detail().is_empty(), "the parser's own complaint is kept");
	assert!(Why.Detail().contains("line"), "{}", Why.Detail());
	assert_eq!(Error::from(Why).Code(), Code::CorpusUnreadable);
	let _ = fs::remove_dir_all(&Root);
}

#[test]
fn a_lane_naming_a_template_nobody_wrote_is_refused_by_lane_and_id() {
	let Root = Scratch("unknown-template");
	let Lanes = r#"{
		"version": 1,
		"templates": { "cruise": { "questions": {
			"action": { "type": "choice", "criteria": { "go": "hold", "stop": "slow" } }
		} } },
		"lanes": { "plain": { "fields": ["speed"], "templates": ["cruise", "absent"] } }
	}"#;
	Write(
		&Root,
		&[("Templates/Observation.json", Observation), ("Templates/Questions.json", Lanes)],
	);
	let Why = Load(&Root).expect_err("a lane serves a template that exists");
	let CorpusError::UnknownTemplateRef { lane, template_id, .. } = &Why else {
		panic!("an unknown template is UnknownTemplateRef, not {Why:?}");
	};
	assert_eq!(lane, "plain");
	assert_eq!(template_id, "absent");
	assert!(Why.Message().contains("plain"), "{}", Why.Message());
	assert!(Why.Message().contains("absent"), "{}", Why.Message());
	assert_eq!(Error::from(Why).Code(), Code::CorpusInvalid);
	let _ = fs::remove_dir_all(&Root);
}

#[test]
fn a_lane_file_is_found_by_its_lanes_object_rather_than_its_name() {
	// The lanes live in a file whose name this crate has never heard of: the
	// layout says where to look, the file name says nothing.
	let Root = Scratch("split");
	let Templates = r#"{
		"version": 4,
		"templates": { "cruise": { "questions": {
			"action": { "type": "choice", "criteria": { "go": "hold", "stop": "slow" } }
		} } }
	}"#;
	let Lanes = r#"{
		"lanes": {
			"plain": { "fields": ["speed", "verdict"], "templates": ["cruise"], "cadence": "every tick" }
		}
	}"#;
	Write(
		&Root,
		&[
			("Templates/Observation.json", Observation),
			("Templates/Questions.json", Templates),
			("shape.json", Lanes),
		],
	);
	let Corpus = Load(&Root).expect("a corpus that keeps its lanes beside its templates loads");
	assert_eq!(Corpus.Version(), 4, "the question file is still the version's source");
	assert_eq!(Corpus.Lanes().count(), 1);
	let Plain = Corpus.Lane("plain").expect("the lane is declared");
	assert_eq!(Plain.Fields, vec!["speed".to_string(), "verdict".to_string()]);
	let _ = fs::remove_dir_all(&Root);
}

#[test]
fn two_sources_declaring_one_lane_are_refused_with_both_paths() {
	let Root = Scratch("duplicate");
	let Lanes = r#"{
		"lanes": { "plain": { "fields": ["speed"], "templates": ["cruise"] } }
	}"#;
	Write(
		&Root,
		&[
			("Templates/Observation.json", Observation),
			("Templates/Questions.json", Questions),
			("extra.json", Lanes),
		],
	);
	let Why = Load(&Root).expect_err("one lane declared twice is refused");
	let CorpusError::DuplicateLane { lane, first, second } = &Why else {
		panic!("a repeated lane is DuplicateLane, not {Why:?}");
	};
	assert_eq!(lane, "plain");
	assert!(first.ends_with("Questions.json"), "{}", first.display());
	assert!(second.ends_with("extra.json"), "{}", second.display());
	assert!(Why.Detail().contains("Questions.json"), "{}", Why.Detail());
	assert!(Why.Detail().contains("extra.json"), "{}", Why.Detail());
	let _ = fs::remove_dir_all(&Root);
}

#[test]
fn a_corpus_declaring_no_lane_names_every_file_it_searched() {
	let Root = Scratch("no-lanes");
	let Templates = r#"{
		"version": 1,
		"templates": { "cruise": { "questions": {
			"action": { "type": "choice", "criteria": { "go": "hold", "stop": "slow" } }
		} } }
	}"#;
	Write(
		&Root,
		&[
			("Templates/Observation.json", Observation),
			("Templates/Questions.json", Templates),
		],
	);
	let Why = Load(&Root).expect_err("a corpus with no lane can select nothing");
	let CorpusError::NoLanes { searched } = &Why else {
		panic!("no lane anywhere is NoLanes, not {Why:?}");
	};
	assert!(!searched.is_empty(), "the refusal says where it looked");
	assert!(Why.Detail().contains("Questions.json"), "{}", Why.Detail());
	assert_eq!(Error::from(Why).Code(), Code::CorpusInvalid);
	let _ = fs::remove_dir_all(&Root);
}

#[test]
fn a_lane_sending_a_field_the_corpus_never_names_is_refused() {
	let Root = Scratch("unknown-field");
	let Lanes = r#"{
		"templates": { "cruise": { "questions": {
			"action": { "type": "choice", "criteria": { "go": "hold", "stop": "slow" } }
		} } },
		"lanes": { "plain": { "fields": ["speed", "altitude"], "templates": ["cruise"] } }
	}"#;
	Write(
		&Root,
		&[("Templates/Observation.json", Observation), ("Templates/Questions.json", Lanes)],
	);
	let Why = Load(&Root).expect_err("a lane may not send a field nobody named");
	let CorpusError::UnknownFieldRef { lane, field, .. } = &Why else {
		panic!("an unknown field is UnknownFieldRef, not {Why:?}");
	};
	assert_eq!(lane, "plain");
	assert_eq!(field, "altitude");
	assert!(Why.Message().contains("altitude"), "{}", Why.Message());
	let _ = fs::remove_dir_all(&Root);
}

#[test]
fn a_condition_the_crate_cannot_read_is_refused_by_lane() {
	let Root = Scratch("bad-condition");
	let Lanes = r#"{
		"templates": { "cruise": { "questions": {
			"action": { "type": "choice", "criteria": { "go": "hold", "stop": "slow" } }
		} } },
		"lanes": { "plain": {
			"fields": ["speed"],
			"templates": ["cruise"],
			"select": { "op": "nonsense", "field": "speed" }
		} }
	}"#;
	Write(
		&Root,
		&[("Templates/Observation.json", Observation), ("Templates/Questions.json", Lanes)],
	);
	let Why = Load(&Root).expect_err("a condition this crate cannot read is refused");
	let CorpusError::BadCondition { lane, .. } = &Why else {
		panic!("an unreadable condition is BadCondition, not {Why:?}");
	};
	assert_eq!(lane, "plain");
	assert!(Why.Message().contains("plain"), "{}", Why.Message());
	let _ = fs::remove_dir_all(&Root);
}

#[test]
fn a_corpus_read_from_two_texts_validates_the_same_way() {
	let Corpus = Read(Observation, Questions).expect("the two texts assemble");
	assert_eq!(Corpus.Lanes().count(), 1);
	assert_eq!(Corpus.Version(), 3);
	assert_eq!(Corpus.Root(), None, "a corpus from strings has no directory");
	let Why = Read(Observation, "{").expect_err("the question text is read too");
	assert!(matches!(Why, CorpusError::MalformedJson { .. }), "{Why:?}");
	assert!(Why.Message().contains("Questions.json"), "{}", Why.Message());
}

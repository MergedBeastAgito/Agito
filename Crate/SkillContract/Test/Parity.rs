//! The Rust side of the differential run for `agentic.skill.validate`.
//!
//! These cases are not written here: they are read from
//! `scenarios/skill-contract.json`, the same file
//! `Package/skill-contract/Test/Parity.test.ts` reads. What this file adds is
//! only the corpus builder, the interpreter and the assertion - the tree, the
//! steps, the expected result and the expected ordered effect list all come
//! from the scenario file, so the two implementations cannot drift by being
//! tested against two hand-written suites.
//!
//! The capability reads a real filesystem, so the corpus is real: the fixture
//! is materialised under `std::env::temp_dir()` exactly as the TypeScript side
//! materialises it under `mkdtemp`, byte for byte, and removed afterwards. Only
//! the root path differs between the two runs, and the root path is outside
//! every observation - `directory` is excluded from the digest and no effect
//! carries a path.
//!
//! `effects` is what the port reported, in order: every skill a `discover`
//! answered and every finding a `validate` reported. Two ports that answered
//! the same findings in a different order disagree here, which is the whole
//! point.
//!
//! `Test/SkillContract.rs` is kept as it is: that suite is the contract, this
//! one is the comparison. `AGENTIC_PARITY_OUT=<path>` makes this test also
//! write what it observed as JSON, which is how `scenarios/run.mjs --compare`
//! gets the Rust half of the differential run.

#![allow(non_snake_case, non_upper_case_globals)]

use std::{
	collections::BTreeMap,
	fs,
	path::{Path, PathBuf},
	sync::atomic::{AtomicU64, Ordering},
	time::{Duration, SystemTime},
};

use AgenticRuntime::BlockOn;
use AgenticSkillContract::{Create, Skill, SkillContract};
use serde_json::{Map, Value, json};

/// Distinguishes two corpora built in the same process, which one test builds
/// one after another.
static Counter:AtomicU64 = AtomicU64::new(0);

/// A corpus on disk, removed when the case is done with it.
struct Corpus {
	Base:PathBuf,
	Root:PathBuf,
}

impl Drop for Corpus {
	fn drop(&mut self) { let _ = fs::remove_dir_all(&self.Base); }
}

/// One `SKILL.md`, as the contract suite writes it.
fn body(Name:&str, Frontmatter:&str) -> String { format!("---\n{Frontmatter}\n---\n\n# {Name}\n\nA body.\n") }

/// The corpus root's index, from the skills that are cited.
fn index(Names:&[String]) -> String {
	let mut Text = Names
		.iter()
		.map(|Name| format!("- [{Name}/]({Name}/)"))
		.collect::<Vec<String>>()
		.join("\n");
	Text.push('\n');
	Text
}

/// The file a `touchAfterFirstValidate` hook writes, without its extension.
fn stem(Document:&str) -> String {
	let Name = Path::new(Document)
		.file_name()
		.map(|Name| Name.to_string_lossy().into_owned())
		.unwrap_or_default();
	match Name.rsplit_once('.') {
		Some((Left, _)) => Left.to_string(),
		None => Name,
	}
}

fn write(Path:&Path, Text:&str) {
	if let Some(Parent) = Path.parent() {
		fs::create_dir_all(Parent).expect("a directory is writable");
	}
	fs::write(Path, Text).expect("a file is writable");
}

/// Every skill a fixture describes: the corpus, then the one linked in.
fn skills_of(Fixture:&Value) -> Vec<Value> {
	let mut Skills:Vec<Value> = Fixture.get("corpus").and_then(Value::as_array).cloned().unwrap_or_default();
	if let Some(Linked) = Fixture.get("symlink").filter(|Value| !Value.is_null()) {
		let mut One = Linked.clone();
		if let Value::Object(Map) = &mut One {
			Map.insert("external".to_string(), Value::Bool(true));
		}
		Skills.push(One);
	}
	Skills
}

/// Build the corpus a fixture describes, and answer it.
///
/// The root sits one directory in from a fresh scratch directory, so the
/// symlinked skill has somewhere to live that is genuinely outside the corpus -
/// a link inside the corpus would prove nothing.
fn materialise(Fixture:&Value) -> Corpus {
	let Unique = format!("skill-parity-{}-{}", std::process::id(), Counter.fetch_add(1, Ordering::SeqCst));
	let Base = std::env::temp_dir().join(Unique);
	let _ = fs::remove_dir_all(&Base);
	fs::create_dir_all(&Base).expect("the scratch directory is writable");
	let Root = Base.join("corpus");
	let Corpus = Corpus { Base, Root };
	if bool_at(Fixture, &["absentRoot"]).unwrap_or(false) {
		return Corpus;
	}
	let Outside = Corpus.Base.join("outside");
	fs::create_dir_all(&Corpus.Root).expect("the corpus root is writable");
	let mut Cited:Vec<String> = Vec::new();
	for Skill in skills_of(Fixture) {
		let Name = string_at(&Skill, &["name"]).unwrap_or_default();
		let External = bool_at(&Skill, &["external"]).unwrap_or(false);
		let Home = if External { Outside.join(&Name) } else { Corpus.Root.join(&Name) };
		fs::create_dir_all(&Home).expect("the skill directory is writable");
		if !bool_at(&Skill, &["noBody"]).unwrap_or(false) {
			write(
				&Home.join("SKILL.md"),
				&body(&Name, &string_at(&Skill, &["frontmatter"]).unwrap_or_default()),
			);
		}
		if let Some(References) = Skill.get("references").and_then(Value::as_object) {
			for (File, Text) in References {
				write(&Home.join("Reference").join(File), Text.as_str().unwrap_or_default());
			}
		}
		if let Some(Text) = Skill.get("referenceIndex").and_then(Value::as_str) {
			write(&Home.join("Reference").join("Index.md"), Text);
		}
		if External {
			#[cfg(unix)]
			std::os::unix::fs::symlink(&Home, Corpus.Root.join(&Name)).expect("the link is created");
		}
		if bool_at(&Skill, &["cited"]).unwrap_or(true) {
			Cited.push(Name);
		}
	}
	if let Some(Files) = Fixture.get("rootFiles").and_then(Value::as_object) {
		for (File, Text) in Files {
			write(&Corpus.Root.join(File), Text.as_str().unwrap_or_default());
		}
	}
	if let Some(Name) = Fixture.get("corpusIndex").and_then(Value::as_str) {
		write(&Corpus.Root.join(Name), &index(&Cited));
	}
	Corpus
}

/// A day forward, on every file under `Path`.
fn touch(Path:&Path) {
	let Ok(Entries) = fs::read_dir(Path) else {
		return;
	};
	for Entry in Entries.flatten() {
		let Child = Entry.path();
		let Ok(Kind) = Entry.file_type() else {
			continue;
		};
		if Kind.is_dir() {
			touch(&Child);
		} else if Kind.is_file() {
			let Later = SystemTime::now() + Duration::from_secs(86_400);
			if let Ok(File) = fs::OpenOptions::new().write(true).open(&Child) {
				let _ = File.set_times(fs::FileTimes::new().set_modified(Later));
			}
		}
	}
}

/// One record, with the four fields the port promises and nothing else.
fn record(Document:&Value) -> Skill {
	Skill {
		Name:string_at(Document, &["name"]).unwrap_or_default(),
		Directory:string_at(Document, &["directory"]).unwrap_or_default(),
		Description:string_at(Document, &["description"]).unwrap_or_default(),
		References:Document
			.get("references")
			.and_then(Value::as_array)
			.map(|List| List.iter().filter_map(Value::as_str).map(str::to_string).collect())
			.unwrap_or_default(),
	}
}

/// The records a `digest` step hands the port: a literal list, or the last
/// discovered records with one mutation applied.
fn digested(Step:&Value, Held:&[Skill]) -> Vec<Skill> {
	if let Some(List) = Step.get("skills").and_then(Value::as_array) {
		return List.iter().map(record).collect();
	}
	let mut Records = Held.to_vec();
	let Mutate = Step.get("mutate").cloned().unwrap_or(Value::Null);
	if bool_at(&Mutate, &["reverse"]).unwrap_or(false) {
		Records.reverse();
	}
	if let Some(At) = number_at(&Mutate, &["at"])
		&& let Some(One) = Records.get_mut(At as usize)
	{
		if let Some(Name) = string_at(&Mutate, &["name"]) {
			One.Name = Name;
		}
		if let Some(Directory) = string_at(&Mutate, &["directory"]) {
			One.Directory = Directory;
		}
		if let Some(Description) = string_at(&Mutate, &["description"]) {
			One.Description = Description;
		}
		if let Some(References) = Mutate.get("references").and_then(Value::as_array) {
			One.References = References.iter().filter_map(Value::as_str).map(str::to_string).collect();
		}
	}
	Records
}

/// Run one scenario, and answer what it did: the result and the ordered
/// effects.
fn run_case(Fixture:&Value, Entry:&Value) -> Value {
	let Fixture = merge(Fixture, Entry.get("fixture").unwrap_or(&Value::Null));
	let Corpus = materialise(&Fixture);
	let Root = Corpus.Root.to_string_lossy().into_owned();
	let Contract = Create();
	let Hook = string_at(&Fixture, &["touchAfterFirstValidate"]);
	let Id = string_at(Entry, &["id"]).unwrap_or_else(|| "<unnamed>".to_string());

	let mut Effects:Vec<String> = Vec::new();
	let mut Held:Vec<Skill> = Vec::new();
	let mut Touched = false;
	let mut Result = Value::Null;

	let Steps = Entry.get("steps").and_then(Value::as_array).cloned().unwrap_or_default();
	for Step in &Steps {
		let Op = string_at(Step, &["op"]).unwrap_or_default();
		match Op.as_str() {
			"discover" => {
				Held = BlockOn(Contract.Discover(&Root));
				for Skill in &Held {
					Effects.push(format!("discovered:{}", Skill.Name));
				}
				Result = json!({
					"kind": "names",
					"names": Held.iter().map(|Skill| Skill.Name.clone()).collect::<Vec<String>>(),
				});
			},
			"touch" => touch(&Corpus.Root),
			"digest" => {
				Result = json!({ "kind": "digest", "value": Contract.Digest(&digested(Step, &Held)) });
			},
			"validate" | "validateDiscovered" => {
				let Wanted = string_at(Step, &["skill"]).unwrap_or_default();
				let One = if Op == "validate" {
					let Discovered = BlockOn(Contract.Discover(&Root));
					for Skill in &Discovered {
						Effects.push(format!("discovered:{}", Skill.Name));
					}
					Discovered.into_iter().find(|Skill| Skill.Name == Wanted)
				} else {
					Held.iter().find(|Skill| Skill.Name == Wanted).cloned()
				};
				let Some(One) = One else {
					panic!("{Id}: no discovered record for {Wanted}");
				};
				let Findings = BlockOn(Contract.Validate(&One));
				for Finding in &Findings {
					Effects.push(format!("finding:{}:{}", Finding.Rule, Finding.Skill));
				}
				Result = json!({
					"kind": "findings",
					"rules": Findings.iter().map(|Finding| Finding.Rule.clone()).collect::<Vec<String>>(),
				});
				if !Touched && let Some(Hook) = &Hook {
					Touched = true;
					write(&Path::new(&One.Directory).join(Hook), &format!("# {}\n", stem(Hook)));
				}
			},
			Other => panic!("unknown skill step {Other}"),
		}
	}

	json!({ "result": Result, "effects": Effects })
}

/// What one observation got wrong, as sentences. Empty means the case agreed.
///
/// This is `scenarios/lib/load.mjs`'s `compare`, written out for the kinds this
/// capability answers: a `findings` result is compared by rule, a `names`
/// result by the names and their order, and a `digest` result by its exact
/// constant - never by a pattern, because the constant is the evidence.
fn mismatches(Observed:&Value, Expect:&Value) -> Vec<String> {
	let mut Wrong = Vec::new();
	let Want = Expect.get("result").cloned().unwrap_or(Value::Null);
	let Got = Observed.get("result").cloned().unwrap_or(Value::Null);
	let Kind = |Document:&Value| Document.get("kind").and_then(Value::as_str).unwrap_or("").to_string();
	if Kind(&Want) != Kind(&Got) {
		Wrong.push(format!("result kind: wanted {}, got {}", Kind(&Want), Kind(&Got)));
	} else {
		match Kind(&Want).as_str() {
			"findings" => {
				if Want.get("rules") != Got.get("rules") {
					Wrong.push(format!(
						"result rules: wanted {}, got {}",
						Want.get("rules").unwrap_or(&Value::Null),
						Got.get("rules").unwrap_or(&Value::Null)
					));
				}
			},
			"names" => {
				if Want.get("names") != Got.get("names") {
					Wrong.push(format!(
						"result names: wanted {}, got {}",
						Want.get("names").unwrap_or(&Value::Null),
						Got.get("names").unwrap_or(&Value::Null)
					));
				}
			},
			"digest" => {
				if let Some(Equals) = Want.get("equals")
					&& Got.get("value") != Some(Equals)
				{
					Wrong.push(format!(
						"result digest: wanted {Equals}, got {}",
						Got.get("value").unwrap_or(&Value::Null)
					));
				}
			},
			Other => Wrong.push(format!("unknown result kind {Other}")),
		}
	}
	if let Some(Effects) = Expect.get("effects")
		&& Effects != Observed.get("effects").unwrap_or(&Value::Null)
	{
		Wrong.push(format!(
			"effects: wanted {Effects}, got {}",
			Observed.get("effects").unwrap_or(&Value::Null)
		));
	}
	Wrong
}

/// Read a dotted path out of a JSON value.
fn at<'a>(Document:&'a Value, Path:&[&str]) -> Option<&'a Value> {
	let mut Cursor = Document;
	for Step in Path {
		Cursor = Cursor.get(*Step)?;
	}
	Some(Cursor)
}

fn string_at(Document:&Value, Path:&[&str]) -> Option<String> {
	at(Document, Path).and_then(Value::as_str).map(str::to_string)
}

fn bool_at(Document:&Value, Path:&[&str]) -> Option<bool> { at(Document, Path).and_then(Value::as_bool) }

fn number_at(Document:&Value, Path:&[&str]) -> Option<u64> { at(Document, Path).and_then(Value::as_u64) }

/// Merge a patch into a fixture, one level deep per section, recursively. A
/// missing patch is not a patch: it leaves the fixture alone.
fn merge(Base:&Value, Patch:&Value) -> Value {
	if Patch.is_null() {
		return Base.clone();
	}
	match (Base, Patch) {
		(Value::Object(Base), Value::Object(Patch)) => {
			let mut Merged = Base.clone();
			for (Key, Patched) in Patch {
				let Next = match Merged.get(Key) {
					Some(Existing) => merge(Existing, Patched),
					None => Patched.clone(),
				};
				Merged.insert(Key.clone(), Next);
			}
			Value::Object(Merged)
		},
		_ => Patch.clone(),
	}
}

/// The scenario file this crate answers to.
fn scenario_file() -> Value {
	let Path = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../scenarios/skill-contract.json");
	let Text = fs::read_to_string(&Path).unwrap_or_else(|Why| panic!("{} is readable: {Why}", Path.display()));
	serde_json::from_str(&Text).unwrap_or_else(|Why| panic!("{} is JSON: {Why}", Path.display()))
}

/// Every case in the file.
fn cases(File:&Value) -> Vec<Value> { File.get("cases").and_then(Value::as_array).cloned().unwrap_or_default() }

/// A case this side cannot express is skipped, and the file says why.
fn skipped_on_rust(Entry:&Value) -> bool {
	Entry
		.get("skip")
		.and_then(Value::as_array)
		.map(|Sides| Sides.iter().any(|Side| Side.as_str() == Some("rust")))
		.unwrap_or(false)
}

/// Every scenario, against its expectation, with what was observed left behind
/// for the differential run to compare against the TypeScript half.
#[test]
fn every_scenario_agrees_with_its_expectation() {
	let File = scenario_file();
	let Fixture = File.get("fixture").cloned().unwrap_or(Value::Null);
	let mut Observed:BTreeMap<String, Value> = BTreeMap::new();
	let mut Wrong:Vec<String> = Vec::new();
	let mut Ran = 0usize;

	for Entry in cases(&File) {
		if skipped_on_rust(&Entry) {
			continue;
		}
		let Id = string_at(&Entry, &["id"]).unwrap_or_else(|| "<unnamed>".to_string());
		let One = run_case(&Fixture, &Entry);
		for Sentence in mismatches(&One, Entry.get("expect").unwrap_or(&Value::Null)) {
			Wrong.push(format!("{Id}: {Sentence}"));
		}
		Observed.insert(Id, One);
		Ran += 1;
	}

	if let Ok(Path) = std::env::var("AGENTIC_PARITY_OUT") {
		let Document =
			serde_json::to_string_pretty(&Value::Object(Observed.clone().into_iter().collect::<Map<String, Value>>()))
				.unwrap_or_default();
		let _ = fs::write(Path, Document);
	}

	assert!(!Observed.is_empty(), "no scenarios were found");
	assert_eq!(
		Ran,
		cases(&File).iter().filter(|Entry| !skipped_on_rust(Entry)).count(),
		"every scenario in the file must run on this side"
	);
	assert!(
		Wrong.is_empty(),
		"the Rust port disagrees with the scenario file:\n{}",
		Wrong.join("\n")
	);
}

/// Every scenario names a case this crate's contract suite has, so the case map
/// in `Documentation/Parity.md` cannot point at a test that was renamed away.
#[test]
fn every_scenario_names_a_case_the_rust_suite_has() {
	let Contract = include_str!("../Test/SkillContract.rs");
	for Entry in cases(&scenario_file()) {
		let Id = string_at(&Entry, &["id"]).unwrap_or_else(|| "<unnamed>".to_string());
		let Name = string_at(&Entry, &["rust"]).unwrap_or_else(|| panic!("{Id} names no Rust case"));
		assert!(
			Contract.contains(&format!("fn {Name}(")),
			"{Id} names a Rust case that does not exist: {Name}"
		);
	}
}

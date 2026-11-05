//! The real corpus on disk, and the five lanes it declares.
//!
//! Every other test in this workspace answers from a corpus written for the test
//! it sits beside. This one reads the corpus a caller actually drives with - the
//! NFSU2 example's `Templates/` directory - because a loader proved only against
//! a fixture written next to it has proved that the two agree, and nothing more.
//!
//! Run it with the table visible:
//! `cargo test -p agentic --test CorpusLanes -- --nocapture`.
//!
//! # Why this lives in the composition root, not beside `Crate/Lane`
//!
//! `Crate/Lane/Test/Lane.rs` states the rule: "If a game or a page name appeared
//! here, the crate would be claiming a caller it does not have." `drive`,
//! `corner`, `recover`, `race` and `context` are that caller's vocabulary, so a
//! test naming them inside the crate would make the claim the crate refuses. The
//! root is where a real caller is wired to a capability, and it is the only crate
//! that both depends on `agentic-lane` and may name a game.
//!
//! # Two places the corpus can be, and why both are read
//!
//! The corpus is a checkout of its own beside this repository, so a checkout of
//! `Agentic` alone - which is what CI has - does not carry it. A test that
//! skipped when the corpus was absent would be a gate that stopped covering the
//! five lanes without saying so, so this crate carries a copy under
//! `Test/Fixture/CorpusLanes/`, read only when the sibling is not there. When
//! both are present they must say the same thing
//! ([`the_vendored_copy_says_what_the_sibling_checkout_says`]), which is what
//! keeps the copy from becoming a second source of truth.
//!
//! `AGENTIC_CORPUS` points the test at a corpus somewhere else entirely and wins
//! over both, because a corpus a caller names is the corpus under test.
//!
//! Nothing here opens a socket: the only I/O is `fs::read_to_string`, and the
//! capability under test declares `"network": false` in its manifest.

#![allow(non_snake_case, non_upper_case_globals)]

use std::{
	collections::BTreeSet,
	env,
	fs,
	path::{Path, PathBuf},
};

use AgenticLane::{Corpus, Load, Rule, Rules, Select, State, When};
use serde_json::{Value, json};

/// The variable that points this test at a corpus directory.
const Variable:&str = "AGENTIC_CORPUS";

/// Where the corpus sits beside this repository.
///
/// The NFSU2 example is a checkout of its own, one level above `Agentic`, so the
/// path is relative to an ancestor rather than to this crate. Written relative
/// because the repository is public: `Scripts/AuditPaths.sh` refuses a tracked
/// file that names a machine.
///
/// The path is the corpus *root*, not its `Templates/` directory: `Load` takes
/// the directory a caller points at and finds the templates inside it, so a
/// corpus copied without its wrapper still loads.
const Relative:&str = "Example/NFSU2";

/// The five lanes the corpus is expected to declare, in the order its own map
/// sorts them.
const Expected:[&str; 5] = ["context", "corner", "drive", "race", "recover"];

/// The copy this crate carries, read when the sibling checkout is absent.
fn Vendored() -> PathBuf { Path::new(env!("CARGO_MANIFEST_DIR")).join("Test/Fixture/CorpusLanes") }

/// The sibling checkout, when this machine has one.
///
/// `AGENTIC_CORPUS` wins when it is set, and is returned without a check: a
/// variable that names a corpus which is not there is a mistake to report by
/// name, not one to paper over with the vendored copy.
fn Sibling() -> Option<PathBuf> {
	if let Some(Given) = env::var_os(Variable) {
		return Some(PathBuf::from(Given));
	}
	Path::new(env!("CARGO_MANIFEST_DIR"))
		.ancestors()
		.take(6)
		.map(|Root| Root.join(Relative))
		.find(|Candidate| Looks(Candidate))
}

/// Whether a directory is a corpus: the observation is either at its root or in
/// the `Templates/` directory beside it, and `Load` reads either shape.
fn Looks(Dir:&Path) -> bool {
	Dir.join("Templates/Observation.json").is_file() || Dir.join("Observation.json").is_file()
}

/// The directory this run reads, and the sibling checkout behind it.
fn Directory() -> (PathBuf, Option<PathBuf>) {
	let Real = Sibling();
	let Dir = Real.clone().unwrap_or_else(Vendored);
	if !Looks(&Dir) {
		panic!(
			"no corpus at {}: set {Variable} to a corpus directory, or check one out at `{Relative}` relative to an \
			 ancestor of {}, or restore the copy at {}",
			Dir.display(),
			env!("CARGO_MANIFEST_DIR"),
			Vendored().display()
		);
	}
	(Dir, Real)
}

/// The corpus at a directory, or a panic naming the directory and the refusal.
fn Read(Dir:&Path) -> Corpus {
	Load(Dir).unwrap_or_else(|Why| panic!("{} does not load: {Why} ({})", Dir.display(), Why.Detail()))
}

/// The selection policy, in the order a state is tested against it.
///
/// The corpus states each template's `when` as a sentence and carries no
/// predicate: `Crate/Lane` makes the condition a value the caller supplies,
/// because the honest one often reads a counter the corpus cannot see -
/// `blocked_ticks` is exactly that, and the observation lists it under
/// `derived`. So this list is the policy, ordered the way the corpus's own
/// cadences read: a race outranks everything, a car that cannot make progress
/// outranks a corner, and the ordinary tick is last.
fn Policy() -> Rules {
	Rules::New(vec![
		Rule::New("race", When::Any(vec![When::Equal("racetype", 1), When::Equal("racetype", 7)])),
		Rule::New(
			"recover",
			When::Any(vec![
				When::Equal("verdict", "blocked"),
				When::Equal("verdict", "stalled"),
				When::Above("blocked_ticks", 2.0),
			]),
		),
		Rule::New("corner", When::Above("yaw_rate", 0.35)),
		Rule::New("context", When::Present("traffic")),
		Rule::New("drive", When::Present("route_cue_x")),
	])
	.Falling("drive")
}

/// One representative state, and the lane and template it must select.
struct Case {
	/// What the state is, in words, so a failure names the case and not an
	/// index.
	Name:&'static str,
	/// The lane the policy must select.
	Lane:&'static str,
	/// The template the lane serves first, which is its ordinary question.
	Template:&'static str,
	/// The observation, written as the JSON a caller would send.
	Observation:Value,
}

/// One state per lane, each chosen so the policy reaches that lane and no
/// earlier one.
fn Cases() -> Vec<Case> {
	vec![
		Case {
			Name:"a race is running",
			Lane:"race",
			Template:"race.start",
			Observation:json!({
				"speed_kmh": 142,
				"gear": 5,
				"racetype": 1,
				"track": 4701,
				"route_cue_x": 0.52
			}),
		},
		Case {
			Name:"against a wall",
			Lane:"recover",
			Template:"recovery.blocked",
			Observation:json!({
				"verdict": "blocked",
				"situation": "nose against a wall, throttle held",
				"speed_kmh": 0,
				"gear": 1,
				"moved_m": 0.2,
				"turned_degrees": 3,
				"blocked_ticks": 4,
				"grounded": [true, true, true, true],
				"surface": [1, 1, 1, 1]
			}),
		},
		Case {
			Name:"turning hard",
			Lane:"corner",
			Template:"corner.entry",
			Observation:json!({
				"speed_kmh": 96,
				"heading_degrees": 210,
				"route_cue_x": 0.31,
				"yaw_rate": 0.62
			}),
		},
		Case {
			Name:"traffic nearby",
			Lane:"context",
			Template:"traffic.ghost",
			Observation:json!({
				"speed_kmh": 48,
				"gear": 3,
				"heading_degrees": 90,
				"position": [0.0, 0.0, 0.0],
				"velocity": [13.3, 0.0, 0.0],
				"route_cue_x": 0.5,
				"route_cue_pixels": 24,
				"road_band_motion": 18.4,
				"traffic": [{ "ahead_m": 34.0, "right_m": 1.2, "speed_kmh": 52, "closing_kmh": 4 }]
			}),
		},
		Case {
			Name:"an ordinary tick",
			Lane:"drive",
			Template:"freeroam.track_cue",
			Observation:json!({
				"speed_kmh": 63,
				"gear": 4,
				"heading_degrees": 45,
				"route_cue_x": 0.47,
				"verdict": "moving",
				"situation": "clear road, cue slightly left",
				"moved_m": 8.4,
				"turned_degrees": 2,
				"blocked_ticks": 0
			}),
		},
	]
}

/// A state as one line, so a printed case stays one line.
fn Line(Value:&Value) -> String { serde_json::to_string(Value).unwrap_or_else(|Why| format!("<unwritable: {Why}>")) }

#[test]
fn the_corpus_on_disk_declares_exactly_the_five_lanes() {
	let (Dir, Real) = Directory();
	println!(
		"corpus {} ({})",
		Dir.display(),
		match Real {
			Some(_) => "the sibling checkout",
			None => "the vendored copy",
		}
	);
	let Corpus = Read(&Dir);
	println!(
		"version {} · {} lanes · {} templates · {} observed fields",
		Corpus.Version(),
		Corpus.Lanes().count(),
		Corpus.Templates().count(),
		Corpus.Fields().count()
	);

	let Names:Vec<&str> = Corpus.Lanes().map(|Lane| Lane.Name.as_str()).collect();
	assert_eq!(
		Names, Expected,
		"the corpus declares {Names:?}, and this test expects exactly {Expected:?}"
	);

	for Lane in Corpus.Lanes() {
		assert!(!Lane.Fields.is_empty(), "lane {} sends no field", Lane.Name);
		assert!(!Lane.Templates.is_empty(), "lane {} serves no template", Lane.Name);
		for Name in &Lane.Templates {
			assert!(
				Corpus.Template(Name).is_some(),
				"lane {} serves {Name}, which the corpus does not carry",
				Lane.Name
			);
		}
		// The loader already refuses a lane sending a field the observation
		// neither reads nor derives; asserted here because the real corpus is the
		// one nobody reviews against the code. `Observed` is the right rule rather
		// than the `state` table alone: the honest lane sends the sentence that
		// says what the numbers mean, and that sentence is derived, not read.
		for Field in &Lane.Fields {
			assert!(
				Corpus.Observed(Field),
				"lane {} sends {Field}, which the observation neither reads nor derives",
				Lane.Name
			);
		}
		println!(
			"lane {:<8} {:2} fields · {} · {}",
			Lane.Name,
			Lane.Fields.len(),
			Lane.Cadence,
			Lane.Templates.join(", ")
		);
	}
}

#[test]
fn every_lane_is_selected_by_the_policy_from_one_representative_state() {
	let (Dir, _) = Directory();
	let Corpus = Read(&Dir);
	let Policy = Policy();
	let mut Selected = BTreeSet::new();

	for Case in Cases() {
		let Body = Line(&Case.Observation);
		let State = State::Of(Case.Observation).expect("a case is a JSON object");
		let Chosen = Select(&Corpus, &State, &Policy, &Rules::default())
			.unwrap_or_else(|Why| panic!("case '{}': {Why}", Case.Name));
		println!(
			"case {:<18} expected {:<24} selected {:<24} {}",
			Case.Name,
			format!("{} / {}", Case.Lane, Case.Template),
			Chosen.Name(),
			Body
		);
		assert_eq!(
			Chosen.Lane.Name, Case.Lane,
			"case '{}' selected lane '{}', expected '{}' for state {Body}",
			Case.Name, Chosen.Lane.Name, Case.Lane
		);
		assert_eq!(
			Chosen.Template.Name, Case.Template,
			"case '{}' selected template '{}', expected '{}' for state {Body}",
			Case.Name, Chosen.Template.Name, Case.Template
		);
		Selected.insert(Chosen.Lane.Name.clone());
	}

	let Wanted:BTreeSet<String> = Expected.iter().map(|Name| Name.to_string()).collect();
	assert_eq!(
		Selected, Wanted,
		"the cases exercised {Selected:?}, and all five lanes must be exercised"
	);
}

#[test]
fn a_state_no_rule_names_falls_back_to_drive() {
	let (Dir, _) = Directory();
	let Corpus = Read(&Dir);
	let Body = json!({ "speed_kmh": 12, "gear": 2 });
	let State = State::Of(Body.clone()).expect("the state is a JSON object");
	let Chosen = Select(&Corpus, &State, &Policy(), &Rules::default()).expect("the fallback names a lane");
	println!(
		"case {:<18} expected {:<24} selected {:<24} {}",
		"nothing holds",
		"drive / freeroam.track_cue",
		Chosen.Name(),
		Line(&Body)
	);
	assert_eq!(
		Chosen.Lane.Name,
		"drive",
		"a state no rule names selected '{}', expected 'drive' for state {}",
		Chosen.Lane.Name,
		Line(&Body)
	);
	assert_eq!(
		Chosen.Template.Name, "freeroam.track_cue",
		"the fallback selected template '{}', expected the drive lane's first: 'freeroam.track_cue'",
		Chosen.Template.Name
	);
}

#[test]
fn the_vendored_copy_says_what_the_sibling_checkout_says() {
	let Some(Real) = Sibling() else {
		println!(
			"no corpus at `{Relative}` from any ancestor of this crate: the copy at {} is the only one this run \
			 reads, so there is nothing to compare it against",
			Vendored().display()
		);
		return;
	};

	for File in ["Templates/Observation.json", "Templates/Questions.json"] {
		let Copied = fs::read_to_string(Vendored().join(File))
			.unwrap_or_else(|Why| panic!("the vendored {File} is unreadable: {Why}"));
		let Beside = fs::read_to_string(Real.join(File))
			.unwrap_or_else(|Why| panic!("{} is unreadable: {Why}", Real.join(File).display()));
		let Copied:Value = serde_json::from_str(&Copied).expect("the vendored copy is JSON");
		let Beside:Value = serde_json::from_str(&Beside).expect("the sibling checkout is JSON");
		assert_eq!(
			Copied,
			Beside,
			"Test/Fixture/CorpusLanes/{File} has drifted from {}: re-copy it, or the five lanes this crate asserts \
			 against are not the ones the corpus declares",
			Real.join(File).display()
		);
		println!("{File} agrees with {}", Real.join(File).display());
	}
}

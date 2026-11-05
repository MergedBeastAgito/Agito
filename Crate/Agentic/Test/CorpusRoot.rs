//! Where the real corpus is, and the states that select its five lanes.
//!
//! Two targets read the same corpus: `Test/CorpusLanes.rs` asserts what it
//! declares and which lane each representative state selects, and
//! `Test/CorpusDecide.rs` runs those selections through the entry point against
//! a canned reply. Both need the same three things - the discovery rule, the
//! five expected lanes, and the representative states - and a second copy of any
//! of them would be a second answer to "which corpus, and which lane wins".
//!
//! The module holds no test of its own. Its claims are made by the targets that
//! include it, each in its own binary, so a corpus that moved would fail a suite
//! named after what it broke.
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
//! (`Test/CorpusLanes.rs::the_vendored_copy_says_what_the_sibling_checkout_says`),
//! which is what keeps the copy from becoming a second source of truth.
//!
//! `AGENTIC_CORPUS` points the tests at a corpus somewhere else entirely and wins
//! over both, because a corpus a caller names is the corpus under test.
//!
//! Nothing here opens a socket: the only I/O is `fs::read_to_string` behind
//! `Load`, and the capability under test declares `"network": false` in its
//! manifest.

#![allow(dead_code)]

use std::{
	env,
	path::{Path, PathBuf},
};

use AgenticLane::{Corpus, Load};
use serde_json::Value;

/// The variable that points these tests at a corpus directory.
pub const Variable:&str = "AGENTIC_CORPUS";

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
pub const Relative:&str = "Example/NFSU2";

/// The five lanes the corpus is expected to declare, in the order its own map
/// sorts them.
pub const Expected:[&str; 5] = ["context", "corner", "drive", "race", "recover"];

/// The copy this crate carries, read when the sibling checkout is absent.
pub fn Vendored() -> PathBuf { Path::new(env!("CARGO_MANIFEST_DIR")).join("Test/Fixture/CorpusLanes") }

/// The sibling checkout, when this machine has one.
///
/// `AGENTIC_CORPUS` wins when it is set, and is returned without a check: a
/// variable that names a corpus which is not there is a mistake to report by
/// name, not one to paper over with the vendored copy.
pub fn Sibling() -> Option<PathBuf> {
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
pub fn Looks(Dir:&Path) -> bool {
	Dir.join("Templates/Observation.json").is_file() || Dir.join("Observation.json").is_file()
}

/// The directory this run reads, and the sibling checkout behind it.
pub fn Directory() -> (PathBuf, Option<PathBuf>) {
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
pub fn Read(Dir:&Path) -> Corpus {
	Load(Dir).unwrap_or_else(|Why| panic!("{} does not load: {Why} ({})", Dir.display(), Why.Detail()))
}

// No selection policy lives here. The corpus states each lane's `select`, so both
// targets pass `Rules::default()` and the conditions in `Questions.json` are what
// select. A rule list beside them would be a second answer to "which lane wins",
// and the first time the two disagreed the test would be pinning the copy rather
// than the corpus. The one thing the corpus cannot state - which of a lane's
// templates to ask - is a caller rule, and these states carry a cue so the
// lane's first template is the ordinary answer.

/// One representative state, and the lane and template it must select.
pub struct Case {
	/// What the state is, in words, so a failure names the case and not an
	/// index.
	pub Name:&'static str,
	/// The lane the policy must select.
	pub Lane:&'static str,
	/// The template the lane serves first, which is its ordinary question.
	pub Template:&'static str,
	/// The observation, written as the JSON a caller would send.
	pub Observation:Value,
}

/// One state per lane, each chosen so the corpus's conditions reach that lane
/// and no earlier one.
pub fn Cases() -> Vec<Case> {
	vec![
		Case {
			Name:"a race is running",
			Lane:"race",
			Template:"race.start",
			Observation:serde_json::json!({
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
			Observation:serde_json::json!({
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
			Observation:serde_json::json!({
				"speed_kmh": 96,
				"heading_degrees": 210,
				"route_cue_x": 0.31,
				"yaw_rate": 0.62,
				"turned_degrees": 22
			}),
		},
		Case {
			Name:"traffic nearby",
			Lane:"context",
			Template:"traffic.ghost",
			Observation:serde_json::json!({
				"speed_kmh": 48,
				"gear": 3,
				"heading_degrees": 90,
				"position": [0.0, 0.0, 0.0],
				"velocity": [13.3, 0.0, 0.0],
				"route_cue_x": 0.5,
				"route_cue_pixels": 24,
				"road_band_motion": 18.4,
				"lead_m": 34,
				"traffic": [{ "ahead_m": 34.0, "right_m": 1.2, "speed_kmh": 52, "closing_kmh": 4 }]
			}),
		},
		Case {
			Name:"an ordinary tick",
			Lane:"drive",
			Template:"freeroam.track_cue",
			Observation:serde_json::json!({
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
pub fn Line(Value:&Value) -> String {
	serde_json::to_string(Value).unwrap_or_else(|Why| format!("<unwritable: {Why}>"))
}

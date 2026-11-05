//! Reading a corpus off a directory, and the two ways a predicate can fail.
//!
//! Every case here builds its corpus in a fresh directory under the system's
//! temp directory and deletes it on the way out, so the suite never reads the
//! real corpus, never touches the network, and two tests can run at once without
//! sharing a file. What the corpus *contains* is the shape a real one has - an
//! observation with readings and derived sentences, templates whose questions
//! are choices and scores, and five lanes that each name the fields their
//! question reads - because a fixture that is not that shape proves nothing
//! about a corpus that is.
//!
//! The five lane names are a corpus's vocabulary, not this crate's: nothing in
//! `Source/` names a lane, and the loader would read a corpus of one lane or of
//! fifty the same way. They are here because a corpus with several lanes and
//! mutually exclusive conditions is what selection is for, and a fixture with
//! two lanes cannot show a lane being chosen rather than defaulted to.
//!
//! # What each test is defending
//!
//! - a load that works, down to the typed question inside a template: a corpus
//!   that parses into names but loses its criteria is a corpus that composes
//!   requests with nothing in them;
//! - a missing file named by its path, a file that is not JSON named by its
//!   path, and a lane naming a template nobody wrote - the three refusals a
//!   corpus author actually hits;
//! - selection by condition: three contexts that must land on three different
//!   lanes, a context no lane holds, and a context two lanes hold at once.
//!
//! The refusals matter as much as the answers: a loader that answered `Ok` for
//! a corpus it could not read, or a selector that returned the first of several
//! matching lanes, would still pass a test that only checked the happy path.

#![allow(non_snake_case, non_upper_case_globals)]

use std::{
	env, fs,
	path::{Path, PathBuf},
	process,
	sync::atomic::{AtomicU64, Ordering},
	time::{SystemTime, UNIX_EPOCH},
};

use AgenticLane::{Context, Corpus, CorpusError, Load, Question, SelectError, SelectLane};
use serde_json::{Value, json};

/// A directory that deletes itself, so a failing test leaves no corpus behind.
///
/// The name carries the process id, a counter and the clock: two test threads in
/// one process, or two `cargo test` runs at once, must not share a directory,
/// because one test's malformed file would be the other's corpus.
struct Scratch {
	/// The directory this guard owns.
	Path:PathBuf,
}

impl Scratch {
	/// A fresh, empty directory under the system's temp directory.
	fn New() -> Self {
		static Next:AtomicU64 = AtomicU64::new(0);
		let Count = Next.fetch_add(1, Ordering::Relaxed);
		let Nanos = SystemTime::now().duration_since(UNIX_EPOCH).map(|Span| Span.subsec_nanos()).unwrap_or_default();
		let Path = env::temp_dir().join(format!("agentic-lane-{}-{Count}-{Nanos}", process::id()));
		// A leftover from a run that was killed: the guard never got to delete
		// it, and reading a stale file would be a test that passes for the
		// wrong reason.
		let _ = fs::remove_dir_all(&Path);
		fs::create_dir_all(&Path).expect("a scratch directory can be created");
		Self { Path }
	}

	/// The corpus root this guard owns.
	fn Root(&self) -> &Path { &self.Path }

	/// Write one file inside the root, creating the directories above it, and
	/// answer the path it landed at - which is what an error assertion compares
	/// against.
	fn Write(&self, Relative:&str, Text:&str) -> PathBuf {
		let Path = self.Path.join(Relative);
		if let Some(Parent) = Path.parent() {
			fs::create_dir_all(Parent).expect("the directory above a corpus file can be created");
		}
		fs::write(&Path, Text).expect("a corpus file can be written");
		Path
	}
}

impl Drop for Scratch {
	fn drop(&mut self) { let _ = fs::remove_dir_all(&self.Path); }
}

/// The observation: readings the ingestion takes, and the sentences it derives.
///
/// Two halves, and both are sendable: a lane that sends `verdict` beside
/// `speed_kmh` is the lane that works, so a loader that refused a derived field
/// would refuse the lanes worth having.
const Observation:&str = r#"{
	"$comment": "what the ingestion can hand a decision model",
	"version": 1,
	"state": {
		"speed_kmh": { "type": "number", "source": "memory", "unit": "km/h", "range": [0, 400] },
		"gear": { "type": "integer", "source": "memory", "range": [-1, 6] },
		"heading_degrees": { "type": "number", "source": "memory", "unit": "deg", "range": [0, 360] },
		"route_cue_x": { "type": "number", "source": "frame", "range": [0, 1] },
		"yaw_rate": { "type": "number", "source": "memory", "unit": "rad/s" },
		"racetype": { "type": "integer", "source": "memory" },
		"road_band_motion": { "type": "number", "source": "frame" }
	},
	"derived": {
		"moved_m": "metres the car actually advanced over the last six ticks",
		"turned_degrees": "heading change over the same window",
		"blocked_ticks": "consecutive ticks with the throttle held and almost no metres",
		"verdict": "one of moving, creeping, blocked, stalled",
		"situation": "the verdict in words, sent beside the numbers"
	}
}"#;

/// The templates: six question sets, a choice and a score in the ones that carry
/// both, which is the shape the endpoint is asked for.
const Templates:&str = r#"{
	"freeroam.cruise": {
		"when": "free roam, no cue on screen, speed below target",
		"questions": {
			"action": {
				"type": "choice",
				"instructions": "The route cue is not visible and the car is under its target pace. What next?",
				"criteria": {
					"throttle": "Hold the line and keep the throttle down",
					"brake": "Slow down",
					"hold": "Coast and change nothing"
				}
			},
			"confidence": {
				"type": "score",
				"instructions": "How sure are you of that action?",
				"criteria": ["guessing", "unsure", "fairly sure", "certain"]
			}
		}
	},
	"freeroam.track_cue": {
		"when": "a route cue is on screen",
		"questions": {
			"action": {
				"type": "choice",
				"instructions": "The route arrow is on screen at route_cue_x. What next?",
				"criteria": {
					"steer_left": "Turn toward the left of the road",
					"steer_right": "Turn toward the right of the road",
					"throttle": "Hold the line and keep the throttle down"
				}
			}
		}
	},
	"corner.entry": {
		"when": "approaching a turn, speed above the turn pace",
		"questions": {
			"action": {
				"type": "choice",
				"instructions": "A turn is coming and the car is above its cornering pace. What next?",
				"criteria": {
					"brake": "Brake before the turn",
					"throttle": "Stay on the throttle through it"
				}
			},
			"severity": {
				"type": "score",
				"instructions": "How much speed should come off?",
				"criteria": ["none", "a little", "a lot", "as much as possible"]
			}
		}
	},
	"recovery.blocked": {
		"when": "verdict is blocked or stalled",
		"questions": {
			"action": {
				"type": "choice",
				"instructions": "The car cannot make progress where it is pointing. Which way should it go?",
				"criteria": {
					"reverse_left": "Back out with the wheel turned left",
					"reverse_right": "Back out with the wheel turned right",
					"throttle": "Hold the throttle down and wait for the car to come free"
				}
			}
		}
	},
	"race.start": {
		"when": "racetype is 1 or 7",
		"questions": {
			"action": {
				"type": "choice",
				"instructions": "A race is running. What should the car do?",
				"criteria": {
					"throttle": "Full throttle, follow the route",
					"brake": "Brake for a corner"
				}
			},
			"risk": {
				"type": "score",
				"instructions": "How much risk is acceptable here?",
				"criteria": ["none", "cautious", "normal", "maximum"]
			}
		}
	},
	"traffic.ghost": {
		"when": "other cars are near and ghost mode is on",
		"questions": {
			"action": {
				"type": "choice",
				"instructions": "Other cars are close but contact cannot happen. What next?",
				"criteria": {
					"throttle": "Ignore them and keep the pace",
					"brake": "Slow for them"
				}
			}
		}
	}
}"#;

/// The lane that holds for every context, which is the tier a corpus uses to say
/// "anything else".
fn Always() -> Value { json!({ "op": "always" }) }

/// A car in a turn at speed: the two numbers that say it is actually rotating.
fn Corner() -> Value {
	json!({ "op": "and", "of": [
		{ "op": "gt", "field": "speed_kmh", "than": 50 },
		{ "op": "gt", "field": "yaw_rate", "than": 0.3 }
	] })
}

/// A race is running.
fn Race() -> Value { json!({ "op": "in", "field": "racetype", "values": [1, 7] }) }

/// Five lanes, with the three conditions a variant has to change passed in.
///
/// `recover` and `context` keep their conditions because every variant needs
/// them: one lane that holds on a derived sentence, and one whose condition is a
/// *negation* over a field that is absent when the route arrow is not on screen -
/// which is the documented way to say "no cue", and the case a loader that read
/// a missing field as zero would get wrong.
fn Lanes(Drive:Value, Corner:Value, Race:Value) -> String {
	json!({
		"drive": {
			"fields": ["speed_kmh", "gear", "heading_degrees", "route_cue_x", "verdict", "situation", "moved_m"],
			"templates": ["freeroam.track_cue", "freeroam.cruise"],
			"select": Drive,
			"cadence": "every tick (150-250 ms)",
			"why": "the fastest lane: the numbers plus the sentence that says what they mean"
		},
		"corner": {
			"fields": ["speed_kmh", "heading_degrees", "route_cue_x", "yaw_rate"],
			"templates": ["corner.entry"],
			"select": Corner,
			"cadence": "on approach to a turn",
			"why": "yaw rate says how the car is actually rotating, which the cue alone cannot"
		},
		"recover": {
			"fields": ["verdict", "situation", "speed_kmh", "gear", "moved_m", "blocked_ticks"],
			"templates": ["recovery.blocked"],
			"select": { "op": "in", "field": "verdict", "values": ["blocked", "stalled"] },
			"cadence": "only while blocked, creeping or slipping",
			"why": "the escape lane: the window's metres are the evidence that the car is against something"
		},
		"race": {
			"fields": ["speed_kmh", "gear", "racetype", "route_cue_x"],
			"templates": ["race.start"],
			"select": Race,
			"cadence": "when racetype is 1 or 7",
			"why": "the race lane adds the two fields that say a race is on at all"
		},
		"context": {
			"fields": ["speed_kmh", "gear", "heading_degrees", "route_cue_x", "road_band_motion"],
			"templates": ["traffic.ghost"],
			"select": { "op": "not", "of": { "op": "gt", "field": "route_cue_x", "than": -1 } },
			"cadence": "rarely - a slow lane for judgement, not control",
			"why": "the full state, for questions where latency does not matter"
		}
	})
	.to_string()
}

/// The question file: the version, the templates, and one lane table.
fn Questions(Lanes:&str) -> String {
	[r#"{"version": 1, "templates": "#, Templates, r#", "lanes": "#, Lanes, "}"].concat()
}

/// A corpus directory on disk, as a corpus is copied somewhere: two files under
/// `Templates/`.
fn Written(Lanes:&str) -> Scratch {
	let Directory = Scratch::New();
	Directory.Write("Templates/Observation.json", Observation);
	Directory.Write("Templates/Questions.json", &Questions(Lanes));
	Directory
}

/// The five-lane corpus, read off a directory.
fn Loaded() -> (Scratch, Corpus) {
	let Directory = Written(&Lanes(Always(), Corner(), Race()));
	let Corpus = Load(Directory.Root()).expect("the five-lane corpus loads");
	(Directory, Corpus)
}

/// A free-roaming tick with the route arrow on screen: no condition holds, so
/// the catch-all answers.
fn Driving() -> Context {
	Context::New()
		.With("speed_kmh", 92)
		.With("gear", 4)
		.With("racetype", 0)
		.With("verdict", "moving")
		.With("route_cue_x", 0.52)
		.With("yaw_rate", 0.05)
}

/// A car turning at speed: the corner lane's own two numbers.
fn Turning() -> Context {
	Context::New()
		.With("speed_kmh", 78)
		.With("gear", 3)
		.With("racetype", 0)
		.With("verdict", "moving")
		.With("route_cue_x", 0.22)
		.With("yaw_rate", 0.42)
}

/// A race in progress: `racetype` is the field the race lane reads.
fn Racing() -> Context {
	Context::New()
		.With("speed_kmh", 141)
		.With("gear", 5)
		.With("racetype", 1)
		.With("verdict", "moving")
		.With("route_cue_x", 0.48)
		.With("yaw_rate", 0.08)
}

/// A car held against something: the verdict is the derived sentence the
/// recovery lane holds on, and it is a field no reading carries.
fn Stuck() -> Context {
	Context::New()
		.With("speed_kmh", 3)
		.With("gear", 1)
		.With("racetype", 0)
		.With("verdict", "blocked")
		.With("situation", "the nose is against a wall")
		.With("moved_m", 0.2)
		.With("blocked_ticks", 5)
		.With("route_cue_x", 0.5)
		.With("yaw_rate", 0.0)
}

/// A car with no route arrow on screen at all: `route_cue_x` is absent, which is
/// what the context lane's negation reads.
fn Uncued() -> Context {
	Context::New()
		.With("speed_kmh", 64)
		.With("gear", 4)
		.With("racetype", 0)
		.With("verdict", "moving")
		.With("road_band_motion", 21)
		.With("yaw_rate", 0.02)
}

#[test]
fn a_five_lane_corpus_loads_off_a_directory_with_its_questions_typed() {
	let (Directory, Corpus) = Loaded();

	assert_eq!(Corpus.Version(), 1);
	assert_eq!(
		Corpus.Lanes().map(|Lane| Lane.Name.as_str()).collect::<Vec<&str>>(),
		vec!["context", "corner", "drive", "race", "recover"],
		"five lanes, sorted by name"
	);
	assert_eq!(Corpus.Templates().count(), 6, "six templates");
	assert_eq!(Corpus.Root().map(PathBuf::as_path), Some(Directory.Root()), "the corpus remembers where it came from");

	// The two halves of the observation, and the union a lane is checked
	// against: a reading and a derived sentence are both sendable.
	assert!(Corpus.Observed("speed_kmh"), "the observation declares a reading");
	assert!(Corpus.Observed("verdict"), "and a derived sentence");
	assert!(!Corpus.Observed("unobserved"), "and nothing nobody declared");
	assert_eq!(Corpus.Derived().count(), 5, "five derived sentences");

	// A lane is a request shape: the fields it sends, in its own order, and the
	// templates it serves in preference order.
	let Drive = Corpus.Lane("drive").expect("the drive lane is declared");
	assert_eq!(
		Drive.Fields,
		vec![
			"speed_kmh".to_string(),
			"gear".to_string(),
			"heading_degrees".to_string(),
			"route_cue_x".to_string(),
			"verdict".to_string(),
			"situation".to_string(),
			"moved_m".to_string()
		]
	);
	assert_eq!(
		Drive.Templates,
		vec!["freeroam.track_cue".to_string(), "freeroam.cruise".to_string()],
		"the lane's order is its preference"
	);
	assert!(!Drive.Cadence.is_empty(), "a lane carries how often it is worth sending");
	assert!(!Drive.Why.is_empty(), "and why it exists");

	// The templates are typed values, not carried text: a choice has its
	// options, a score has its scale, and both are what a request is built from.
	let Cruise = Corpus.Template("freeroam.cruise").expect("the cruise template is declared");
	assert!(Cruise.When.starts_with("free roam"), "the corpus's own sentence is kept: {}", Cruise.When);
	assert_eq!(Cruise.Questions.len(), 2);
	let Question::Choice { Instructions, Criteria } = Cruise.Questions.get("action").expect("the cruise template asks for an action") else {
		panic!("a choice question is a choice");
	};
	assert!(Instructions.contains("target pace"), "the instructions travel verbatim: {Instructions}");
	assert_eq!(
		Criteria.keys().cloned().collect::<Vec<String>>(),
		vec!["brake".to_string(), "hold".to_string(), "throttle".to_string()],
		"every option is carried, sorted by name"
	);
	let Question::Score { Criteria, .. } = Cruise.Questions.get("confidence").expect("and for a confidence") else {
		panic!("a score question is a score");
	};
	assert_eq!(Criteria, &vec!["guessing".to_string(), "unsure".to_string(), "fairly sure".to_string(), "certain".to_string()]);
	assert_eq!(Cruise.Questions["confidence"].Type(), "score");

	// Every lane names only templates the corpus carries: that is the check the
	// loader exists for, so a loaded corpus is a corpus that passed it.
	for Lane in Corpus.Lanes() {
		for Served in &Lane.Templates {
			assert!(Corpus.Template(Served).is_some(), "lane {} serves {Served}, which exists", Lane.Name);
		}
	}
}

#[test]
fn a_missing_observation_file_is_refused_by_its_path() {
	let Directory = Scratch::New();
	Directory.Write("Templates/Questions.json", &Questions(&Lanes(Always(), Corner(), Race())));
	let Expected = Directory.Root().join("Templates/Observation.json");

	let Why = Load(Directory.Root()).expect_err("a corpus with no observation cannot be read");
	let CorpusError::MissingFile { path } = &Why else {
		panic!("a missing file is MissingFile, not {Why:?}");
	};
	assert_eq!(path, &Expected, "the refusal names the file that is absent");
	assert_eq!(Why.Path(), Some(Expected.as_path()), "and answers it as the path it is about");
	assert!(Why.Unreadable(), "a missing file is a corpus that could not be read at all");
	assert!(Why.Message().contains("Observation.json"), "the sentence names it: {}", Why.Message());
	assert!(
		Why.Message().contains(&Expected.display().to_string()),
		"the whole path is in the sentence: {}",
		Why.Message()
	);
}

#[test]
fn a_missing_question_file_is_refused_by_its_path() {
	let Directory = Scratch::New();
	Directory.Write("Templates/Observation.json", Observation);
	let Expected = Directory.Root().join("Templates/Questions.json");

	let Why = Load(Directory.Root()).expect_err("a corpus with no question file cannot be read");
	let CorpusError::MissingFile { path } = &Why else {
		panic!("a missing file is MissingFile, not {Why:?}");
	};
	assert_eq!(path, &Expected, "the refusal names the file that is absent");
}

#[test]
fn a_question_file_that_is_not_json_is_malformed_and_named() {
	let Directory = Scratch::New();
	Directory.Write("Templates/Observation.json", Observation);
	let Expected = Directory.Write("Templates/Questions.json", "{\"version\": 1, \"templates\":");

	let Why = Load(Directory.Root()).expect_err("a file that is not JSON is not a corpus");
	let CorpusError::MalformedJson { path, .. } = &Why else {
		panic!("a broken file is MalformedJson, not {Why:?}");
	};
	assert_eq!(path, &Expected, "the refusal names the file that would not parse");
	assert!(Why.Unreadable(), "a file that is not JSON is unreadable, not invalid");
	assert!(Why.Message().contains("Questions.json"), "{}", Why.Message());
	assert!(
		Why.Detail().contains("column"),
		"the parser's own line and column travel in the detail: {}",
		Why.Detail()
	);
	assert!(
		std::error::Error::source(&Why).is_some(),
		"the parser's error is kept as the cause rather than flattened into a sentence"
	);
}

#[test]
fn a_broken_observation_is_malformed_too_rather_than_an_empty_corpus() {
	let Directory = Scratch::New();
	let Expected = Directory.Write("Templates/Observation.json", "{\"state\": ");
	Directory.Write("Templates/Questions.json", &Questions(&Lanes(Always(), Corner(), Race())));

	let Why = Load(Directory.Root()).expect_err("a broken observation is not an empty corpus");
	let CorpusError::MalformedJson { path, .. } = &Why else {
		panic!("a broken file is MalformedJson, not {Why:?}");
	};
	assert_eq!(path, &Expected, "the observation is the file named, not the question file");
}

#[test]
fn a_lane_that_names_a_template_nobody_wrote_is_refused_at_load() {
	let Directory = Written(&Lanes(Always(), Corner(), Race()).replace("corner.entry", "corner.apex"));

	let Why = Load(Directory.Root()).expect_err("a lane cannot serve a template that does not exist");
	let CorpusError::UnknownTemplateRef { lane, template_id } = &Why else {
		panic!("an unknown template is UnknownTemplateRef, not {Why:?}");
	};
	assert_eq!(lane, "corner", "the refusal names the lane to open");
	assert_eq!(template_id, "corner.apex", "and the id it could not find");
	assert!(!Why.Unreadable(), "the corpus was read and disagrees with itself");
	assert!(Why.Path().is_none(), "a lane naming a template is not about one file");
	assert!(Why.Message().contains("corner.apex"), "{}", Why.Message());
}

#[test]
fn a_lane_that_sends_a_field_nobody_observes_is_refused_at_load() {
	let Directory = Written(&Lanes(Always(), Corner(), Race()).replace("\"yaw_rate\"", "\"yaw_rate_typo\""));

	let Why = Load(Directory.Root()).expect_err("a lane cannot send a field the observation does not carry");
	let CorpusError::UnknownFieldRef { lane, field } = &Why else {
		panic!("an unknown field is UnknownFieldRef, not {Why:?}");
	};
	assert_eq!(lane, "corner", "the refusal names the lane");
	assert_eq!(field, "yaw_rate_typo", "and the field, which is the typo to fix");
}

#[test]
fn each_context_selects_the_one_lane_whose_condition_holds() {
	let (_Keep, Corpus) = Loaded();

	let Drive = SelectLane(&Corpus, &Driving()).expect("a driving context belongs to a lane");
	assert_eq!(Drive.Name, "drive", "nothing more specific holds, so the catch-all answers");

	let Corner = SelectLane(&Corpus, &Turning()).expect("a turning context belongs to a lane");
	assert_eq!(Corner.Name, "corner", "speed and yaw rate say the car is in a turn");

	let Race = SelectLane(&Corpus, &Racing()).expect("a racing context belongs to a lane");
	assert_eq!(Race.Name, "race", "racetype says a race is running");

	let Recover = SelectLane(&Corpus, &Stuck()).expect("a stuck context belongs to a lane");
	assert_eq!(Recover.Name, "recover", "the verdict is a derived field, and a lane may select on one");

	let Context = SelectLane(&Corpus, &Uncued()).expect("an uncued context belongs to a lane");
	assert_eq!(Context.Name, "context", "no route cue on screen is a negation over an absent field");

	// Selection is a function of the corpus and the context alone: no clock, no
	// file system, no global, so the same context replays to the same lane.
	assert_eq!(
		SelectLane(&Corpus, &Racing()).map(|Lane| Lane.Name.clone()),
		Ok("race".to_string()),
		"a recorded context replays to the same selection"
	);
}

#[test]
fn a_context_no_lane_holds_is_refused_rather_than_defaulted() {
	// The catch-all removed: a corpus whose lanes all state a condition, and one
	// context that satisfies none of them.
	let Directory = Written(&Lanes(json!({ "op": "gt", "field": "speed_kmh", "than": 300 }), Corner(), Race()));
	let Corpus = Load(Directory.Root()).expect("a corpus with no catch-all still loads");

	let Why = SelectLane(&Corpus, &Driving()).expect_err("no lane's condition holds");
	let SelectError::NoMatchingLane { Context: Held, .. } = &Why else {
		panic!("nothing holding is NoMatchingLane, not {Why:?}");
	};
	assert!(
		Held.contains("speed_kmh=92"),
		"the context travels with the refusal, so two runs can be compared: {Held}"
	);
	assert!(Why.Message().contains("no lane"), "{}", Why.Message());
	assert!(Why.Lanes().is_empty(), "nothing matched, so no lane is named as a match");
	assert!(
		Why.Detail().contains("speed_kmh=92"),
		"the detail is the context, as one line: {}",
		Why.Detail()
	);
}

#[test]
fn a_context_two_lanes_hold_is_ambiguous_rather_than_first_wins() {
	// Two conditional lanes that both hold: picking one would be this crate
	// guessing at a priority the corpus never stated, and the guess would be
	// invisible in the answer.
	let Directory = Written(&Lanes(
		Always(),
		json!({ "op": "gt", "field": "speed_kmh", "than": 50 }),
		json!({ "op": "in", "field": "racetype", "values": [1] }),
	));
	let Corpus = Load(Directory.Root()).expect("an overlapping corpus still loads");

	let Why = SelectLane(&Corpus, &Racing()).expect_err("two lanes cannot both be the one");
	let SelectError::AmbiguousMatch { Lanes, .. } = &Why else {
		panic!("two lanes holding is AmbiguousMatch, not {Why:?}");
	};
	assert_eq!(
		Lanes,
		&vec!["corner".to_string(), "race".to_string()],
		"every lane that held is named, in the corpus's own order"
	);
	assert_eq!(Why.Lanes().len(), 2, "and answerable as a list");
	assert!(Why.Message().contains("2 lanes"), "{}", Why.Message());
	assert!(
		Why.Detail().contains("corner") && Why.Detail().contains("race"),
		"the detail names them: {}",
		Why.Detail()
	);
}

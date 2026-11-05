//! The real corpus decided end to end: five lanes, five canned replies, no socket.
//!
//! `Test/CorpusLanes.rs` proves that the corpus on disk declares the five lanes
//! and that the policy reaches each of them. This file proves the rest of the
//! pipeline over that same corpus: the selected lane composes its own fields and
//! its own template's questions, the request goes through the [`Poster`] port,
//! and the reply comes back typed - choice, distribution, score, spend and meter
//! - without a socket anywhere in the run.
//!
//! Run it with the table visible:
//! `cargo test -p agentic --test CorpusDecide -- --nocapture`.
//!
//! # Why one canned reply per template, and not one for the whole file
//!
//! Every template the corpus serves asks `action` (a choice) plus a second
//! question that differs by template - `risk` for a race, `severity` for a
//! corner, `urgency` for the free-roam cue, `confidence` for the rest - and the
//! options of `action` are that template's own criteria. A single canned body
//! would answer a lane asking `urgency` with a `confidence` score: the decoder
//! would read it, the test would pass, and both would be describing a reply no
//! endpoint could have sent. So the fixture is a document per template, each one
//! answering exactly the questions that template asks, and each document is read
//! and validated by the same [`Fixture`] reader the crate's own end-to-end target
//! uses - a distribution that does not sum to one is refused before the run, not
//! discovered by a floating-point assertion in the middle of it.
//!
//! A document's `budget` is one, because each run below makes exactly one
//! request: the poster does not enforce the number, the run's own meter does, and
//! a meter configured from the document is what makes the ceiling assertion a
//! fact about the fixture rather than a literal in the test.
//!
//! # Why the poster is included from `Crate/Lane/Test/Fake.rs`
//!
//! It is the workspace's one recording poster and one fixture reader, and
//! `Fake.rs` documents itself as a module test binaries include (`mod Fake;`).
//! Copying either here would be a second reader of the same document shape - the
//! kind of copy that agrees today and drifts later. The path crosses the crate
//! boundary because the poster is test-only by location: it lives beside
//! `Crate/Lane`'s tests rather than in `Source/`, and `Source/` is what ships.
//!
//! # What a failure names
//!
//! Every case prints the lane, the template, the state and the choice before it
//! asserts anything, so a red run says which of the five it was about. The lane
//! and the template are asserted separately rather than as `lane / template`,
//! because a lane serving a different first template is a corpus edit and a lane
//! selected wrongly is a policy edit, and one string would not tell them apart.
//!
//! Nothing here opens a socket: the only I/O is `fs::read_to_string` behind
//! `Load`, and the capability under test declares `"network": false` in its
//! manifest.

#![allow(non_snake_case, non_upper_case_globals)]

#[path = "../../Lane/Test/Fake.rs"]
mod Fake;
mod CorpusRoot;

use std::{collections::BTreeSet, sync::Arc, time::Duration};

use AgenticLane::{Answered, Budget, Create, DecidePort, Options, Poster, Rules, State};
use AgenticRuntime::{BlockOn, Call, Never, Wall};
use CorpusRoot::{Cases, Directory, Line, Policy, Read};
use Fake::{Fixture, FixturePoster};
use serde_json::Value;

/// The canned reply for one template of the corpus, compiled in from
/// `Test/Fixture/CorpusDecide/`.
///
/// Keyed by template rather than by lane: a document answers the questions a
/// template asks, and a lane that served a different template first would need
/// that template's document. A template with no document is a panic naming it,
/// because a silent fallback would answer the wrong questions and pass.
fn Canned(Template:&str) -> &'static str {
	match Template {
		"race.start" => include_str!("Fixture/CorpusDecide/race.start.json"),
		"recovery.blocked" => include_str!("Fixture/CorpusDecide/recovery.blocked.json"),
		"corner.entry" => include_str!("Fixture/CorpusDecide/corner.entry.json"),
		"traffic.ghost" => include_str!("Fixture/CorpusDecide/traffic.ghost.json"),
		"freeroam.track_cue" => include_str!("Fixture/CorpusDecide/freeroam.track_cue.json"),
		Other => {
			panic!(
				"no canned reply for template `{Other}`: Test/Fixture/CorpusDecide/ carries one document per template \
				 the five lanes serve first, and a corpus that reordered its templates needs one more"
			)
		},
	}
}

/// One call's bound, on the wall clock. The fixture answers immediately, so this
/// never expires; it is passed because the port takes a [`Call`] and a run that
/// passed none would not be exercising the signature a caller uses.
fn Bound() -> Call { Call::Within(Duration::from_secs(1), &Wall, Arc::new(Never)) }

/// The model every lane in this corpus falls back to, since none names one.
const Model:&str = "clef-flash";

#[test]
fn every_lane_is_decided_against_the_corpus_with_the_fixture_poster() {
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
	let mut Selected = BTreeSet::new();

	for Case in Cases() {
		// One document, one poster, one decider per case: the document's budget
		// is a request count, and a run that spent it is the run it describes.
		let Poster = FixturePoster::Of(Fixture::Read(Canned(Case.Template)));
		let Fixture = Poster.Fixture();
		let Decider = Create(
			Corpus.clone(),
			Poster.clone() as Arc<dyn Poster>,
			Options {
				Model:Model.to_string(),
				Lanes:Policy(),
				Templates:Rules::default(),
				Budget:Budget::New(Poster.Budget(), 0),
			},
		);
		let State = State::Of(Case.Observation.clone()).expect("a case is a JSON object");

		let Selection = Decider
			.Select(&State)
			.unwrap_or_else(|Why| panic!("case '{}': no lane was selected: {Why}", Case.Name));
		println!(
			"case {:<18} lane {:<10} template {:<22} {}",
			Case.Name,
			Selection.Lane.Name,
			Selection.Template.Name,
			Line(&Case.Observation)
		);
		assert_eq!(
			Selection.Lane.Name,
			Case.Lane,
			"case '{}' selected lane '{}', expected '{}' for state {}",
			Case.Name,
			Selection.Lane.Name,
			Case.Lane,
			Line(&Case.Observation)
		);
		assert_eq!(
			Selection.Template.Name,
			Case.Template,
			"case '{}' selected template '{}', expected '{}' for state {}",
			Case.Name,
			Selection.Template.Name,
			Case.Template,
			Line(&Case.Observation)
		);
		Selected.insert(Selection.Lane.Name.clone());

		// --- the round trip -----------------------------------------------
		let Answer = BlockOn(Decider.Decide(&Selection, &State, Bound())).unwrap_or_else(|Why| {
			panic!(
				"case '{}': the fixture answers without a socket, and this failed: {Why}",
				Case.Name
			)
		});

		let (Choice, Share) = Answer.Choice("action").unwrap_or_else(|| {
			panic!(
				"case '{}': the reply answered no `action`, and every template asks it",
				Case.Name
			)
		});
		let Expected = Fixture
			.Choice("action")
			.unwrap_or_else(|| panic!("case '{}': the canned reply chose no option", Case.Name));
		println!("           choice {Choice} at {Share} · meter {}", Decider.Budget().Line());
		assert_eq!(
			Choice, Expected,
			"case '{}': the typed answer names the option the document chose",
			Case.Name
		);

		let Some(Answered::Choice { Choice: Chosen, Probabilities }) = Answer.Answer("action") else {
			panic!(
				"case '{}': `action` is a choice question in every template, and this is not a choice",
				Case.Name
			);
		};
		assert_eq!(Chosen, Expected);
		assert_eq!(
			Probabilities,
			&Fixture.Probabilities("action"),
			"case '{}': a share is kept for every option the document named, and no option it did not",
			Case.Name
		);
		let Sum:f64 = Probabilities.values().sum();
		assert!(
			(Sum - 1.0).abs() < 1e-6,
			"case '{}': the shares sum to one within 1e-6, saw {Sum}",
			Case.Name
		);
		assert!(
			Probabilities.values().all(|Share| (0.0..=1.0).contains(Share)),
			"case '{}': every share is a probability · {Probabilities:?}",
			Case.Name
		);

		// The reply's options are the corpus's own criteria for `action`, which
		// is the claim a single canned body could not make: an answer spread
		// over options this template never offered is a body no endpoint could
		// have sent, and the decoder has no request in hand to notice it.
		let Template = Corpus
			.Template(Case.Template)
			.unwrap_or_else(|| panic!("case '{}': the corpus carries no template '{}'", Case.Name, Case.Template));
		let Action = Template
			.Questions
			.get("action")
			.unwrap_or_else(|| panic!("case '{}': template '{}' asks no `action`", Case.Name, Case.Template));
		let Offered = Action.Criteria();
		for Option in Probabilities.keys() {
			assert!(
				Offered.contains(Option),
				"case '{}': the reply spread mass over `{Option}`, which `action` does not offer; it offers \
				 {Offered:?}",
				Case.Name
			);
		}

		// --- the composed request -----------------------------------------
		let Posted = Poster.Only();
		let Names = Posted.Names();
		let Sent:Value = serde_json::from_str(&Posted.Text()).expect("the posted body is JSON");
		assert_eq!(
			Sent["model"],
			serde_json::json!(Model),
			"case '{}': the body names the bare model",
			Case.Name
		);
		assert!(
			!Posted.Path().contains("chat"),
			"case '{}': a chat-completions route would answer this body with a 400 · {}",
			Case.Name,
			Posted.Path()
		);

		let Carried:BTreeSet<&str> = Sent["state"]
			.as_object()
			.unwrap_or_else(|| panic!("case '{}': `state` is not an object · {}", Case.Name, Sent))
			.keys()
			.map(String::as_str)
			.collect();
		let Named:BTreeSet<&str> = Selection.Lane.Fields.iter().map(String::as_str).collect();
		assert_eq!(
			Carried, Named,
			"case '{}': the state is the lane's fields and nothing else the observation carried",
			Case.Name
		);

		let Asked:Vec<&str> = Sent["questions"]
			.as_object()
			.unwrap_or_else(|| panic!("case '{}': `questions` is not an object · {}", Case.Name, Sent))
			.keys()
			.map(String::as_str)
			.collect();
		assert_eq!(
			Asked, Names,
			"case '{}': the body's question names are the request's own, not a second description",
			Case.Name
		);
		assert!(
			Names.contains(&"action"),
			"case '{}': the request asks `action`, which is what the reply answers · {Names:?}",
			Case.Name
		);

		// --- the second question, and the spend ---------------------------
		let Second = Names
			.iter()
			.copied()
			.find(|Name| *Name != "action")
			.unwrap_or_else(|| panic!("case '{}': template '{}' asks only one question", Case.Name, Case.Template));
		let ExpectedScore = Fixture.Response()["result"]["answers"][Second]["score"]
			.as_f64()
			.unwrap_or_else(|| panic!("case '{}': the canned reply scored no `{Second}`", Case.Name));
		assert_eq!(
			Answer.Score(Second),
			Some(ExpectedScore),
			"case '{}': the second question is a score, and it is the document's",
			Case.Name
		);

		let Meter = Decider.Budget();
		assert_eq!(
			Meter.MaxRequests,
			Fixture.Budget(),
			"case '{}': the run's ceiling is the fixture's own budget",
			Case.Name
		);
		assert_eq!(Meter.Requests, 1, "case '{}': one round trip is one request", Case.Name);
		assert_eq!(Meter.Failed, 0, "case '{}': nothing failed: {:?}", Case.Name, Meter.Last);
		let Reported = Fixture.Response()["result"]["usage"]["input_tokens"]
			.as_u64()
			.unwrap_or_else(|| panic!("case '{}': the canned reply reports no input tokens", Case.Name));
		assert_eq!(
			Answer.Usage.Input, Reported,
			"case '{}': the answer carries what the reply reported",
			Case.Name
		);
		assert_eq!(
			Meter.Input, Reported,
			"case '{}': and the meter charged exactly that",
			Case.Name
		);
	}

	let Wanted:BTreeSet<String> = CorpusRoot::Expected.iter().map(|Name| Name.to_string()).collect();
	assert_eq!(
		Selected, Wanted,
		"the cases decided {Selected:?}, and all five lanes must be decided end to end"
	);
}

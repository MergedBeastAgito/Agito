//! The real corpus on disk, and the five lanes it declares.
//!
//! Every other test in this workspace answers from a corpus written for the test
//! it sits beside. This one reads the corpus a caller actually drives with - the
//! NFSU2 example's `Templates/` files, reached through the corpus root - because
//! a loader proved only against a fixture written next to it has proved that the
//! two agree, and nothing more.
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
//! # Where the corpus is read from
//!
//! `Test/CorpusRoot.rs` holds the discovery rule, the five expected lanes and
//! the representative states, because `Test/CorpusDecide.rs` needs the same three
//! and a second copy of any of them would be a second answer to "which corpus,
//! and which lane wins". The two places the corpus can be - the sibling checkout,
//! and the copy under `Test/Fixture/CorpusLanes/` read when it is absent - are
//! described there, and
//! [`the_vendored_copy_says_what_the_sibling_checkout_says`] is the test that
//! keeps the copy from becoming a second source of truth.
//!
//! Nothing here opens a socket: the only I/O is `fs::read_to_string`, and the
//! capability under test declares `"network": false` in its manifest.

#![allow(non_snake_case, non_upper_case_globals)]

mod CorpusRoot;

use std::{collections::BTreeSet, fs};

use AgenticLane::{Rules, Select, State};
use CorpusRoot::{Cases, Directory, Expected, Line, Read, Relative, Sibling, Vendored};
use serde_json::{Value, json};

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
fn every_lane_is_selected_by_its_own_condition_from_one_representative_state() {
	let (Dir, _) = Directory();
	let Corpus = Read(&Dir);
	let mut Selected = BTreeSet::new();

	for Case in Cases() {
		let Body = Line(&Case.Observation);
		let State = State::Of(Case.Observation).expect("a case is a JSON object");
		// No rule list: the corpus's own `select` conditions are the policy, which
		// is the claim this test exists to hold.
		let Chosen = Select(&Corpus, &State, &Rules::default(), &Rules::default())
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
fn a_state_no_condition_holds_falls_back_to_drive() {
	let (Dir, _) = Directory();
	let Corpus = Read(&Dir);
	let Body = json!({ "speed_kmh": 12, "gear": 2 });
	let State = State::Of(Body.clone()).expect("the state is a JSON object");
	let Chosen = Select(&Corpus, &State, &Rules::default(), &Rules::default())
		.expect("the corpus carries one catch-all");
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

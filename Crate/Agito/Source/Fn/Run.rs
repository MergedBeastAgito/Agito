//! What each verb does.
//!
//! The two that need no browser are answered from the library alone, so a
//! machine with no Chromium can still check the corpus and every manifest. The
//! three that do need one build a session through [`crate::Fn::Composition`]
//! and hand it to a capability: nothing here calls an adapter directly.

use std::{sync::Arc, time::Duration};

use ::CDP::{OpenOptions, Ownership};
use ::Driver::{Chat, Mode, Question};
use ::Verification::Verify;
use ::RunTime::{Block, Call, Never};
use ::Contract::{Create as CreateContract, SkillContract};
use serde_json::json;

use crate::{
	Fn::{
		Composition::{Browser, Chat as ChatOver, ChatSpec, Design, Headless},
		Discover,
		Resolve,
		Module::Verify::{ReadinessJSON, VerbOf},
	},
	Struct::{
		Binary::{Command::Verb, Shape::Shape},
		Manifest::All,
		Note::Given,
	},
};

/// The variable that names a skill corpus when no root is given.
const SkillVariable:&str = "AGITO_SKILL_PATH";

/// The corpus beside this crate: the repository keeps it two levels up.
const SkillRelative:&str = "../../Documentation/Skill";

/// Where an installed tree keeps the corpus the release ships.
const SkillAbsolute:&str = "/usr/local/share/agito/skills";

/// Run one verb, answering the text to print or the reason it could not be run.
pub async fn Run(Command:Verb) -> Result<String, String> {
	match Command {
		Verb::Manifests { Adapter } => {
			// The workspace's own table first, then whatever the adapter roots
			// declare. The origin column is what keeps the two apart: a row
			// that says `workspace` is compiled into this binary, and a row that
			// names a file was read from it when the question was asked.
			let Roots = Discover::Roots(&Adapter);
			let Foreign = Discover::Discover(&Roots)?;
			let mut Lines:Vec<String> = All
				.iter()
				.map(|Manifest| {
					Row(
						Manifest.Name,
						Manifest.Kind,
						Manifest.Port,
						&Verbs(Manifest.Verbs),
						Manifest.State,
						"workspace",
					)
				})
				.collect();
			Lines.extend(Foreign.iter().map(|One| {
				let Origin = One.Path.display().to_string();
				Row(&One.Name, &One.Kind, &One.Port, &Declared(&One.JSON), &One.State, &Origin)
			}));
			Ok(Lines.join("\n"))
		},
		Verb::Verify { Module, Adapter } => {
			let Roots = Discover::Roots(&Adapter);
			let Readiness = crate::Fn::Module::Verify::Verify(&Module, &Roots)?;
			let Text = serde_json::to_string_pretty(&ReadinessJSON(&Readiness)).unwrap_or_else(|_| "{}".to_string());
			if Readiness.Ready { Ok(Text) } else { Err(Text) }
		},
		Verb::Skills { Root, JSON } => {
			// The argument when there is one, and the three-tier default when
			// there is not: beside this crate, then `AGITO_SKILL_PATH`, then
			// the installed location. The resolved path is what the summary
			// and the JSON report, so a caller can see which corpus was read.
			let Root = match Root {
				Some(Named) => Resolve::Given(std::path::Path::new(&Named)),
				None => Resolve::Default(SkillVariable, SkillRelative, SkillAbsolute),
			};
			let Root = Root.to_string_lossy().into_owned();
			let Contract = CreateContract();
			let Skills = Contract.Discover(&Root).await;
			let mut Findings = Vec::new();
			for Skill in &Skills {
				Findings.extend(Contract.Validate(Skill).await);
			}
			let Digest = Contract.Digest(&Skills);
			if JSON {
				Ok(serde_json::to_string_pretty(&json!({
					"root": Root,
					"digest": Digest,
					"skills": Skills,
					"findings": Findings,
				}))
				.unwrap_or_else(|_| "{}".to_string()))
			} else {
				let mut Lines = vec![format!(
					"{} skill(s) under {Root}, {} finding(s), {Digest}",
					Skills.len(),
					Findings.len()
				)];
				for Finding in &Findings {
					Lines.push(format!("  {} {}: {}", Finding.Skill, Finding.Rule, Finding.Detail));
				}
				Ok(Lines.join("\n"))
			}
		},
		Verb::Browse { URL, Expression, Attach, Bind, Timeout } => {
			let Browse = Browser(Headless());
			let Call = Budget(Timeout);
			let Options = match (Attach, Bind) {
				(Some(Endpoint), Some(Key)) => OpenOptions::Existing(Endpoint, Key, Call.clone()),
				(Some(Endpoint), None) => OpenOptions::Attach(Endpoint, Call.clone()),
				(None, Some(_)) => {
					return Err(
						"binding an existing page needs --attach: a browser this tool launched has no page to bind"
							.to_string(),
					);
				},
				(None, None) => OpenOptions::Launch(Call.clone()),
			};
			let Ownership = Options.Ownership();
			let Target = Browse
				.Open(&URL, Options)
				.await
				.map_err(|Why| format!("{}: {}\n{}", Why.Code().Name(), Why.Message(), Why.Detail()))?;
			let Bound = Target.URL.clone();
			let Value = Browse
				.Evaluate(&Target, &Expression, Call)
				.await
				.map_err(|Why| format!("{}: {}\n{}", Why.Code().Name(), Why.Message(), Why.Detail()))?;
			Browse.Dispose(Target).await;
			Ok(serde_json::to_string_pretty(&json!({
				"url": Bound,
				"ownership": match Ownership {
					Ownership::Launched => "launched",
					Ownership::Attached => "attached",
				},
				"value": Value,
			}))
			.unwrap_or_else(|_| "{}".to_string()))
		},
		Verb::Chat {
			URL,
			Model,
			Question: Text,
			Attach,
			Bind,
			Selectors: Overrides,
			Format: Wanted,
			Thread,
			Title,
			Project,
			Account,
			Captured,
			Observe,
			Timeout,
		} => {
			let mut Selectors = match &Overrides {
				Some(Named) => {
					let File = Resolve::Given(std::path::Path::new(Named));
					let Text = std::fs::read_to_string(&File)
						.map_err(|Why| format!("the selector file {} could not be read: {Why}", File.display()))?;
					let Parsed:serde_json::Value = serde_json::from_str(&Text)
						.map_err(|Why| format!("the selector file {} is not JSON: {Why}", File.display()))?;
					Driver::Struct::Selectors::Merged(&Parsed)
				},
				None => Driver::DefaultSelectors(),
			};
			// The file is a statement about the product; the flag is a
			// statement about this call. The call wins, because a
			// caller who typed it meant it - and a flag that
			// silently lost to a file would capture a whole
			// thread in a format they had already ruled out.
			let Shape = match &Wanted {
				Some(Named) => {
					Shape::From(Named).ok_or_else(|| {
						let Accepted = Shape::All()
							.iter()
							.map(|Shape| Shape::Name(*Shape))
							.collect::<Vec<&str>>()
							.join(", ");
						format!("--format takes {Accepted}, not {Named:?}")
					})?
				},
				// No flag: the selector file's reading is also this call's
				// output shape, which is what every call before `note` did.
				None => Shape::Of(Selectors.Format),
			};
			// A shape decides the reading, not the other way round: `note`
			// reads Markdown whatever the file said, because a note built from
			// rendered text is a note whose formulas are glyphs.
			Selectors.Format = Shape.Reading();
			let Reading = Selectors.Format.Name();
			let Given = Given { Thread, Title, Project, Account, Captured };
			// A note is filed under the thread it came from, and the page's
			// own URL is the only thing that says which one that was. With a
			// bind that URL is not known until the page opens, so this check
			// runs before the question when it can and after the answer when
			// it cannot: a call that cannot file its note must not type into
			// somebody's tab first.
			//
			// `both` carries a note, so it inherits the requirement: a call
			// that would print a note it could not file is refused before it
			// types, exactly as `note` is.
			if Shape.CarriesNote()
				&& Given.Thread.as_deref().is_none_or(|Thread| Thread.trim().is_empty())
				&& Bind.is_none()
				&& crate::Fn::Note::ThreadOf(&URL).is_none()
			{
				return Err(crate::Fn::Note::Unfiled(&URL));
			}
			let Driver = ChatOver(Browser(Headless()), ChatSpec { URL:URL.clone(), Attach, Bind, Selectors });
			let Call = Budget(Timeout);
			// An observer is a sink and a stop, the way a mutation observer is
			// a callback and a disconnect: the driver pushes every
			// read here and this decides when it has seen enough.
			// `--observe` writes each one to stderr, so stdout
			// stays the answer a caller is parsing.
			let mut Seen = |Observation:&Driver::Observation| {
				if Observe {
					eprintln!(
						"{}",
						serde_json::to_string(&json!({
							"count": Observation.Count,
							"chars": Observation.Text.chars().count(),
							"busy": Observation.Busy,
							"fresh": Observation.Fresh,
							"settled": Observation.Settled,
						}))
						.unwrap_or_else(|_| "{}".to_string())
					);
				}
				Driver::Control::Continue
			};
			let Answer = Driver
				.Observe(Question { Text, Mode:Mode::Search, Model }, Call, &mut Seen)
				.await
				.map_err(|Why| format!("{}: {}\n{}", Why.Code().Name(), Why.Message(), Why.Detail()))?;
			// A note is the whole of stdout: a caller redirecting the call
			// gets a file a reader opens, not a document inside a JSON string.
			if Shape == Shape::Note {
				return crate::Fn::Note::Note(&Answer, Given);
			}
			// The pair shape prints two renderings and the document built from
			// the second, and it adds them to the same JSON rather than
			// printing twice: one call, one document, and every fact in it
			// came from the one read the driver took. Nothing is written -
			// stdout is the sink, and a caller that wants two files splits
			// them.
			let mut Out = Envelope(&Answer, Reading);
			if Shape == Shape::Both {
				let Note = crate::Fn::Note::Note(&Answer, Given)?;
				if let Some(Object) = Out.as_object_mut() {
					Object.insert("markdown".to_string(), json!(Answer.Markdown));
					Object.insert("note".to_string(), json!(Note));
				}
			}
			Ok(serde_json::to_string_pretty(&Out).unwrap_or_else(|_| "{}".to_string()))
		},
		Verb::Design { URL, Selectors, Timeout } => {
			let Verifier = Design(Browser(Headless()), URL.clone());
			let Measurements = Verifier
				.Measure(&Selectors, Budget(Timeout))
				.await
				.map_err(|Why| format!("{}: {}", Why.Code().Name(), Why.Message()))?;
			Ok(serde_json::to_string_pretty(&json!({
				"url": URL,
				"measurements": Measurements.iter().map(|Measurement| json!({
					"selector": Measurement.Selector,
					"present": Measurement.Present,
					"value": Measurement.Value,
				})).collect::<Vec<_>>(),
			}))
			.unwrap_or_else(|_| "{}".to_string()))
		},
	}
}

/// One row of the manifest listing: name, kind, port, verbs, state and where
/// it came from.
///
/// The origin is the last column because the difference between "this binary
/// carries it" and "this file said so" is the difference between a claim that
/// was compiled and a claim that was read.
fn Row(Name:&str, Kind:&str, Port:&str, Verbs:&str, State:&str, Origin:&str) -> String {
	format!("{Name:<32} {Kind:<12} {Port:<24} {Verbs:<20} {State:<16} {Origin}")
}

/// The verbs column for a module the table carries: its names, or `-` when it
/// declares none.
///
/// `-` rather than an empty cell because a column of blanks reads as a value
/// that failed to load, and "this module declares no verb" is a fact.
fn Verbs(Names:&[&str]) -> String { if Names.is_empty() { "-".to_string() } else { Names.join(",") } }

/// The verbs column for a module read from a file: what its own bytes declare,
/// or `?` when that declaration is not the shape `verify` reads.
///
/// A listing shows the module rather than hiding it, the way it shows one whose
/// state is missing; `verify` is where a malformed declaration becomes a reason.
/// But it must not print a verb the file did not declare, so the marker is `?`
/// and never `-`.
fn Declared(JSON:&str) -> String {
	let Ok(Parsed) = serde_json::from_str::<serde_json::Value>(JSON) else {
		return "?".to_string();
	};
	match VerbOf(&Parsed) {
		Ok(Names) => Verbs(&Names.iter().map(String::as_str).collect::<Vec<_>>()),
		Err(_) => "?".to_string(),
	}
}

/// The JSON a `text` or a `markdown` call prints, and the base of the one a
/// `both` call prints.
///
/// One builder rather than two literals, because the two documents differ by
/// the keys a pair read adds and by nothing else: a second literal would let a
/// fact be added to one shape and forgotten in the other, and the two would
/// then describe different calls.
fn Envelope(Answer:&Driver::Answer, Reading:&str) -> serde_json::Value {
	json!({
		// The page's own URL, not the one this call was given: with a bind
		// they are different, and the one that answered is the one a caller
		// recording provenance needs.
		"url": Answer.URL,
		"format": Reading,
		"text": Answer.Text,
		"model": Answer.Model,
		"mode": Answer.Mode,
		"trace": Answer.Trace.iter().map(|Step| json!({
			"depth": Step.Depth,
			"label": Step.Label,
			"detail": Step.Detail,
		})).collect::<Vec<_>>(),
		"sources": Answer.Sources.iter().map(|Source| json!({
			"url": Source.URL,
			"label": Source.Label,
		})).collect::<Vec<_>>(),
	})
}

/// One call's budget, on the wall clock.
///
/// The composition root is the one place a wall clock is read: a capability is
/// handed a deadline, never a clock of its own.
fn Budget(Milliseconds:u64) -> Call {
	Call::Within(Duration::from_millis(Milliseconds), &CDP::Tokio, Arc::new(Never))
}

/// Drive a future to completion, for the two verbs that are pure.
///
/// A caller that already has an executor should `await` instead; this exists so
/// a test can call a verb without starting a runtime.
pub fn Drive<Work:std::future::Future>(Work:Work) -> Work::Output { Block(Work) }

//! What each verb does.
//!
//! The two that need no browser are answered from the library alone, so a
//! machine with no Chromium can still check the corpus and every manifest. The
//! three that do need one build a session through [`crate::Fn::Composition`]
//! and hand it to a capability: nothing here calls an adapter directly.

use std::{sync::Arc, time::Duration};

use AgenticBrowserCDP::{OpenOptions, Ownership};
use AgenticChatDriver::{Chat, Mode, Question};
use AgenticDesignVerification::Verify;
use AgenticRuntime::{BlockOn, Call, Never};
use AgenticSkillContract::{Create as CreateContract, SkillContract};
use serde_json::json;

use crate::{
	Fn::{
		Composition::{Browser, Chat as ChatOver, ChatSpec, Design, Headless},
		Discover,
		VerifyModule::{ReadinessJSON, VerifyModule},
	},
	Struct::{Binary::Command::Verb, Manifest::All},
};

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
				.map(|Manifest| Row(Manifest.Name, Manifest.Kind, Manifest.Port, Manifest.State, "workspace"))
				.collect();
			Lines.extend(Foreign.iter().map(|One| {
				let Origin = One.Path.display().to_string();
				Row(&One.Name, &One.Kind, &One.Port, &One.State, &Origin)
			}));
			Ok(Lines.join("\n"))
		},
		Verb::Verify { Module, Adapter } => {
			let Roots = Discover::Roots(&Adapter);
			let Readiness = VerifyModule(&Module, &Roots)?;
			let Text = serde_json::to_string_pretty(&ReadinessJSON(&Readiness)).unwrap_or_else(|_| "{}".to_string());
			if Readiness.Ready { Ok(Text) } else { Err(Text) }
		},
		Verb::Skills { Root, Json } => {
			let Contract = CreateContract();
			let Skills = Contract.Discover(&Root).await;
			let mut Findings = Vec::new();
			for Skill in &Skills {
				Findings.extend(Contract.Validate(Skill).await);
			}
			let Digest = Contract.Digest(&Skills);
			if Json {
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
		Verb::Browse { Url, Expression, Attach, Bind, Timeout } => {
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
				.Open(&Url, Options)
				.await
				.map_err(|Why| format!("{}: {}\n{}", Why.Code().Name(), Why.Message(), Why.Detail()))?;
			let Bound = Target.Url.clone();
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
			Url,
			Model,
			Question: Text,
			Attach,
			Bind,
			Selectors: Overrides,
			Format: Wanted,
			Observe,
			Timeout,
		} => {
			let mut Selectors = match &Overrides {
				Some(Path) => {
					let Text = std::fs::read_to_string(Path)
						.map_err(|Why| format!("the selector file {Path} could not be read: {Why}"))?;
					let Parsed:serde_json::Value = serde_json::from_str(&Text)
						.map_err(|Why| format!("the selector file {Path} is not JSON: {Why}"))?;
					AgenticChatDriver::Struct::Selectors::Merged(&Parsed)
				},
				None => AgenticChatDriver::DefaultSelectors(),
			};
			// The file is a statement about the product; the flag is a
			// statement about this call. The call wins, because a
			// caller who typed it meant it - and a flag that
			// silently lost to a file would capture a whole
			// thread in a format they had already ruled out.
			if let Some(Named) = &Wanted {
				match AgenticChatDriver::Format::From(Named) {
					Some(Format) => Selectors.Format = Format,
					None => {
						let Accepted = AgenticChatDriver::Format::All()
							.iter()
							.map(|Format| Format.Name())
							.collect::<Vec<&str>>()
							.join(" or ");
						return Err(format!("--format takes {Accepted}, not {Named:?}"));
					},
				}
			}
			let Reading = Selectors.Format.Name();
			let Driver = ChatOver(Browser(Headless()), ChatSpec { Url:Url.clone(), Attach, Bind, Selectors });
			let Call = Budget(Timeout);
			// An observer is a sink and a stop, the way a mutation observer is
			// a callback and a disconnect: the driver pushes every
			// read here and this decides when it has seen enough.
			// `--observe` writes each one to stderr, so stdout
			// stays the answer a caller is parsing.
			let mut Seen = |Observation:&AgenticChatDriver::Observation| {
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
				AgenticChatDriver::Control::Continue
			};
			let Answer = Driver
				.Observe(Question { Text, Mode:Mode::Search, Model }, Call, &mut Seen)
				.await
				.map_err(|Why| format!("{}: {}\n{}", Why.Code().Name(), Why.Message(), Why.Detail()))?;
			Ok(serde_json::to_string_pretty(&json!({
				"url": Url,
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
					"url": Source.Url,
					"label": Source.Label,
				})).collect::<Vec<_>>(),
			}))
			.unwrap_or_else(|_| "{}".to_string()))
		},
		Verb::Design { Url, Selectors, Timeout } => {
			let Verifier = Design(Browser(Headless()), Url.clone());
			let Measurements = Verifier
				.Measure(&Selectors, Budget(Timeout))
				.await
				.map_err(|Why| format!("{}: {}", Why.Code().Name(), Why.Message()))?;
			Ok(serde_json::to_string_pretty(&json!({
				"url": Url,
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

/// One row of the manifest listing: name, kind, port, state and where it came
/// from.
///
/// The origin is the fifth column because the difference between "this binary
/// carries it" and "this file said so" is the difference between a claim that
/// was compiled and a claim that was read.
fn Row(Name:&str, Kind:&str, Port:&str, State:&str, Origin:&str) -> String {
	format!("{Name:<32} {Kind:<12} {Port:<24} {State:<16} {Origin}")
}

/// One call's budget, on the wall clock.
///
/// The composition root is the one place a wall clock is read: a capability is
/// handed a deadline, never a clock of its own.
fn Budget(Milliseconds:u64) -> Call {
	Call::Within(Duration::from_millis(Milliseconds), &AgenticBrowserCDP::Tokio, Arc::new(Never))
}

/// Drive a future to completion, for the two verbs that are pure.
///
/// A caller that already has an executor should `await` instead; this exists so
/// a test can call a verb without starting a runtime.
pub fn Drive<Work:std::future::Future>(Work:Work) -> Work::Output { BlockOn(Work) }

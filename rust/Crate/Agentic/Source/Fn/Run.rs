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
		Composition::{Browser, Chat as ChatOver, Design, Headless},
		VerifyModule::{ReadinessJSON, VerifyModule},
	},
	Struct::{Binary::Command::Verb, Manifest::All},
};

/// Run one verb, answering the text to print or the reason it could not be run.
pub async fn Run(Command:Verb) -> Result<String, String> {
	match Command {
		Verb::Manifests => {
			Ok(All
				.iter()
				.map(|Manifest| {
					format!(
						"{:<32} {:<12} {:<24} {}",
						Manifest.Name, Manifest.Kind, Manifest.Port, Manifest.State
					)
				})
				.collect::<Vec<String>>()
				.join("\n"))
		},
		Verb::Verify { Module } => {
			let Readiness = VerifyModule(&Module)?;
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
		Verb::Browse { Url, Expression, Attach, Timeout } => {
			let Browse = Browser(Headless());
			let Call = Budget(Timeout);
			let Options = match Attach {
				Some(Endpoint) => OpenOptions::Attach(Endpoint, Call.clone()),
				None => OpenOptions::Launch(Call.clone()),
			};
			let Ownership = Options.Ownership();
			let Target = Browse
				.Open(&Url, Options)
				.await
				.map_err(|Why| format!("{}: {}", Why.Code().Name(), Why.Message()))?;
			let Value = Browse
				.Evaluate(&Target, &Expression, Call)
				.await
				.map_err(|Why| format!("{}: {}", Why.Code().Name(), Why.Message()))?;
			Browse.Dispose(Target).await;
			Ok(serde_json::to_string_pretty(&json!({
				"url": Url,
				"ownership": match Ownership {
					Ownership::Launched => "launched",
					Ownership::Attached => "attached",
				},
				"value": Value,
			}))
			.unwrap_or_else(|_| "{}".to_string()))
		},
		Verb::Chat { Url, Model, Question: Text, Timeout } => {
			let Driver = ChatOver(Browser(Headless()), Url.clone());
			let Call = Budget(Timeout);
			let Answer = Driver
				.Ask(Question { Text, Mode:Mode::Search, Model }, Call)
				.await
				.map_err(|Why| format!("{}: {}", Why.Code().Name(), Why.Message()))?;
			Ok(serde_json::to_string_pretty(&json!({
				"url": Url,
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

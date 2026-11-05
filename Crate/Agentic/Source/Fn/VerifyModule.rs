//! One module's readiness, from the manifest it ships.
//!
//! The check is the interesting part: a manifest that names four errors while
//! the crate's own type names three is a manifest that has drifted from its
//! code, and a harness reading it would prepare the wrong thing. Comparing the
//! two is what makes `provides.errors` a claim rather than a comment, which is
//! the job the TypeScript contract tests did and the job this verb does for the
//! Rust tree.
//!
//! # Two reports, one shape
//!
//! A module this binary carries can be checked against its own code: the table
//! says one thing, the file says another, and the crate's `Code` type says a
//! third, so all three are compared. A third-party adapter cannot - this binary
//! has never seen its code - and pretending otherwise would be the one thing a
//! readiness report must not do. So the foreign report compares the manifest
//! against what is knowable: the kind vocabulary, the four state classes, the
//! shape of its error list, and whether the ports it requires are ports somebody
//! here provides. `code` is `null` in that report rather than a copy of the
//! manifest's own list, because a copy would read as agreement.
//!
//! # Verbs are a claim too
//!
//! `provides.verbs` is the other half of a module's surface: the chat driver
//! answers `agentic.chat.ask`, and `agentic.chat.note` is a call it also
//! accepts. A verb declared in a manifest that nothing reads is a promise made
//! to nobody, so both reports carry the names the file declares, and a shipped
//! module's names are compared with the table's the way its port and its error
//! codes are. The shape is checked rather than assumed - a `provides.verbs` that
//! is not an object keyed by non-empty verb names is refused, in the same shape
//! as a kind outside the vocabulary.
//!
//! What a foreign report cannot answer, the value can: an adapter states its own
//! port, and `AgenticAdapter::Serves` compares that statement with the manifest
//! that registered it. `agentic verify` never constructs an adapter - a command
//! that did would run a third party's code to decide whether it is well formed.

use std::{collections::BTreeSet, path::PathBuf};

use AgenticBrowserCDP::Code as BrowserCode;
use AgenticChatDriver::Code as ChatCode;
use AgenticDesignVerification::Code as DesignCode;
use AgenticLane::Code as LaneCode;
use AgenticRuntime::Code as RuntimeCode;
use AgenticSkillContract::Rule as SkillRule;
use AgenticSpacetimeCatalog::Code as CatalogCode;
use serde_json::{Value, json};

use crate::{
	Fn::Discover,
	Struct::{
		External::External,
		Kind::{Known, Names},
		Manifest::{All, Find, Manifest},
	},
};

/// What one module reports about itself.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Readiness {
	/// The module's logical name.
	pub Module:String,
	/// What kind of thing it is.
	pub Kind:String,
	/// The port it provides.
	pub Port:String,
	/// The verbs it declares under `provides.verbs`, in the order its
	/// manifest names them. Empty for a module that declares none.
	pub Verbs:Vec<String>,
	/// The retention class it declares.
	pub State:String,
	/// The errors the manifest names.
	pub Manifest:Vec<String>,
	/// The codes the crate's own type declares, or `None` when this binary
	/// cannot see the crate - which is the case for every third-party module.
	pub Code:Option<Vec<String>>,
	/// Whether the manifest was read from a file rather than carried by this
	/// binary.
	pub External:bool,
	/// Whether every check that applies to it passed.
	pub Ready:bool,
	/// Why it is not ready, when it is not.
	pub Reason:Option<String>,
}

/// The codes each crate's type declares, by module name.
///
/// A crate with no error vocabulary of its own - the workflow, the composition
/// root and the adapter port - declares an empty list, and its manifest must
/// too. For the adapter port that is not an omission: a code names a substrate
/// failure, and the port has no substrate; the adapter that reports one declares
/// it in its own manifest.
pub fn Declared(Name:&str) -> Option<Vec<&'static str>> {
	Some(match Name {
		"@agentic/runtime" => RuntimeCode::All().into_iter().map(RuntimeCode::Name).collect(),
		"@agentic/spacetime-catalog" => CatalogCode::All().into_iter().map(CatalogCode::Name).collect(),
		"@agentic/skill-contract" => SkillRule::All().into_iter().map(SkillRule::Name).collect(),
		"@agentic/chat-driver" => ChatCode::All().into_iter().map(ChatCode::Name).collect(),
		"@agentic/browser-cdp" => BrowserCode::All().into_iter().map(BrowserCode::Name).collect(),
		"@agentic/design-verification" => DesignCode::All().into_iter().map(DesignCode::Name).collect(),
		"@agentic/lane" => LaneCode::All().into_iter().map(LaneCode::Name).collect(),
		"@agentic/workflow" | "@agentic/composition" | "@agentic/adapter" => Vec::new(),
		_ => return None,
	})
}

/// The states the manifest may name.
fn States() -> Vec<&'static str> {
	AgenticRuntime::States
		.into_iter()
		.map(AgenticRuntime::StateClass::Name)
		.collect()
}

/// A string field of a parsed manifest, or the empty string.
fn Text(Parsed:&Value, Key:&str) -> String { Parsed.get(Key).and_then(Value::as_str).unwrap_or_default().to_string() }

/// The port a manifest says it provides.
fn Provides(Parsed:&Value) -> String {
	Parsed
		.get("provides")
		.and_then(|Provides| Provides.get("port"))
		.and_then(Value::as_str)
		.unwrap_or_default()
		.to_string()
}

/// The errors a manifest names, in the order it names them.
fn Errors(Parsed:&Value) -> Vec<String> {
	Parsed
		.get("provides")
		.and_then(|Provides| Provides.get("errors"))
		.and_then(Value::as_array)
		.map(|Errors| Errors.iter().filter_map(Value::as_str).map(str::to_string).collect())
		.unwrap_or_default()
}

/// The verbs a manifest declares under `provides`, in the order it names them.
///
/// The shape on disk is an object keyed by the verb's own name, because a verb
/// carries an error list the way a port does: `agentic.chat.note` is not
/// `agentic.chat.ask`, and a code the verb reports belongs to the verb. So the
/// names are the object's keys, and declaring none is the ordinary case -
/// every module but the chat driver declares no verb at all - which is why an
/// absent key answers an empty list rather than an error.
///
/// A `provides.verbs` that is not that shape is refused here rather than
/// reported as a verb no caller could ever name: a bare string, a list of
/// values, a declaration that is not an object, or a key that is empty or
/// blank. The caller turns the `Err` into the reason a module is not ready,
/// which is the same shape as a kind outside the vocabulary.
pub fn VerbsOf(Parsed:&Value) -> Result<Vec<String>, String> {
	let Some(Provides) = Parsed.get("provides") else {
		return Ok(Vec::new());
	};
	match Provides.get("verbs") {
		None | Some(Value::Null) => Ok(Vec::new()),
		Some(Value::Object(Verbs)) => {
			let mut Names = Vec::with_capacity(Verbs.len());
			for (Name, Declaration) in Verbs {
				if Name.trim().is_empty() {
					return Err("names a verb with no name in `provides.verbs`".to_string());
				}
				if !Declaration.is_object() {
					return Err(format!("declares the verb {Name:?} and its declaration is not an object"));
				}
				Names.push(Name.clone());
			}
			Ok(Names)
		},
		Some(_) => Err("`provides.verbs` is not an object keyed by verb name".to_string()),
	}
}

/// The ports a manifest says it requires.
fn Requires(Parsed:&Value) -> Vec<String> {
	Parsed
		.get("requires")
		.and_then(|Requires| Requires.get("ports"))
		.and_then(Value::as_array)
		.map(|Ports| Ports.iter().filter_map(Value::as_str).map(str::to_string).collect())
		.unwrap_or_default()
}

/// Report one module: a module this workspace ships, or one an adapter root
/// declares.
pub fn VerifyModule(Name:&str, Roots:&[PathBuf]) -> Result<Readiness, String> {
	if let Some(Shipped) = Find(Name) {
		return Report(Shipped);
	}
	let Foreign = Discover::Discover(Roots)?;
	let Some(One) = Foreign.iter().find(|One| One.Name == Name) else {
		let mut Known:Vec<&str> = All.iter().map(|One| One.Name).collect();
		Known.extend(Foreign.iter().map(|One| One.Name.as_str()));
		Known.sort_unstable();
		return Err(format!(
			"{Name} is not a module this workspace ships and no adapter root declares it; try one of: {}. A \
			 third-party module is found through --adapter or {}.",
			Known.join(", "),
			Discover::Variable
		));
	};
	ReportForeign(One, &Ports(&Foreign))
}

/// A module this binary carries: the table, the file and the crate's own type
/// must agree.
fn Report(One:&'static Manifest) -> Result<Readiness, String> {
	let Some(Code) = Declared(One.Name) else {
		return Err(format!("{} has no code vocabulary in this binary", One.Name));
	};
	let Parsed:Value = serde_json::from_str(One.JSON)
		.map_err(|Why| format!("{} ships a manifest that does not parse: {Why}", One.Name))?;
	let ManifestErrors = Errors(&Parsed);
	let State = Text(&Parsed, "state");
	let FileName = Text(&Parsed, "name");
	let FileKind = Text(&Parsed, "kind");
	let FilePort = Provides(&Parsed);
	let Verbs = VerbsOf(&Parsed);
	let FileVerbs:Vec<&str> = Verbs
		.as_ref()
		.map(|List| List.iter().map(String::as_str).collect())
		.unwrap_or_default();
	let Declared:Vec<String> = Code.iter().map(|Code| (*Code).to_string()).collect();
	let Reason = if FileName != One.Name {
		Some(format!(
			"the table calls this module {} and its file calls it {FileName}",
			One.Name
		))
	} else if FileKind != One.Kind {
		Some(format!(
			"the table calls this kind {} and its file calls it {FileKind}",
			One.Kind
		))
	} else if FilePort != One.Port {
		Some(format!("the table names the port {} and its file names {FilePort}", One.Port))
	} else if !Known(One.Kind) {
		Some(format!("{:?} is not one of {}", One.Kind, Names()))
	} else if let Err(Why) = &Verbs {
		Some(Why.clone())
	} else if FileVerbs != One.Verbs {
		Some(format!(
			"the table names the verbs [{}] and its file names [{}]",
			One.Verbs.join(", "),
			FileVerbs.join(", ")
		))
	} else if ManifestErrors != Declared {
		Some(format!(
			"the manifest names [{}] and the crate names [{}]",
			ManifestErrors.join(", "),
			Declared.join(", ")
		))
	} else if !States().contains(&State.as_str()) {
		Some(format!("{State:?} is not one of the four state classes"))
	} else {
		None
	};
	Ok(Readiness {
		Module:One.Name.to_string(),
		Kind:One.Kind.to_string(),
		Port:One.Port.to_string(),
		Verbs:Verbs.unwrap_or_default(),
		State,
		Manifest:ManifestErrors,
		Code:Some(Declared),
		External:false,
		Ready:Reason.is_none(),
		Reason,
	})
}

/// A module read from a file: everything a manifest can be held to without its
/// code.
fn ReportForeign(One:&External, Ports:&[String]) -> Result<Readiness, String> {
	let Parsed:Value = serde_json::from_str(&One.JSON)
		.map_err(|Why| format!("{} ships a manifest that does not parse: {Why}", One.Path.display()))?;
	let ManifestErrors = Errors(&Parsed);
	let Verbs = VerbsOf(&Parsed);
	let Requires = Requires(&Parsed);
	let Entry = Parsed
		.get("artefact")
		.and_then(|Artefact| Artefact.get("entry"))
		.and_then(Value::as_str)
		.unwrap_or_default()
		.to_string();
	let Missing:Vec<&str> = Requires
		.iter()
		.map(String::as_str)
		.filter(|Port| !Ports.iter().any(|Known| Known == Port))
		.collect();
	let Distinct = ManifestErrors.iter().collect::<BTreeSet<&String>>().len() == ManifestErrors.len();
	let Reason = if !Known(&One.Kind) {
		Some(format!("{:?} is not one of {}", One.Kind, Names()))
	} else if let Err(Why) = &Verbs {
		Some(Why.clone())
	} else if !States().contains(&One.State.as_str()) {
		Some(format!("{:?} is not one of the four state classes", One.State))
	} else if ManifestErrors.iter().any(|Code| Code.trim().is_empty()) {
		Some("names an empty error code".to_string())
	} else if !Distinct {
		Some(format!("names a code twice: [{}]", ManifestErrors.join(", ")))
	} else if !Missing.is_empty() {
		Some(format!(
			"requires [{}], which neither this workspace nor any module under the adapter roots provides",
			Missing.join(", ")
		))
	} else if Entry.is_empty() {
		Some("names no `artefact.entry`".to_string())
	} else {
		None
	};
	Ok(Readiness {
		Module:One.Name.clone(),
		Kind:One.Kind.clone(),
		Port:One.Port.clone(),
		Verbs:Verbs.unwrap_or_default(),
		State:One.State.clone(),
		Manifest:ManifestErrors,
		Code:None,
		External:true,
		Ready:Reason.is_none(),
		Reason,
	})
}

/// Every port this binary can name: the ones it ships and the ones the adapter
/// roots declare.
fn Ports(Foreign:&[External]) -> Vec<String> {
	All.iter()
		.map(|One| One.Port.to_string())
		.chain(Foreign.iter().map(|One| One.Port.clone()))
		.collect()
}

/// A readiness report as the JSON a harness reads.
pub fn ReadinessJSON(Readiness:&Readiness) -> Value {
	json!({
		"module": Readiness.Module,
		"kind": Readiness.Kind,
		"port": Readiness.Port,
		"verbs": Readiness.Verbs,
		"state": Readiness.State,
		"ready": Readiness.Ready,
		"external": Readiness.External,
		"errors": { "manifest": Readiness.Manifest, "code": Readiness.Code },
		"reason": Readiness.Reason,
	})
}

/// Every manifest, as the summary `agentic manifests` prints.
pub fn Catalogue() -> Vec<Manifest> { All.to_vec() }

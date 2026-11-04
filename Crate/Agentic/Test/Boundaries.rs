//! The arrows, asserted against the tree rather than remembered.
//!
//! This is the rule `Scripts/Boundaries.mjs` enforced over the TypeScript
//! packages, read from the Rust half after those packages were deleted. A
//! crate's kind comes from its `module.json`; the edges come from its
//! `Cargo.toml`; and the rule is the one the README states - a capability may
//! import a capability or the runtime, an adapter may import a capability or
//! the runtime, the workflow, the composition root and the runtime may import
//! anything, a facade may import any port, only the composition root may import
//! a facade, and nothing may import the composition root.
//!
//! Reading the two files rather than `cargo tree` is deliberate: `cargo tree`
//! resolves features, needs a lock and a registry, and answers a question about
//! a build, while these are the two statements a reviewer reads. A crate with
//! no readable manifest is a failure here rather than a skip, because a
//! boundary check that passes over a crate it could not read is a check that
//! has stopped covering the tree without saying so.

#![allow(non_snake_case, non_upper_case_globals)]

use std::{
	collections::BTreeMap,
	fs,
	path::{Path, PathBuf},
};

/// The kinds a manifest may declare, and the only inputs the rule accepts.
///
/// A kind outside this list is not a crate with an unusual role: it is a
/// manifest this file cannot reason about, and it is refused rather than read
/// as "anything goes".
const Kinds:[&str; 6] = ["runtime", "capability", "adapter", "workflow", "composition", "facade"];

/// One crate, as the two files that describe it.
struct Module {
	/// The directory under `Crate/`, for a failure that names a path.
	Directory:PathBuf,
	/// The Cargo package name, which is what a dependency edge names.
	Package:String,
	/// The kind its manifest declares.
	Kind:String,
}

/// The workspace root, from this crate's own manifest directory.
fn root() -> PathBuf {
	Path::new(env!("CARGO_MANIFEST_DIR"))
		.join("../..")
		.canonicalize()
		.expect("the workspace root is reachable from Crate/Agentic")
}

/// The name of the section a line opens, if it opens one.
fn section(Line:&str) -> Option<String> {
	let Trimmed = Line.trim();
	let Inner = Trimmed.strip_prefix('[')?.strip_suffix(']')?;
	// `[[test]]` is an array of tables and names no dependency section; the
	// strip above leaves `[test]`, which is not one either.
	Some(Inner.trim_start_matches('[').trim_end_matches(']').to_string())
}

/// The `[package] name`, which is the identity a dependency edge names.
fn package_name(Manifest:&str) -> Option<String> {
	let mut Inside = false;
	for Line in Manifest.lines() {
		if let Some(Name) = section(Line) {
			Inside = Name == "package";
			continue;
		}
		if Inside
			&& let Some(Value) = Line.trim().strip_prefix("name")
			&& let Some(Value) = Value.trim_start().strip_prefix('=')
		{
			return Some(Value.trim().trim_matches('"').to_string());
		}
	}
	None
}

/// Every `agentic-*` or `agentic` dependency the crate declares, in any of the
/// three dependency sections.
///
/// The keys are compared against the workspace's own package names by the
/// caller, so a dependency on a third-party crate is read and ignored rather
/// than guessed at.
fn dependencies(Manifest:&str) -> Vec<String> {
	let mut Inside = false;
	let mut Found = Vec::new();
	for Line in Manifest.lines() {
		if let Some(Name) = section(Line) {
			Inside = matches!(Name.as_str(), "dependencies" | "dev-dependencies" | "build-dependencies");
			continue;
		}
		if !Inside {
			continue;
		}
		let Trimmed = Line.trim();
		if Trimmed.starts_with('#') || Trimmed.is_empty() {
			continue;
		}
		if let Some((Key, _)) = Trimmed.split_once('=') {
			Found.push(Key.trim().trim_matches('"').to_string());
		}
	}
	Found
}

/// Every crate in the workspace, with the kind and the package name it
/// declares.
fn modules() -> Vec<Module> {
	let Directory = root().join("Crate");
	let mut Entries:Vec<PathBuf> = fs::read_dir(&Directory)
		.expect("Crate/ is readable")
		.filter_map(Result::ok)
		.map(|Entry| Entry.path())
		.filter(|Path| Path.is_dir())
		.collect();
	Entries.sort();
	let mut Found = Vec::new();
	for One in Entries {
		let Manifest = One.join("module.json");
		let Text =
			fs::read_to_string(&Manifest).unwrap_or_else(|Why| panic!("{} is readable: {Why}", Manifest.display()));
		let Parsed:serde_json::Value =
			serde_json::from_str(&Text).unwrap_or_else(|Why| panic!("{} is JSON: {Why}", Manifest.display()));
		let Kind = Parsed
			.get("kind")
			.and_then(serde_json::Value::as_str)
			.unwrap_or_else(|| panic!("{} names no kind", Manifest.display()))
			.to_string();
		let Cargo = One.join("Cargo.toml");
		let Text = fs::read_to_string(&Cargo).unwrap_or_else(|Why| panic!("{} is readable: {Why}", Cargo.display()));
		let Package = package_name(&Text).unwrap_or_else(|| panic!("{} names no package", Cargo.display()));
		Found.push(Module { Directory:One, Package, Kind });
	}
	Found
}

/// Whether an edge from one kind to another is one the design allows.
///
/// The rule is total: a kind this file does not know is refused in either
/// position rather than waved through.
/// `every_crate_declares_a_kind_the_rule_knows` is the case that says why the
/// tree is broken when that happens; this one makes sure the answer is never
/// "allowed" on the way there.
fn allowed(Importer:&str, Imported:&str) -> bool {
	if !Kinds.contains(&Importer) || !Kinds.contains(&Imported) {
		return false;
	}
	// A facade is the outward edge of the tree: it names the ports a consumer
	// would otherwise name, so it may import any of them, and it is the one
	// thing a capability must not reach for - a crate that imported the facade
	// would be importing its own re-export, and the surface would grow a second
	// arrow pointing outward. Only the composition root, which publishes that
	// surface, may name it.
	if Imported == "facade" {
		return Importer == "composition";
	}
	match Importer {
		// The runtime holds no workspace and the composition root binds ports to
		// adapters, so both may name anything; a workflow composes capabilities
		// and owns the bindings, so it may too, and a facade re-exports ports it
		// does not implement.
		"runtime" | "workflow" | "composition" | "facade" => true,
		// A capability declares the traits it needs and nothing else: an adapter
		// in its dependency list would make it untestable without the provider.
		"capability" => matches!(Imported, "capability" | "runtime"),
		// An adapter implements one port against one mechanism, so it may name a
		// capability's port shapes - never another adapter, which would make the
		// substrate depend on a second substrate.
		"adapter" => matches!(Imported, "capability" | "runtime"),
		_ => false,
	}
}

/// Every edge, and what the rule says about it.
fn edges() -> (Vec<Module>, BTreeMap<String, String>, Vec<String>) {
	let Modules = modules();
	let KindsByPackage:BTreeMap<String, String> = Modules
		.iter()
		.map(|Module| (Module.Package.clone(), Module.Kind.clone()))
		.collect();
	let mut Problems = Vec::new();
	for Module in &Modules {
		let Cargo = Module.Directory.join("Cargo.toml");
		let Text = fs::read_to_string(&Cargo).expect("a crate's manifest is readable");
		for Dependency in dependencies(&Text) {
			if Dependency == Module.Package {
				continue;
			}
			let Some(Imported) = KindsByPackage.get(&Dependency) else {
				continue;
			};
			if !allowed(&Module.Kind, Imported) {
				Problems.push(format!(
					"{}/Cargo.toml: {} imports {Dependency} ({Imported})",
					Module.Directory.strip_prefix(root()).unwrap_or(&Module.Directory).display(),
					Module.Kind
				));
			}
		}
	}
	(Modules, KindsByPackage, Problems)
}

/// Every edge between the workspace's own crates points inward.
#[test]
fn every_arrow_points_inward() {
	let (_, _, Problems) = edges();
	assert!(Problems.is_empty(), "a dependency reverses an arrow:\n{}", Problems.join("\n"));
}

/// Nothing depends on the composition root, which is what makes it a root.
#[test]
fn nothing_imports_the_composition_root() {
	let Modules = modules();
	let Roots:Vec<&Module> = Modules.iter().filter(|Module| Module.Kind == "composition").collect();
	assert_eq!(Roots.len(), 1, "the workspace ships exactly one composition root");
	let Root = &Roots[0];
	let mut Problems = Vec::new();
	for Module in &Modules {
		if Module.Package == Root.Package {
			continue;
		}
		let Text = fs::read_to_string(Module.Directory.join("Cargo.toml")).expect("a manifest is readable");
		if dependencies(&Text).contains(&Root.Package) {
			Problems.push(format!("{} depends on the composition root {}", Module.Package, Root.Package));
		}
	}
	assert!(Problems.is_empty(), "{}", Problems.join("\n"));
}

/// Every crate declares one of the kinds the rule knows how to read.
///
/// A kind the rule does not name falls through `allowed` to `false`, which
/// would report every one of its edges as a violation and read as a defect in
/// the crate rather than in this file. Naming the kinds here keeps that failure
/// legible.
#[test]
fn every_crate_declares_a_kind_the_rule_knows() {
	let Modules = modules();
	assert!(!Modules.is_empty(), "Crate/ holds no module.json at all");
	for Module in &Modules {
		assert!(
			Kinds.contains(&Module.Kind.as_str()),
			"{} declares the unknown kind {:?}",
			Module.Package,
			Module.Kind
		);
	}
}

/// The check has something to check: the tree's own edges are read, not
/// invented.
#[test]
fn the_workspace_has_the_edges_the_rule_reads() {
	let (Modules, ..) = edges();
	let Edges = Modules
		.iter()
		.flat_map(|Module| {
			let Text = fs::read_to_string(Module.Directory.join("Cargo.toml")).expect("a manifest is readable");
			dependencies(&Text)
		})
		.filter(|Dependency| Modules.iter().any(|Module| &Module.Package == Dependency))
		.count();
	assert!(
		Edges > 0,
		"no crate in this workspace depends on another; the check is reading nothing"
	);
}

/// The rule, and the parser that feeds it, on inputs this file owns.
///
/// The tree above is correct today, which is also what a rule that has been
/// inverted reports. These cases pin the direction: the two edges the design
/// forbids are refused, the ones it allows are not, and the manifest reader
/// finds a dependency in each of the three sections a crate may declare one in.
mod rule {
	use super::{allowed, dependencies, package_name};

	#[test]
	fn a_capability_may_not_import_an_adapter() {
		assert!(!allowed("capability", "adapter"));
		assert!(!allowed("capability", "workflow"));
		assert!(!allowed("capability", "composition"));
		assert!(allowed("capability", "capability"));
		assert!(allowed("capability", "runtime"));
	}

	#[test]
	fn an_adapter_may_not_import_another_adapter() {
		assert!(!allowed("adapter", "adapter"));
		assert!(allowed("adapter", "capability"));
		assert!(allowed("adapter", "runtime"));
	}

	#[test]
	fn the_outer_three_may_import_anything() {
		for Importer in ["runtime", "workflow", "composition"] {
			for Imported in ["runtime", "capability", "adapter", "workflow", "composition"] {
				assert!(allowed(Importer, Imported), "{Importer} -> {Imported}");
			}
		}
	}

	#[test]
	fn an_unknown_kind_is_refused_rather_than_assumed() {
		assert!(!allowed("plugin", "runtime"));
		assert!(!allowed("runtime", "plugin"));
	}

	#[test]
	fn a_facade_names_any_port_and_only_the_root_names_the_facade() {
		for Imported in ["runtime", "capability", "adapter", "workflow"] {
			assert!(allowed("facade", Imported), "facade -> {Imported}");
		}
		assert!(allowed("composition", "facade"));
		for Importer in ["runtime", "capability", "adapter", "workflow", "facade"] {
			assert!(!allowed(Importer, "facade"), "{Importer} -> facade");
		}
	}

	#[test]
	fn the_manifest_reader_finds_the_package_and_every_dependency_section() {
		let Manifest = "\
[package]
name = \"agentic-probe\"

[lib]
name = \"Probe\"

[dependencies]
agentic-runtime = { workspace = true }
serde_json = \"1\"

[dev-dependencies]
agentic-skill-contract = { workspace = true }

[build-dependencies]
agentic-browser-cdp = { workspace = true }
";
		assert_eq!(package_name(Manifest).as_deref(), Some("agentic-probe"));
		let Found = dependencies(Manifest);
		assert!(Found.contains(&"agentic-runtime".to_string()), "{Found:?}");
		assert!(Found.contains(&"serde_json".to_string()), "{Found:?}");
		assert!(Found.contains(&"agentic-skill-contract".to_string()), "{Found:?}");
		assert!(Found.contains(&"agentic-browser-cdp".to_string()), "{Found:?}");
	}
}

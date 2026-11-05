//! Where a third-party module's manifest is found.
//!
//! The workspace's own table is compiled in. Everything else has to be named,
//! and there are exactly two ways to name it: `--adapter` on the command line,
//! and `AGENTIC_ADAPTER_PATH` when the flag is absent. A path list, not a
//! registry: nothing here is loaded, nothing is constructed, and the order is
//! the order of the directories rather than the order of somebody's side effect.
//!
//! Two levels are read. A root may be a crate directory - it has a
//! `module.json` - or a directory of crate directories. Deeper than that would
//! make the answer depend on a tree the caller did not describe, and a tool that
//! walks further than it was told is a tool whose listing changes when an
//! unrelated directory appears.
//!
//! A root that does not exist is skipped with a warning, and a `module.json` that
//! exists but does not parse is a refusal. The asymmetry is the point: absence is
//! not a claim - a stale entry in a path list must not break the workspace's own
//! listing - while a file that was found and is wrong is exactly what a reader
//! must not pass over.

use std::{
	collections::{BTreeMap, BTreeSet},
	env,
	ffi::OsStr,
	fs,
	path::{Path, PathBuf},
};

use crate::{
	Fn::Resolve,
	Struct::{External::External, Manifest::All},
};

/// The variable a caller uses when it does not pass `--adapter`.
pub const Variable:&str = "AGENTIC_ADAPTER_PATH";

/// The directories to read: the flag when it was given, the environment
/// otherwise.
///
/// Both are resolved through [`Resolve::Given`], so a relative entry means the
/// same directory whatever the process was started in.
pub fn Roots(Given:&[String]) -> Vec<PathBuf> {
	if !Given.is_empty() {
		return Given.iter().map(|One| Resolve::Given(Path::new(One))).collect();
	}
	// `var_os` rather than `var`: a path list is a list of paths, and a path is
	// not required to be UTF-8. A value that is not readable as text is still a
	// value somebody set, and dropping it silently would make the listing depend
	// on the encoding of an unrelated directory name.
	FromValue(env::var_os(Variable).as_deref())
}

/// The directories a path-list value names.
///
/// Separate from [`Roots`] so the parse can be exercised without setting a
/// process-wide variable: `std::env::set_var` is unsafe, and a test that reached
/// for it would be a test that races every other test in the binary.
pub fn FromValue(Value:Option<&OsStr>) -> Vec<PathBuf> {
	let Some(Value) = Value else {
		return Vec::new();
	};
	env::split_paths(Value)
		.filter(|One| !One.as_os_str().is_empty())
		.map(|One| Resolve::Given(&One))
		.collect()
}

/// Every third-party module under these roots, in path order.
///
/// Two modules with one name, or a module claiming a name the workspace already
/// ships, is an ambiguity a manifest exists to prevent: a caller asking for that
/// name would get one of two answers and no way to tell which.
pub fn Discover(Roots:&[PathBuf]) -> Result<Vec<External>, String> {
	let mut Paths:BTreeSet<PathBuf> = BTreeSet::new();
	for Root in Roots {
		if !Root.exists() {
			eprintln!("agentic: the adapter root {} does not exist; skipping it", Root.display());
			continue;
		}
		if !Root.is_dir() {
			eprintln!("agentic: the adapter root {} is not a directory; skipping it", Root.display());
			continue;
		}
		let Own = Root.join("module.json");
		if Own.is_file() {
			Paths.insert(Canonical(&Own));
		}
		let Entries = fs::read_dir(Root)
			.map_err(|Why| format!("the adapter root {} could not be read: {Why}", Root.display()))?;
		for Entry in Entries {
			let Entry = Entry.map_err(|Why| format!("the adapter root {} could not be read: {Why}", Root.display()))?;
			let Candidate = Entry.path().join("module.json");
			if Candidate.is_file() {
				Paths.insert(Canonical(&Candidate));
			}
		}
	}
	let mut Found = Vec::new();
	for Path in Paths {
		Found.push(External::Read(&Path)?);
	}
	let mut Seen:BTreeMap<&str, &Path> = BTreeMap::new();
	for One in &Found {
		if All.iter().any(|Shipped| Shipped.Name == One.Name) {
			return Err(format!(
				"{} declares the name {}, which this workspace already ships",
				One.Path.display(),
				One.Name
			));
		}
		if let Some(First) = Seen.insert(&One.Name, &One.Path) {
			return Err(format!(
				"{} and {} both declare the name {}",
				First.display(),
				One.Path.display(),
				One.Name
			));
		}
	}
	Ok(Found)
}

/// A path as the filesystem spells it, so one file named two ways is read once.
fn Canonical(Path:&Path) -> PathBuf { fs::canonicalize(Path).unwrap_or_else(|_| Path.to_path_buf()) }

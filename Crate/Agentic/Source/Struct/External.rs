//! A module this workspace does not ship, read from its own `module.json`.
//!
//! The built-in table is compiled in: the bytes the binary reports are the bytes
//! that were checked in, and no file has to exist at run time. A third-party
//! adapter cannot be compiled in - not being compiled in is what `kind:
//! "adapter"` means - so it is read, and the difference is kept visible in the
//! type rather than smoothed over. An [`External`] is a file that was on disk
//! when the question was asked; a `Manifest` is a claim the binary carries.
//!
//! Reading is the only place a manifest's kind is decided. A file that names no
//! name, no port, or a kind outside [`crate::Struct::Kind::Kinds`] is refused
//! here rather than listed and doubted later: a listing that shows a module the
//! verifier cannot reason about is worse than a listing that says why it would
//! not.

use std::path::{Path, PathBuf};

use serde_json::Value;

use crate::Struct::Kind::{Known, Names};

/// One module read from a file, with the fields a listing and a report need.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct External {
	/// The name its manifest declares.
	pub Name:String,
	/// The kind its manifest declares, already checked against the vocabulary.
	pub Kind:String,
	/// The port its manifest says it provides.
	pub Port:String,
	/// The retention class its manifest declares. Empty when it names none,
	/// which the readiness report refuses rather than this reader: a listing
	/// can still show a module whose state is missing.
	pub State:String,
	/// The manifest's bytes, so the readiness report reads the file it names
	/// rather than a paraphrase of it.
	pub JSON:String,
	/// Where it was read from, for a failure that has to name a path.
	pub Path:PathBuf,
}

impl External {
	/// Read one `module.json`, refusing a file this workspace cannot reason
	/// about.
	pub fn Read(Path:&Path) -> Result<Self, String> {
		let JSON =
			std::fs::read_to_string(Path).map_err(|Why| format!("{} could not be read: {Why}", Path.display()))?;
		let Parsed:Value =
			serde_json::from_str(&JSON).map_err(|Why| format!("{} is not JSON: {Why}", Path.display()))?;
		let Text = |Key:&str| Parsed.get(Key).and_then(Value::as_str).unwrap_or_default().to_string();
		let Name = Text("name");
		if Name.is_empty() {
			return Err(format!("{} names no module in `name`", Path.display()));
		}
		let Kind = Text("kind");
		if !Known(&Kind) {
			return Err(format!("{Name} declares the kind {Kind:?}, which is not one of {}", Names()));
		}
		let Port = Parsed
			.get("provides")
			.and_then(|Provides| Provides.get("port"))
			.and_then(Value::as_str)
			.unwrap_or_default()
			.to_string();
		if Port.is_empty() {
			return Err(format!("{Name} names no port in `provides.port`"));
		}
		Ok(Self { Name, Kind, Port, State:Text("state"), JSON, Path:Path.to_path_buf() })
	}
}

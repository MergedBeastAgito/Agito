//! Reading a directory, and the names this contract gives the files it reads.
//!
//! A root that cannot be read is an empty corpus rather than an error, because
//! a sweep that dies on one bad path cannot report the rest. A caller that must
//! tell a typo from an empty corpus stats the root itself - the port does not
//! pretend to know which of the two a missing directory was.

use std::{fs, path::Path};

/// The file a skill's frontmatter lives in.
pub const BodyFile:&str = "SKILL.md";

/// The index a skill's `Reference/` carries.
pub const IndexFile:&str = "Index.md";

/// The directory a skill's worked instances live in.
pub const ReferenceDirectory:&str = "Reference";

/// The corpus root's index, by the two names a corpus actually gives it.
///
/// `Index.md` is the port's own name, and the one it prefers when both exist.
/// `AGENTS.md` is the name a repository already gives the file its tools
/// discover, and a corpus that grows inside such a repository should not have
/// to rename it to be read - one file with two accepted names is cheaper than a
/// corpus that is uncited in its entirety. A skill's own `Reference/Index.md`
/// has one name only: there the card names the file.
pub const CorpusIndexes:[&str; 2] = [IndexFile, "AGENTS.md"];

/// One entry of a directory, as much of it as this contract reads.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Entry {
	/// The entry's name, without its directory.
	pub Name:String,
	/// Whether the entry is a directory without following a link.
	pub Directory:bool,
	/// Whether the entry is a symbolic link.
	pub Symlink:bool,
	/// Whether the entry is a regular file without following a link.
	pub File:bool,
}

/// The entries of a directory, or none.
pub fn Entries(Path:&Path) -> Vec<Entry> {
	let Ok(Read) = fs::read_dir(Path) else {
		return Vec::new();
	};
	let mut Entries = Vec::new();
	for Entry in Read.flatten() {
		let Ok(Kind) = Entry.file_type() else {
			continue;
		};
		let Name = Entry.file_name().to_string_lossy().into_owned();
		Entries.push(Entry { Name, Directory:Kind.is_dir(), Symlink:Kind.is_symlink(), File:Kind.is_file() });
	}
	Entries.sort_by(|Left, Right| Left.Name.cmp(&Right.Name));
	Entries
}

/// A file's text, or none. An unreadable file reads as absent for the reason
/// above.
pub fn Text(Path:&Path) -> Option<String> { fs::read_to_string(Path).ok() }

/// Whether an entry is a directory, following a symlink one level.
pub fn IsDirectory(Entry:&Entry, Path:&Path) -> bool {
	if Entry.Directory {
		return true;
	}
	if !Entry.Symlink {
		return false;
	}
	fs::metadata(Path).map(|Meta| Meta.is_dir()).unwrap_or(false)
}

/// Whether an entry is a file or a link to one.
pub fn IsFile(Entry:&Entry) -> bool { Entry.File || Entry.Symlink }

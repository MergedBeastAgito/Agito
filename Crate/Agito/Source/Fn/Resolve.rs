//! Where a path is read from.
//!
//! One rule, stated once and used everywhere this binary opens something: a
//! path is tried beside the crate that named it first, an environment override
//! is consulted next, and an absolute location is the last resort. The order is
//! deliberate. A tree that ships with its own corpus should read that corpus
//! whatever directory the process was started in, an operator who moves the
//! corpus should not have to rebuild the binary, and a hard-coded absolute path
//! is the one answer that cannot be true on two machines.
//!
//! [`Default`] is the whole of that order, for a path nothing named. [`Given`]
//! is the caller's half of it: what somebody typed is never silently replaced
//! by a default, so an absolute path is taken as written, a relative one is read
//! beside this crate when that is where it points, and otherwise it keeps the
//! working-directory meaning a shell gives it.

use std::{
	env,
	path::{Path, PathBuf},
};

/// This crate's own directory, as the compiler saw it.
///
/// `CARGO_MANIFEST_DIR` is resolved at compile time, so it names the directory
/// the crate was built from. For a binary built in a checkout that is the
/// checkout; for one built by a CI runner it is that runner's scratch
/// directory, which is why every caller of [`Default`] treats a path that does
/// not exist as absent rather than as an answer.
const Home:&str = env!("CARGO_MANIFEST_DIR");

/// A path a caller named, read relative-first and absolute-last.
///
/// An absolute path is the caller's own statement and is returned unchanged. A
/// relative one is first tried beside this crate - `--adapter ../SomeAdapter`
/// then means the same directory whatever the working directory is - and then
/// as given, which is the reading a shell has always given it.
pub fn Given(Named:&Path) -> PathBuf {
	if Named.is_absolute() {
		return Named.to_path_buf();
	}
	let Beside = Path::new(Home).join(Named);
	if Beside.exists() {
		return Beside.canonicalize().unwrap_or(Beside);
	}
	Named.to_path_buf()
}

/// A path nothing named: beside the crate, then the environment, then the
/// installed absolute location.
///
/// `Relative` is joined to this crate's directory and returned when it exists,
/// `Variable` is read when it does not, and `Absolute` is returned otherwise -
/// whether or not it exists, so the refusal a caller sees names the location
/// that was looked for rather than an empty string.
pub fn Default(Variable:&str, Relative:&str, Absolute:&str) -> PathBuf {
	let Beside = Path::new(Home).join(Relative);
	if Beside.exists() {
		return Beside.canonicalize().unwrap_or(Beside);
	}
	if let Some(Value) = env::var_os(Variable).filter(|Value| !Value.is_empty()) {
		return PathBuf::from(Value);
	}
	PathBuf::from(Absolute)
}

//! What a bind presents: the generation it supersedes and the manifest it pins.

/// The manifest a composition root binds.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Manifest {
	/// The composition root's name for the manifest. Opaque here: the digest is
	/// what the manifest is called, and recomputing one over an unspecified
	/// canonical form would be a second convention to keep.
	pub Digest:String,
	/// The capability ids the manifest names. Each must be declared.
	pub Capabilities:Vec<String>,
}

/// One bind.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Bind {
	/// The generation this one supersedes, or `None` for the first.
	pub Previous:Option<String>,
	/// The manifest being bound.
	pub Manifest:Manifest,
}

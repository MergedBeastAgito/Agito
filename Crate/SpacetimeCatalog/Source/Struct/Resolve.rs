//! What a resolver states: which quarantined resource, and what happened to the
//! effects that quarantined it.

/// What the resolver found: the external action landed, or it was absent.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Outcome {
	/// The external action did happen.
	Landed,
	/// The external action did not happen, or its absence was verified
	/// afterwards.
	Absent,
}

impl Outcome {
	/// The string the trace payload carries.
	pub const fn Name(self) -> &'static str {
		match self {
			Outcome::Landed => "landed",
			Outcome::Absent => "absent",
		}
	}
}

/// One answer to "what happened to the effect that made this resource
/// ambiguous".
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Resolve {
	/// The quarantined resource.
	pub Resource:String,
	/// The generation the resolver is answering under.
	pub Generation:String,
	/// What the resolver found.
	pub Outcome:Outcome,
}

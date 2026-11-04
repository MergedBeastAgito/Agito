//! What a claim presents: a resource, the generation, the attempt, and a ttl.

/// A claim on one resource for one attempt.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Claim {
	/// The resource id: a browser session, a task, a workspace.
	pub Resource:String,
	/// The generation the claim is made under, which must be the latest.
	pub Generation:String,
	/// The attempt the lease is for.
	pub Task:String,
	/// How long the lease lasts, in the module's clock unit. Must be positive.
	pub Ttl:i64,
}

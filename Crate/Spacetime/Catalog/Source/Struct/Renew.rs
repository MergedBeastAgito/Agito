//! What a renewal presents: the fence the lease was granted with, and a ttl.

/// A renewal of a lease the caller already holds.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Renew {
	/// The resource the lease is over.
	pub Resource:String,
	/// The generation the lease was granted under.
	pub Generation:String,
	/// The fence the lease was granted with. A stale fence is refused.
	pub Fence:i64,
	/// How much longer the lease should last, in the module's clock unit. Must
	/// be positive.
	pub Ttl:i64,
}

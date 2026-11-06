//! What an effect presents when it lands: the epoch that admitted it, and its
//! own name for what it did.

/// One effect asking to be recorded at the epoch that admitted it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Effect {
	/// The generation the effect was admitted under.
	pub Generation:String,
	/// The resource whose lease admitted it.
	pub Resource:String,
	/// The fence the resource was at when it was admitted.
	pub Fence:i64,
	/// The effect's own name for what it did.
	pub Name:String,
}

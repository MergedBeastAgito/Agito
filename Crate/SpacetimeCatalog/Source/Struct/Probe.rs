//! What a reveal presents: a property version, at the fence the caller holds.

/// One reveal: a property version, at the fence the caller holds.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Probe {
	/// The generation the reveal is made under.
	pub Generation:String,
	/// The resource whose lease the prober holds.
	pub Resource:String,
	/// The fence the lease was granted with.
	pub Fence:i64,
	/// The property being probed.
	pub Property:String,
	/// The property version the disclosure is pinned to. Must be at least one.
	pub Version:i64,
	/// What the owner permits this audience to learn. Never the private value.
	pub Observed:String,
}

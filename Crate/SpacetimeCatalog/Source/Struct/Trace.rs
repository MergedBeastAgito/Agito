//! What a trace presents: one event, keyed by its instant, kind and subject.

/// One appended event.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Trace {
	/// The generation the event happened under.
	pub Generation:String,
	/// What happened; a closed set the module owns.
	pub Kind:String,
	/// What it happened to - a resource, a generation, a property.
	pub Subject:String,
	/// Opaque JSON. Never a private value, and never executable.
	pub Payload:String,
}

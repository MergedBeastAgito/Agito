//! Compile `Source/Script/**/*.ts` into the JavaScript this crate sends.
//!
//! The rule for what a script may contain and how its type syntax is erased
//! lives in `Build/Script.rs`, shared with `Crate/Workflow`, so there is one
//! statement of it rather than one per crate.

include!("../../Build/Script.rs");

fn main() {
	CompileScripts("Source/Script", "Script");
}

//! Compile `Source/Script/**/*.ts` into the JavaScript this crate sends.
//!
//! The same shared rule `Crate/ChatDriver` uses - see `Build/Script.rs` - so
//! the workflow's measurement expression is compiled, checked and filled
//! exactly like the driver's expressions.

include!("../../Build/Script.rs");

fn main() {
	CompileScripts("Source/Script", "Script");
}

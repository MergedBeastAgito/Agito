//! The executable entry point.
//!
//! One `#[tokio::main]`, and everything after it is the library. The runtime is
//! started here and nowhere else: a capability that started its own would be a
//! capability that decides which executor runs the process, and the whole point
//! of the pace being supplied is that it does not.

use std::process::ExitCode;

use Agito::Struct::Binary::Command::Command;

#[tokio::main]
async fn main() -> ExitCode {
	let Command = Command::Parse();
	match Agito::Run(Command.Verb).await {
		Ok(Text) => {
			println!("{Text}");
			ExitCode::SUCCESS
		},
		Err(Why) => {
			eprintln!("{Why}");
			ExitCode::FAILURE
		},
	}
}

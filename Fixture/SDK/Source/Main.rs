//! A consumer of `agentic-sdk`, and of no other crate of this workspace.
//!
//! It performs the flow the facade exists for: build a session, open a page,
//! evaluate an expression, dispose. Every name in it comes from `agentic-sdk` -
//! `Browse`, `Call`, `Chrome`, `ChromeOptions`, `Create`, `Error`, `Never`,
//! `OpenOptions`, `Options`, `Report`, `Target`, `Tokio` - and the only other
//! dependency is `tokio`, because the browser adapter spawns its handler on it.
//! `serde_json` is not named either: the evaluated value is read with `as_str`
//! and `Display`, so the fixture never has to know what type carries it.
//!
//! The page is a checked-in file rather than a network address, so the assertion
//! is about the expression and the session and not about what a server answered
//! today. `AGENTIC_BROWSER` names the binary to drive; without it the well-known
//! paths are tried, and a machine with none of them fails by name.

use std::{path::PathBuf, sync::Arc, time::Duration};

use AgenticSdk::{
	Browse,
	Call,
	Chrome,
	ChromeOptions,
	Create,
	Error,
	Never,
	OpenOptions,
	Options,
	Report,
	Target,
	Tokio,
};

/// A call with a deadline, on the browser's own clock.
fn Budget(Seconds:u64) -> Call { Call::Within(Duration::from_secs(Seconds), &Tokio, Arc::new(Never)) }

/// The browser binary to drive.
fn Browser() -> PathBuf {
	if let Some(Named) = std::env::var_os("AGENTIC_BROWSER") {
		return PathBuf::from(Named);
	}
	let Candidates = [
		"/Applications/Brave Browser.app/Contents/MacOS/Brave Browser",
		"/Applications/Brave Browser Nightly.app/Contents/MacOS/Brave Browser Nightly",
		"/Applications/Google Chrome.app/Contents/MacOS/Google Chrome",
	];
	for Candidate in Candidates {
		let Path = PathBuf::from(Candidate);
		if Path.is_file() {
			return Path;
		}
	}
	panic!("no browser to drive: set AGENTIC_BROWSER to one of {}", Candidates.join(", "));
}

/// The page this fixture opens, as a `file:` URL.
fn PageUrl() -> String {
	let Path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("Page/Probe.html");
	assert!(Path.is_file(), "the fixture page is not at {}", Path.display());
	format!("file://{}", Path.to_string_lossy())
}

#[tokio::main]
async fn main() -> Result<(), Error> {
	// Where a dispose failure goes, because `Dispose` returns `()` and must not
	// fail: a caller that already has its answer must not lose it to a failing
	// cleanup.
	let Reporter:Report = Arc::new(|Target:&Target, Why:&Error| eprintln!("dispose failed on {}: {Why}", Target.Ws));
	let Session = Create(
		Chrome::New(ChromeOptions { Executable:Some(Browser()), Headless:true, NoSandbox:false }),
		Arc::new(Tokio),
		Options { Report:Reporter, ..Options::default() },
	);
	let Bounded = Budget(30);
	let Url = PageUrl();
	let Opened = Session.Open(&Url, OpenOptions::Launch(Bounded.clone())).await?;
	let Value = Session
		.Evaluate(&Opened, "document.querySelector('#probe').textContent", Bounded)
		.await?;
	let Answer = Value.as_str().map(str::to_owned).unwrap_or_else(|| Value.to_string());
	Session.Dispose(Opened).await;
	if Answer != "42" {
		return Err(Error::Navigation(
			format!("the page answered {Answer:?} where the fixture says 42"),
			"the expression read the text content of #probe".to_string(),
		));
	}
	println!("evaluated: {Answer}");
	Ok(())
}

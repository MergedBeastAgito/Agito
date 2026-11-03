//! The wiring: the only place that names a concrete adapter.
//!
//! Three builders, one per thing that needs a browser. Each returns a port,
//! never the adapter behind it, so a caller cannot reach past the composition
//! root into `chromiumoxide`.
//!
//! The pace is the tokio one because this binary is a tokio program. A
//! capability never sees it: it is handed a `Pace` through the driver and the
//! verifier, which is what keeps their dependency lists free of an executor.

use std::sync::Arc;

use AgenticBrowserCDP::{Browse, Chrome, ChromeOptions, Create as CreateSession, Options, Report, Tokio};
use AgenticChatDriver::{Create as CreateChat, Driver, Options as ChatOptions, Page as ChatPage};
use AgenticDesignVerification::{Create as CreateVerify, Verifier};
use AgenticWorkflow::Struct::Binding::{Measure, Page};

/// A browser session with the workspace's pace, reporting a failed release to
/// stderr.
///
/// The reporter writes to stderr rather than stdout because a caller that is
/// piping the tool's answer must not find a diagnostic in the middle of it.
pub fn Browser(Chrome:ChromeOptions) -> Arc<dyn Browse> {
	let Report:Report = Arc::new(|Target, Why| {
		eprintln!("browser-cdp: dispose of {} failed: {}", Target.Url, Why.Message());
	});
	Arc::new(CreateSession(
		Chrome::New(Chrome),
		Arc::new(Tokio),
		Options { Report, ..Options::default() },
	))
}

/// A chat driver over a browser, with the workflow's page binding installed.
pub fn Chat(Browse:Arc<dyn Browse>, Url:impl Into<String>) -> Driver {
	let Page = Arc::new(Page::Create(Browse));
	CreateChat(Page as Arc<dyn ChatPage>, ChatOptions::New(Url), Arc::new(Tokio))
}

/// A design verifier over a browser, with the workflow's measurement binding
/// installed.
pub fn Design(Browse:Arc<dyn Browse>, Url:impl Into<String>) -> Verifier {
	CreateVerify(Arc::new(Measure::Create(Browse, Url)), Arc::new(Tokio))
}

/// What a caller passes to `Browser` when it wants a visible browser on the
/// machine's own display.
pub fn Visible() -> ChromeOptions { ChromeOptions { Executable:None, Headless:false, NoSandbox:false } }

/// What a caller passes to `Browser` when it wants a headless one.
pub fn Headless() -> ChromeOptions { ChromeOptions { Executable:None, Headless:true, NoSandbox:false } }

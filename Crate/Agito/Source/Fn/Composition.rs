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

use ::CDP::{Browse, Chrome, ChromeOptions, Create as CreateSession, Options, Report, Tokio};
use ::Driver::{Create as CreateChat, Driver, Options as ChatOptions, Page as ChatPort, Selectors};
use ::Verification::{Create as CreateVerify, Verifier};
use ::Workflow::Struct::Binding::{Measure, Page};

/// A browser session with the workspace's pace, reporting a failed release to
/// stderr.
///
/// The reporter writes to stderr rather than stdout because a caller that is
/// piping the tool's answer must not find a diagnostic in the middle of it.
pub fn Browser(Chrome:ChromeOptions) -> Arc<dyn Browse> {
	let Report:Report = Arc::new(|Target, Why| {
		eprintln!("browser-cdp: dispose of {} failed: {}", Target.URL, Why.Message());
	});
	Arc::new(CreateSession(
		Chrome::New(Chrome),
		Arc::new(Tokio),
		Options { Report, ..Options::default() },
	))
}

/// Where a chat driver's page comes from, and how it is read.
pub struct ChatSpec {
	/// The chat product's URL. With a bind it is only a fallback: the page
	/// keeps whatever URL it was already showing.
	pub URL:String,
	/// Attach to this endpoint instead of launching a browser.
	pub Attach:Option<String>,
	/// Bind this page on the attached browser instead of opening one.
	pub Bind:Option<String>,
	/// How the page is read.
	pub Selectors:Selectors,
}

/// A chat driver over a browser, with the workflow's page binding installed.
///
/// Three cases, stated rather than inferred: no endpoint launches a browser
/// this session owns; an endpoint with no bind opens a page in somebody else's
/// browser; an endpoint with a bind drives the page that is already there.
pub fn Chat(Browse:Arc<dyn Browse>, Spec:ChatSpec) -> Driver {
	let Page = match &Spec.Attach {
		Some(Endpoint) => Page::Attached(Browse, Endpoint.clone(), Spec.Bind.clone()),
		None => Page::Create(Browse),
	};
	let Options = ChatOptions { Selectors:Spec.Selectors, ..ChatOptions::New(Spec.URL) };
	CreateChat(Arc::new(Page) as Arc<dyn ChatPort>, Options, Arc::new(Tokio))
}

/// A design verifier over a browser, with the workflow's measurement binding
/// installed.
pub fn Design(Browse:Arc<dyn Browse>, URL:impl Into<String>) -> Verifier {
	CreateVerify(Arc::new(Measure::Create(Browse, URL)), Arc::new(Tokio))
}

/// What a caller passes to `Browser` when it wants a visible browser on the
/// machine's own display.
pub fn Visible() -> ChromeOptions { ChromeOptions { Executable:None, Headless:false, NoSandbox:false } }

/// What a caller passes to `Browser` when it wants a headless one.
pub fn Headless() -> ChromeOptions { ChromeOptions { Executable:None, Headless:true, NoSandbox:false } }

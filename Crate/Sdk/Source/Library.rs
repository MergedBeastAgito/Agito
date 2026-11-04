//! `agentic-sdk`: the one crate a consumer names.
//!
//! A consumer of this workspace needs a browser, a chat driver, a design check
//! and a corpus validator. Until this crate existed it named five packages to
//! get them - `agentic-runtime`, `agentic-browser-cdp`, `agentic-chat-driver`,
//! `agentic-design-verification`, `agentic-skill-contract` - each with its own
//! module tree, its own error type and its own spelling of a target. The facade
//! answers one question: what does a consumer import.
//!
//! It adds no behaviour. It holds no state, starts nothing, and declares no port
//! of its own: every item below is an item that already existed one level down,
//! reachable at a path that will not change when the crate that owns it does.
//!
//! # What may appear here
//!
//! The surface a consumer is meant to call, and nothing else: the port traits,
//! the data they take and return, the one constructor that binds a port to a
//! substrate, and the manifest table. What stays out, and why each is a decision
//! rather than a gap:
//!
//! - **`chromiumoxide`.** The CDP client is an implementation detail of
//!   `agentic.browse`: a different substrate would replace one file below this
//!   crate, and a consumer that had named the client's types would have to
//!   change with it. Nothing here re-exports one - [`Chrome`] is this
//!   workspace's launcher, not the library's browser.
//! - **The adapter seam.** `Launcher`, `Transport` and `PageWant` are how a
//!   session is bound to a mechanism. A consumer binds a session, not a
//!   transport; an extension point nobody extends is a second way to get the
//!   browser wrong.
//! - **The policy engine.** Leases, fences, per-run budgets and the local
//!   stand-in are the composition root's business. A consumer that held a lease
//!   would be a second authority over the same resource.
//! - **The script helpers.** `Escape`, `FillDeclared` and the compiled
//!   expressions are build-time machinery for the crates that send JavaScript. A
//!   consumer sends an expression string.
//! - **`BlockOn`.** It drives a future on the current thread, and the browser
//!   adapter spawns its handler on tokio. A consumer that reached for it would
//!   panic inside a launch that had already succeeded, so the executor stays the
//!   consumer's to choose and the adapter's to require.
//!
//! # Five ports, four `Error` types, one at the root
//!
//! Four of the five crates below ship an enum called `Error`, and four ship a
//! constructor called `Create`. Only one of each can sit at the root, so the one
//! that does is `agentic.browse`'s: open, evaluate and dispose are the calls the
//! required flow makes, and a failure to open a page is the failure a consumer
//! matches on first. Every other port's copy is at its own path -
//! [`Port::Chat::Error`], [`Port::Design::Error`], [`Port::Runtime::Error`],
//! [`Port::Skill::Error`] - and every re-export in this file names the port it
//! belongs to, so the choice is readable rather than remembered.
//!
//! # The flow the surface is shaped around
//!
//! Build a session, open a page, evaluate an expression, dispose. Every name in
//! it comes from this crate:
//!
//! ```text
//! use agentic_sdk::{Browse, Call, Chrome, ChromeOptions, Create, Never, OpenOptions, Options, Tokio};
//!
//! let Bounded = Call::Within(Duration::from_secs(30), &Tokio, Arc::new(Never));
//! let Session = Create(
//!     Chrome::New(ChromeOptions { Headless: true, ..Default::default() }),
//!     Arc::new(Tokio),
//!     Options::default(),
//! );
//! let Target = Session.Open(Url, OpenOptions::Launch(Bounded.clone())).await?;
//! let Value = Session.Evaluate(&Target, "document.title", Bounded).await?;
//! Session.Dispose(Target).await;
//! ```
//!
//! The session is opened inside a tokio runtime: [`Tokio`] is the pace the
//! launcher needs, because the adapter spawns the handler that keeps the page
//! answering, and a spawn outside a runtime panics. That is the adapter's
//! contract, and it is the only reason this facade's flow names an executor at
//! all - the capability crates themselves stay runtime-neutral.

#![allow(non_snake_case)]
#![allow(non_upper_case_globals)]
#![allow(clippy::upper_case_acronyms)]

pub mod Port;
pub mod Struct;

/// `agentic.runtime` - the deadline and the caller's cancellation, as one value
/// every port method takes. Supplied rather than read from a global, so two
/// operations in one process can be bounded differently.
pub use AgenticRuntime::Call;
/// `agentic.runtime` - the return type of every port method: one boxed future.
/// It is what keeps `Arc<dyn Port>` possible, because a port with an `async fn`
/// in it is not object-safe and a port that is not object-safe cannot be
/// substituted by a fake.
pub use AgenticRuntime::FutureBox;
/// `agentic.runtime` - the caller's signal, read as a flag and awaited as a
/// future.
pub use AgenticRuntime::Cancellation;
/// `agentic.runtime` - the only clock a deadline is compared against.
pub use AgenticRuntime::Clock;
/// `agentic.runtime` - the clock, plus the ability to wait.
pub use AgenticRuntime::Pace;
/// `agentic.runtime` - a cancellation that never fires.
pub use AgenticRuntime::Never;
/// `agentic.runtime` - a cancellation the caller fires when it gives up.
pub use AgenticRuntime::Signal;
/// `agentic.runtime` - the wall clock, for a caller with nothing better.
pub use AgenticRuntime::Wall;
/// `agentic.runtime` - why a bounded operation stopped: the deadline passed, or
/// the caller cancelled.
pub use AgenticRuntime::Expired;
/// `agentic.runtime` - one operation plus one deadline, as an answer or an
/// expiry.
pub use AgenticRuntime::Bound;
/// `agentic.runtime` - two operations, whichever answers first.
pub use AgenticRuntime::Race;
/// `agentic.runtime` - which of the two `Race` returned.
pub use AgenticRuntime::Either;
/// `agentic.browse` - the port itself: open, evaluate, dispose.
pub use AgenticBrowserCDP::Browse;
/// `agentic.browse` - what an operation is bounded by, plus whether to attach to
/// a browser somebody else owns.
pub use AgenticBrowserCDP::OpenOptions;
/// `agentic.browse` - the handle a session is bound to.
pub use AgenticBrowserCDP::Target;
/// `agentic.browse` - who owns the page behind a target, and therefore who may
/// close it.
pub use AgenticBrowserCDP::Ownership;
/// `agentic.browse` - the port bound to one launcher and one pace.
pub use AgenticBrowserCDP::Session;
/// `agentic.browse` - build that session.
pub use AgenticBrowserCDP::Create;
/// `agentic.browse` - how a session is configured: where a dispose failure goes,
/// and how long a release may take.
pub use AgenticBrowserCDP::Options;
/// `agentic.browse` - where a dispose failure goes, because it may not be
/// returned.
pub use AgenticBrowserCDP::Report;
/// `agentic.browse` - the launcher the workspace ships. The only name here whose
/// implementation is a substrate, and it names none of the substrate's types.
pub use AgenticBrowserCDP::Chrome;
/// `agentic.browse` - how to start or reach a browser.
pub use AgenticBrowserCDP::ChromeOptions;
/// `agentic.browse` - the pace that launcher requires, because it spawns a
/// handler on tokio.
pub use AgenticBrowserCDP::Tokio;
/// `agentic.browse` - the one error type a session operation fails with.
pub use AgenticBrowserCDP::Error;
/// `agentic.browse` - its three codes: `browser-unavailable`,
/// `navigation-failed`, `timeout`.
pub use AgenticBrowserCDP::Code;

/// The workspace's manifest table: every module it ships, with the port each
/// provides and the bytes of its `module.json`.
pub use crate::Struct::Manifest::All;
/// One module, as the workspace ships it.
pub use crate::Struct::Manifest::Manifest;
/// The module with a given name, if the workspace ships it.
pub use crate::Struct::Manifest::Find;

//! `agito-sdk`: the one crate a consumer names.
//!
//! A consumer of this workspace needs a browser, a chat driver, a design check
//! and a corpus validator. Until this crate existed it named five packages to
//! get them - `agito-runtime`, `agito-browser-cdp`, `agito-chat-driver`,
//! `agito-design-verification`, `agito-skill-contract` - each with its own
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
//!   `agito.browse`: a different substrate would replace one file below this
//!   crate, and a consumer that had named the client's types would have to
//!   change with it. Nothing here re-exports one - [`Chrome`] is this
//!   workspace's launcher, not the library's browser.
//! - **The browser's own adapter seam.** `Launcher`, `Transport` and `PageWant`
//!   are how a session is bound to a mechanism inside `agito-browser-cdp`. A
//!   consumer binds a session, not a transport; an extension point nobody
//!   extends is a second way to get the browser wrong. That is the opposite of
//!   [`Adapter`], which is exported: this seam is one crate's internals, while
//!   the adapter port is the shape another crate implements.
//! - **The policy engine.** Leases, fences, per-run budgets and the runtime's
//!   local stand-in - a different double from the adapter's [`StandIn`] - are
//!   the composition root's business. A consumer that held a lease would be a
//!   second authority over the same resource.
//! - **The script helpers.** `Escape`, `FillDeclared` and `AssertParameters` are
//!   build-time machinery for the crates that send JavaScript, and a consumer
//!   sends an expression string. The `script!` macro is the one exception, and it
//!   is re-exported: it is how a consumer that keeps its own
//!   `Source/Script/**/*.ts` turns those files into the string `Evaluate` takes,
//!   so without it such a consumer would have to name `agito-runtime` for a
//!   single macro and the facade would not have removed the dependency it exists
//!   to remove.
//! - **`Block`.** It drives a future on the current thread, and the browser
//!   adapter spawns its handler on tokio. A consumer that reached for it would
//!   panic inside a launch that had already succeeded, so the executor stays the
//!   consumer's to choose and the adapter's to require.
//! - **The four ports outside the consumer's flow.** `agito.lane.decide`,
//!   `agito.catalog.rules`, `agito.workflow` and `agito.compose` are not
//!   re-exported here, and that is curation rather than an omission: this crate
//!   answers what a consumer imports to browse a page, ask a chat, check a
//!   design and validate a corpus, while those four are what the composition
//!   root asks for. `agito-workflow` is the sharpest case - it depends on
//!   `agito-browser-cdp`, so re-exporting it would hand every consumer the
//!   substrate choices this crate exists to hide. `agito.compose` is the root,
//!   which nothing in this tree may name. `agito.lane.decide` leaves its round
//!   trip through a `Poster` the caller supplies, and `agito.catalog.rules` is
//!   the model a manifest is checked against rather than a call a consumer
//!   makes. The set is exactly these six, and widening it is an edit to this
//!   file rather than a line that arrives with the next crate.
//!
//! # Six ports, five `Error` types, one at the root
//!
//! Five of the six crates below ship an enum called `Error`, and four ship a
//! constructor called `Create`. Only one of each can sit at the root, so the one
//! that does is `agito.browse`'s: open, evaluate and dispose are the calls the
//! required flow makes, and a failure to open a page is the failure a consumer
//! matches on first. Every other port's copy is at its own path -
//! [`Port::Adapter::Error`], [`Port::Chat::Error`], [`Port::Design::Error`],
//! [`Port::RunTime::Error`], [`Port::Skill::Error`] - and every re-export in
//! this file names the port it belongs to, so the choice is readable rather than
//! remembered.
//!
//! # The flow the surface is shaped around
//!
//! Build a session, open a page, evaluate an expression, dispose. Every name in
//! it comes from this crate:
//!
//! ```text
//! use SDK::{Browse, Call, Chrome, ChromeOptions, Create, Never, OpenOptions, Options, Tokio};
//!
//! let Bounded = Call::Within(Duration::from_secs(30), &Tokio, Arc::new(Never));
//! let Session = Create(
//!     Chrome::New(ChromeOptions { Headless: true, ..Default::default() }),
//!     Arc::new(Tokio),
//!     Options::default(),
//! );
//! let Target = Session.Open(URL, OpenOptions::Launch(Bounded.clone())).await?;
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

/// `agito.runtime` - the deadline and the caller's cancellation, as one value
/// every port method takes. Supplied rather than read from a global, so two
/// operations in one process can be bounded differently.
pub use ::RunTime::Call;
/// `agito.runtime` - the return type of every port method: one boxed future.
/// It is what keeps `Arc<dyn Port>` possible, because a port with an `async fn`
/// in it is not object-safe and a port that is not object-safe cannot be
/// substituted by a fake.
pub use ::RunTime::FutureBox;
/// `agito.runtime` - the caller's signal, read as a flag and awaited as a
/// future.
pub use ::RunTime::Cancellation;
/// `agito.runtime` - the only clock a deadline is compared against.
pub use ::RunTime::Clock;
/// `agito.runtime` - the clock, plus the ability to wait.
pub use ::RunTime::Pace;
/// `agito.runtime` - a cancellation that never fires.
pub use ::RunTime::Never;
/// `agito.runtime` - a cancellation the caller fires when it gives up.
pub use ::RunTime::Signal;
/// `agito.runtime` - the wall clock, for a caller with nothing better.
pub use ::RunTime::Wall;
/// `agito.runtime` - why a bounded operation stopped: the deadline passed, or
/// the caller cancelled.
pub use ::RunTime::Expired;
/// `agito.runtime` - one operation plus one deadline, as an answer or an
/// expiry.
pub use ::RunTime::Bound;
/// `agito.runtime` - two operations, whichever answers first.
pub use ::RunTime::Race;
/// `agito.runtime` - which of the two `Race` returned.
pub use ::RunTime::Either;
/// `agito.runtime` - the macro that turns a consumer's own
/// `Source/Script/**/*.ts` (compiled to `OUT_DIR/Script` by its `build.rs`) into
/// the expression string a port method takes, with the parameters it declares
/// filled in. Re-exported because it is the consumer's entry point to the script
/// convention and not one of the helpers the convention is built from; the macro
/// expands to `$crate::FillDeclared`, so `$crate` still resolves to
/// `agito-runtime` and nothing below this crate becomes reachable through it.
pub use ::RunTime::script;
/// `agito.browse` - the port itself: open, evaluate, dispose.
pub use ::CDP::Browse;
/// `agito.browse` - what an operation is bounded by, plus whether to attach to
/// a browser somebody else owns.
pub use ::CDP::OpenOptions;
/// `agito.browse` - the handle a session is bound to.
pub use ::CDP::Target;
/// `agito.browse` - who owns the page behind a target, and therefore who may
/// close it.
pub use ::CDP::Ownership;
/// `agito.browse` - the port bound to one launcher and one pace.
pub use ::CDP::Session;
/// `agito.browse` - build that session.
pub use ::CDP::Create;
/// `agito.browse` - how a session is configured: where a dispose failure goes,
/// and how long a release may take.
pub use ::CDP::Options;
/// `agito.browse` - where a dispose failure goes, because it may not be
/// returned.
pub use ::CDP::Report;
/// `agito.browse` - the launcher the workspace ships. The only name here whose
/// implementation is a substrate, and it names none of the substrate's types.
pub use ::CDP::Chrome;
/// `agito.browse` - how to start or reach a browser.
pub use ::CDP::ChromeOptions;
/// `agito.browse` - the pace that launcher requires, because it spawns a
/// handler on tokio.
pub use ::CDP::Tokio;
/// `agito.browse` - the one error type a session operation fails with.
pub use ::CDP::Error;
/// `agito.browse` - its three codes: `browser-unavailable`,
/// `navigation-failed`, `timeout`.
pub use ::CDP::Code;
/// `agito.adapter` - the port a substrate this workspace did not write
/// implements: what it serves, one request bounded by the caller's deadline, and
/// a release that never fails.
///
/// It is at the root because a third-party adapter is the consumer that has
/// nothing else: a crate outside this tree depends on this facade and on no
/// other Agito package, so a port it cannot name here is a port it cannot
/// implement. Every method takes `&self`, which is what makes `Arc<dyn Adapter>`
/// possible.
pub use ::Adapter::Adapter;
/// `agito.adapter` - whether a value agrees with the manifest that named it.
/// The composition root calls this once, where both halves are in hand; the
/// manifest reader cannot, because it never constructs the value.
pub use ::Adapter::Serves;
/// `agito.adapter` - a scripted adapter, so a consumer tests its own use of
/// the port with no substrate behind it.
pub use ::Adapter::StandIn;

/// The workspace's manifest table: every module it ships, with the port each
/// provides and the bytes of its `module.json`.
pub use crate::Struct::Manifest::All;
/// One module, as the workspace ships it.
pub use crate::Struct::Manifest::Manifest;
/// The module with a given name, if the workspace ships it.
pub use crate::Struct::Manifest::Find;

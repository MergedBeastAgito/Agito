---
title: "Converting the capability system to Rust"
thread: 4ffde6d5-a7d3-404f-96b5-e24f8009a76b
project: DELO
kind: research
date: 2026-11-03
status: captured
source: https://www.perplexity.ai/search/4ffde6d5-a7d3-404f-96b5-e24f8009a76b
captured: 2026-11-03T21:57:33
segments: 1
collapsed: false
steps: 3
sources: 35
---

# Converting the capability system to Rust

| Field         | Value                                                                 |
| ------------- | --------------------------------------------------------------------- |
| Thread        | `4ffde6d5-a7d3-404f-96b5-e24f8009a76b`                                |
| Project       | DELO                                                                  |
| Source        | https://www.perplexity.ai/search/4ffde6d5-a7d3-404f-96b5-e24f8009a76b |
| Captured      | 2026-11-03T21:57:33                                                   |
| Segments      | 1                                                                     |
| Trace steps   | 3                                                                     |
| Cited sources | 35                                                                    |
| Characters    | 31499                                                                 |

## Transcript

DELO 7:02 PM

I am converting a TypeScript capability system into Rust and I need a workspace
design, not a survey. Here is the system and the house style it must fit.

THE SYSTEM (currently TypeScript, five packages, each with a package.json and a
module.json manifest):

- @agentic/browser-cdp (adapter): browser execution substrate - attach versus
  launch, bounded evaluation, disposal. state: holds-transport. errors:
  browser-unavailable, navigation-failed, timeout.
- @agentic/chat-driver (capability): ask a chat product, correlate turns, verify
  mode and model, read the answer and its step trace. requires the browse port.
  state: holds-transport.
- @agentic/design-verification (capability): measure a built page and produce
  evidence; the site's own expectations stay outside. requires the browse port.
  state: stateless.
- @agentic/skill-contract (capability): discover, validate and package a skill
  corpus. state: stateless.
- @agentic/runtime (runtime): admission, leases, budgets, cancellation, cleanup,
  telemetry. state: holds-workspace. Four layers: capability (a use case plus
  the port it needs), adapter (one port against one provider), workflow (a
  composition), composition root (the only place that names concrete adapters).
  Arrows point inward; nothing imports a root; a capability importing an adapter
  is a defect. Ports are owned by their consumers, not by a central crate. Every
  package carries a manifest whose state class decides what a pre-warm harness
  may prepare (stateless / holds-transport / holds-identity / holds-workspace)
  and what it must revalidate at admission.

THE HOUSE STYLE it must mimic, from github.com/PlayForm/Run (Rust, branch
Current), verbatim:

- .cargo/config.toml sets build.target-dir = "Target", and release profile
  opt-level 3, codegen-units 1, debug false, lto true, panic "abort", strip
  true.
- Cargo.toml declares [lib] name = "Library", path = "Source/Library.rs",
  crate-type = ["staticlib", "cdylib", "rlib"], and several [[bin]] entries
  (PRun, Run, Inn, InnKeeper) all pointing at that same Source/Library.rs.
- Source/Library.rs begins with #![allow(non_snake_case)] and
  #![allow(clippy::upper_case_acronyms)], then `mod Fn; mod Struct;`, and its
  main is #[tokio::main] delegating to Struct::Binary::Command::Struct::Fn().Fn.
- Modules live under Source/Fn/Binary/Command/Index.rs,
  Source/Fn/Binary/Command/Parallel/GPG/Git.rs,
  Source/Struct/Binary/Command.rs - PascalCase directories and files, functions
  under Fn, types under Struct.
- Dependencies in that repo include tokio (full), clap (derive), rayon,
  crossbeam-queue, ratatui, crossterm, walkdir, globset, once_cell, futures.
- CI is Build.yml, Check.yml, Publish.yml, Rust.yml, Auto.yml, Dependabot.yml,
  GitHub.yml.

QUESTIONS, in the order I need them answered:

1. Crate boundaries: which of the five should be separate crates in one
   workspace, and which should be modules inside one crate? Apply the rule that
   ports belong to consumers - what does that mean concretely in Cargo terms,
   where a shared `ports` crate is exactly the dependency magnet to avoid?
2. The CDP question: can browser-cdp be Rust (chromiumoxide, headless_chrome, or
   raw CDP over a websocket), or does it have to stay TypeScript? Compare
   against the option of keeping a small TS sidecar process that Rust drives
   over stdio JSON-RPC. Give the failure modes of each and a recommendation.
3. How do the manifest and the four state classes survive into Rust - a build
   script generating static data, a serde-loaded JSON file per crate, or const
   data in the crate itself? Which of those keeps the "no build at launch"
   promise and still lets a harness read it?
4. Where do the async runtime, cancellation and budgets live so that a
   capability does not have to know about tokio directly? Show the trait shape
   for a port with cancellation and a deadline.
5. How do I express "the arrows point inward" as something a compiler or CI can
   enforce in a Cargo workspace, given that Cargo itself permits any dependency
   direction?
6. Map the TypeScript manifest fields (name, kind, version, provides, requires,
   artefact with a hash, state, cost with cold/warm ms, verify command) onto
   Rust idioms without losing the harness's ability to decide what to pre-warm.
   Be concrete: give crate names, module paths in the house style, trait
   signatures, and the CI check. Say where you are unsure, and flag anything in
   the house style that fights the design.

chromiumoxide handler poll browser connect launch rust headless_chrome rust
browser synchronous thread websocket cargo metadata forbid dependencies
workspace architecture

![](https://www.google.com/s2/favicons?sz=128&domain=https://docs.rs/headless_chrome/latest/headless_chrome/browser/struct.Browser.html)

Browser in headless_chrome - Rust - Docs.rs docs

![](https://www.google.com/s2/favicons?sz=128&domain=https://github.com/rust-lang/cargo/issues/10636)

default features always resolved for dependency in workspace mode github

![](https://www.google.com/s2/favicons?sz=128&domain=https://github.com/mattsse/chromiumoxide/issues/49)

Chrome stays idle infinitely #49 - mattsse/chromiumoxide - GitHub github

![](https://www.google.com/s2/favicons?sz=128&domain=https://lib.rs/crates/headless_chrome_fork)

headless_chrome_fork — WebSocket library in Rust // Lib.rs lib

![](https://www.google.com/s2/favicons?sz=128&domain=https://docs.rs/headless_chrome_fork/latest/headless_chrome/browser/struct.Browser.html)

Browser in headless_chrome::browser - Rust - Docs.rs docs

![](https://www.google.com/s2/favicons?sz=128&domain=https://dev.to/vhub_systems_ed5641f65d59/headless-browsers-in-rust-chromiumoxide-vs-headlesschrome-vs-the-python-alternative-25e5)

Headless Browsers in Rust: Chromiumoxide vs headless_chrome vs ... dev

![](https://www.google.com/s2/favicons?sz=128&domain=https://github.com/rust-headless-chrome/rust-headless-chrome/blob/master/src/browser/process.rs)

rust-headless-chrome/src/browser/process.rs at main - GitHub github

![](https://www.google.com/s2/favicons?sz=128&domain=https://www.browsercat.com/docs/connect-with/headless-chrome)

Getting Started with Rust Headless Chrome - BrowserCat browsercat

![](https://www.google.com/s2/favicons?sz=128&domain=https://lib.rs/crates/spider_chromiumoxide_types)

spider_chromiumoxide_types — Rust web dev library // Lib.rs lib

![](https://www.google.com/s2/favicons?sz=128&domain=https://github.com/rust-lang/cargo/issues/11133)

Allow publishing with path dependencies without a version (RFC2906) github

![](https://www.google.com/s2/favicons?sz=128&domain=https://docs.rs/oxide-browser-sh)

oxide_browser_sh - Rust - Docs.rs docs

![](https://www.google.com/s2/favicons?sz=128&domain=https://crates.io/crates/cargo-workspace-deps)

cargo-workspace-deps - crates.io: Rust Package Registry crates

![](https://www.google.com/s2/favicons?sz=128&domain=https://github.com/rust-lang/cargo/issues/5332)

Allow to share versions of dependencies between unrelated Cargo ... github

![](https://www.google.com/s2/favicons?sz=128&domain=https://www.reddit.com/r/rust/comments/aqgsxk/announcing_the_headless_chrome_crate_puppeteer/)

Announcing the Headless Chrome crate ("Puppeteer for Rust") reddit

![](https://www.google.com/s2/favicons?sz=128&domain=https://docs.rs/fob-browser-test)

fob_browser_test - Rust - Docs.rs docs
https://github.com/PlayForm/Run/tree/Current
https://docs.rs/chromiumoxide/latest/chromiumoxide/
https://doc.rust-lang.org/cargo/commands/cargo-metadata.html

![](https://www.google.com/s2/favicons?sz=128&domain=https://docs.rs/chromiumoxide/latest/chromiumoxide/)

chromiumoxide - Rust docs

![](https://www.google.com/s2/favicons?sz=128&domain=https://doc.rust-lang.org/cargo/commands/cargo-metadata.html)

cargo metadata - The Cargo Book doc.rust-lang

Keep **all five** as separate crates. Add a composition-root crate and, when you
have an actual reusable composition, a workflow crate. Keep consumer-owned
browse traits inside the two capabilities—not in a shared `ports` crate—and put
the concrete adapter-to-port bindings in the composition root.

That is the design I would implement. It preserves the five existing admission
units, fits the PascalCase source layout, and gives CI meaningful boundaries to
enforce.

I could not retrieve the `Current` branch of PlayForm/Run, so I am treating your
verbatim description as the house-style specification, not claiming independent
verification of that repository.

## 1. Crate boundaries and consumer-owned ports

## Workspace

```
Cargo.toml
Cargo.lock
.cargo/Config.toml             # Actual Cargo filename must be config.toml
Crate/
  BrowserCDP/
    Cargo.toml
    module.json
    Source/
      Library.rs
      Fn.rs
      Struct.rs
      Fn/Session/Attach.rs
      Fn/Session/Launch.rs
      Fn/Session/Dispose.rs
      Struct/Session.rs
      Struct/Error.rs

  ChatDriver/
    Cargo.toml
    module.json
    Source/
      Library.rs
      Fn.rs
      Struct.rs
      Fn/Ask.rs
      Fn/Verify.rs
      Struct/Port/Browse.rs
      Struct/Call.rs
      Struct/Turn.rs
      Struct/Error.rs

  DesignVerification/
    Cargo.toml
    module.json
    Source/
      Library.rs
      Fn.rs
      Struct.rs
      Fn/Measure.rs
      Struct/Port/Browse.rs
      Struct/Call.rs
      Struct/Evidence.rs

  SkillContract/
    Cargo.toml
    module.json
    Source/
      Library.rs
      Fn.rs
      Struct.rs
      Fn/Discover.rs
      Fn/Validate.rs
      Fn/Package.rs
      Struct/Corpus.rs

  Runtime/
    Cargo.toml
    module.json
    Source/
      Library.rs
      Fn.rs
      Struct.rs
      Fn/Admission.rs
      Fn/Cleanup.rs
      Struct/Lease.rs
      Struct/Budget.rs
      Struct/Workspace.rs

  Workflow/                    # Add when there is a real composition
    Cargo.toml
    module.json
    Source/Library.rs
    Source/Fn/VerifySkillPage.rs

  Agentic/                     # Composition root, including CLI entry points
    Cargo.toml
    module.json
    Source/Library.rs
    Source/Main.rs
    Source/Fn/Composition/Build.rs
    Source/Fn/Binding/ChatBrowse.rs
    Source/Fn/Binding/DesignBrowse.rs
    Source/Struct/Binding/Browser.rs
    Source/Struct/Binary/Command.rs
```

Use `.cargo/config.toml` in the actual tree; the capitalized placeholder above
is not a proposed filename change.

| Package name                  | Library crate name          | Responsibility                                        |
| ----------------------------- | --------------------------- | ----------------------------------------------------- |
| `agentic-browser-cdp`         | `AgenticBrowserCDP`         | Concrete browser substrate                            |
| `agentic-chat-driver`         | `AgenticChatDriver`         | Chat use case and its browse contract                 |
| `agentic-design-verification` | `AgenticDesignVerification` | Measurement/evidence use case and its browse contract |
| `agentic-skill-contract`      | `AgenticSkillContract`      | Corpus discovery, validation, packaging               |
| `agentic-runtime`             | `AgenticRuntime`            | Execution policy and resource ownership               |
| `agentic-workflow`            | `AgenticWorkflow`           | Adapter-independent compositions                      |
| `agentic`                     | `Agentic`                   | Wiring and executable entry points                    |

The five existing packages should not become modules of one crate. Doing that
would turn import-direction rules into conventions inside a single compiler
boundary. Keep their internal operations—attach/launch/dispose, ask/verify/read,
discover/validate/package—as modules.

## Dependency graph

Here, `A → B` means “A declares a Cargo dependency on B.”

```
Agentic composition root
  → Runtime
  → BrowserCDP
  → ChatDriver
  → DesignVerification
  → SkillContract
  → Workflow

Workflow
  → ChatDriver
  → DesignVerification
  → SkillContract

BrowserCDP              → no other domain workspace crate
ChatDriver              → no other domain workspace crate
DesignVerification      → no other domain workspace crate
SkillContract           → no other domain workspace crate
Runtime                 → no other domain workspace crate
```

Runtime is a policy engine, not a registry importing every capability. The root
registers capability invocations and cleanup operations with it.

## What “ports belong to consumers” means here

There are two distinct traits:

```
AgenticChatDriver::Struct::Port::Browse::Browse
AgenticDesignVerification::Struct::Port::Browse::Browse
```

They need not have identical methods. Chat might require turn observation;
design verification might require screenshots and layout measurements. Neither
should grow operations merely because the other needs them.

`BrowserCDP` exports concrete substrate operations using its own types. The root
owns a wrapper and implements both consumer traits:

```
// Agentic/Source/Struct/Binding/Browser.rs
pub struct Browser {
    pub Session: std::sync::Arc<AgenticBrowserCDP::Struct::Session::Session>,
}

// Agentic/Source/Fn/Binding/ChatBrowse.rs
impl AgenticChatDriver::Struct::Port::Browse::Browse
    for crate::Struct::Binding::Browser::Browser
{
    // Convert the consumer's requests/results to/from substrate operations.
}
```

The wrapper is local to the root; this avoids trying to implement a foreign
trait for a foreign adapter type.

This introduces small amounts of translation code deliberately. Do not replace
it with a shared “universal browser request” type that quietly becomes the
forbidden ports crate.

An alternative would let `BrowserCDP` depend on the capabilities and implement
their traits directly. That is inward-pointing, but couples your generic
substrate to every consumer. I would not choose it for this system. Root-owned
bindings keep the substrate independent and keep concrete adapter references
confined to wiring.

## 2. CDP: use Rust, with a sidecar only as a migration bridge

Browser CDP does not have to stay TypeScript. `chromiumoxide` provides an async
Rust CDP API; its documented example launches Chrome and continuously polls a
handler task. `headless_chrome` exposes browser launch and connection APIs,
including connection to an externally launched browser.docs+1

My recommendation: implement `agentic-browser-cdp` in Rust using
`chromiumoxide`, behind your own small session API. Keep a TS sidecar only if
existing browser behavior cannot yet pass equivalent Rust conformance tests.

| Choice                         | Fit                                                         | Failure modes you must handle                                                                                                                         |
| ------------------------------ | ----------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------- |
| `chromiumoxide`                | Best fit for an async runtime                               | Handler stops or is not polled; browser disconnects; pending commands outlive callers; target/session disappears; Chrome/protocol mismatch            |
| `headless_chrome`              | Reasonable for predominantly synchronous automation         | Blocking calls consume workers; cancellation does not necessarily interrupt an in-progress call; reconnect/idle behavior must match your lease policy |
| Raw CDP over WebSocket         | Appropriate only for a very narrow, stable protocol surface | You own request-ID correlation, event routing, session attachment, pending-request failure, protocol negotiation, reconnect and shutdown              |
| TS sidecar over stdio JSON-RPC | Lowest migration risk when preserving proven TS behavior    | Child death; framing errors; stdout contamination; pipe backpressure; version skew; orphan browser processes; uncertain outcome after timeout         |

The handler requirement is an explicit part of `chromiumoxide`’s usage model.
`headless_chrome`’s documented connection behavior also includes an
idle-disconnect policy, which you must account for rather than treating a warmed
connection as indefinitely valid.docs+1

## Session semantics to specify before implementation

```
pub enum Ownership {
    Attached, // Never terminate the externally owned browser.
    Launched, // Root/runtime owns eventual browser-process cleanup.
}
```

For both backends:

- Attach and launch are separate operations, not a boolean hidden in a
  constructor.
- Disposal of an attached session closes owned targets/contexts and transport
  according to the lease, not the whole browser.
- Disposal of a launched session includes an explicit browser-process
  termination policy.
- A disconnected session invalidates its warm readiness.
- Map provider failures into your stable `BrowserUnavailable`,
  `NavigationFailed`, and `Timeout` categories; keep provider details as
  diagnostic information.
- Represent cancellation distinctly from a navigation failure. Preserve existing
  external error codes if needed, but add a structured cancellation reason.

“Bounded evaluation” needs two bounds: how long the caller waits and how long
the browser operation may continue. Dropping a Rust future or timing out a
JSON-RPC request is not proof that JavaScript stopped. Define escalation:
protocol-level interruption where available, close an owned target, then
terminate an owned browser if necessary. An attached shared browser may not
permit the strongest escalation.

## If you retain the sidecar

Use versioned, framed JSON-RPC with:

- A startup handshake reporting protocol version and supported operations.
- Request IDs and explicit cancellation messages.
- A local deadline established by Rust; transmit remaining duration, not a
  process-local `Instant`.
- stdout reserved exclusively for protocol traffic; logs on stderr.
- Bounded message sizes and bounded outstanding requests.
- A shutdown acknowledgement followed by forced termination after a cleanup
  grace period.
- No automatic replay of mutating operations after an ambiguous timeout.
- Prebuilt JS plus a pinned runtime distributed with the release—no
  `npm install`, transpilation or browser download at launch.

I cannot establish parity between your current TS implementation and either Rust
library from the package description. Make attach/launch, navigation races,
evaluation timeout, disconnect, disposal and shared-browser ownership the
acceptance suite before removing the sidecar.

## 3. Manifest: checked-in JSON, embedded unchanged, finalized at packaging

Use a checked-in `module.json` per crate as the canonical descriptor. Embed
those exact bytes when the crate needs introspection:

```
pub const ManifestJSON: &str = include_str!("../module.json");
```

The harness reads the packaged JSON without loading the library or starting the
application.

| Approach                               | Decision                                                                           |
| -------------------------------------- | ---------------------------------------------------------------------------------- |
| `build.rs` generating static data      | Optional later for typed access; unnecessary initially                             |
| `serde`-loaded JSON                    | Yes: validate in CI and read in the harness                                        |
| Hand-maintained Rust `const` structure | No as the only source: the external harness would need an export step or execution |
| JSON plus `include_str!`               | Recommended: one descriptor source, readable externally and embedded internally    |

All three can satisfy “no build at launch” if compilation happens before
deployment. The operational rule matters more than the representation: never
launch through `cargo run`, and never use `cargo test` as an admission
verification command.

## Separate descriptor identity from artifact identity

A final binary cannot generally embed the hash of its own final bytes without
creating a self-reference problem.

Use two stages:

1. Source `module.json`: name, kind, version, contracts, state, cost model and
   verification specification.
2. Release packaging: emit a finalized external descriptor/catalog containing
   artifact path, target triple, size and SHA-256.

The embedded JSON is the source descriptor. The external release descriptor
binds that descriptor to the executable or bundle actually delivered.

For statically linked capabilities, several module entries can legitimately
reference the same root executable hash. Separate crates do not require
separately loadable runtime plugins.

## 4. Runtime, cancellation and budgets

Put Tokio in `agentic-runtime`, `agentic-browser-cdp`, and the root entry point.
Keep capabilities executor-independent: `std::future::Future`,
`std::time::Instant`, and consumer-owned traits.

The root starts one Tokio runtime. `agentic-runtime` owns admission queues,
lease records, cancellation sources, deadline timers, budget accounting,
telemetry and cleanup supervision. The browser adapter owns its transport task,
supervised through runtime/root wiring.

## A consumer-owned port

For example, in `ChatDriver/Source/Struct/Call.rs`:

```
use std::{
    future::Future,
    pin::Pin,
    time::Instant,
};

pub type FutureBox<'a, T> =
    Pin<Box<dyn Future<Output = T> + Send + 'a>>;

pub trait Cancellation: Send + Sync {
    fn IsCancelled(&self) -> bool;

    // Must be race-safe and resolve immediately if already cancelled.
    fn Cancelled(&self) -> FutureBox<'_, ()>;
}

pub struct Call<'a> {
    pub Deadline: Instant,
    pub Cancellation: &'a dyn Cancellation,
}
```

In `ChatDriver/Source/Struct/Port/Browse.rs`:

```
use crate::Struct::Call::{Call, FutureBox};

pub struct Evaluation {
    pub Script: String,
    pub MaximumResultBytes: usize,
}

pub struct EvaluationResult {
    pub JSON: String,
}

pub enum BrowseError {
    BrowserUnavailable,
    NavigationFailed(String),
    Timeout,
    Cancelled,
    ResultTooLarge,
}

pub trait Browse: Send + Sync {
    fn Evaluate<'a>(
        &'a self,
        Request: Evaluation,
        Context: Call<'a>,
    ) -> FutureBox<'a, Result<EvaluationResult, BrowseError>>;
}
```

This is an object-safe, boxed-future shape; it permits `Arc<dyn Browse>` without
putting Tokio types in the capability API.

The design-verification capability owns its equivalent contract. Small context
definitions may initially be duplicated. Extract a tiny execution-primitives
crate only if duplication becomes a demonstrated problem; it must never acquire
browse traits, capability DTOs, registrations or adapter discovery.

## Who enforces what?

- Runtime: admission, overall deadline, lease and resource ceilings.
- Capability: algorithmic bounds, such as maximum turns or measurement count.
- Binding/adapter: operation deadline, cancellation observation and bounded
  result size.
- Runtime supervisor: cleanup even after the caller abandons the invocation.

Before each externally visible operation, the binding checks cancellation and
deadline, reserves the appropriate budget, and invokes the adapter. It then
observes completion, cancellation or timeout.

Capabilities can simply `await` the port. They do not need `tokio::select!`.

Cancellation is cooperative, not a guarantee of rollback. Cleanup must be an
explicit async operation with its own short deadline; do not rely on `Drop` to
perform asynchronous disposal. Also distinguish reservation from charging: a
timed-out browser action may already have consumed its budget.

## 5. Enforce inward arrows with Cargo boundaries plus CI

Cargo metadata exposes declared dependencies, their kinds, optional status,
target conditions and renames. Use the declared package dependency lists for the
architecture policy, not only the currently enabled resolved graph. That catches
dormant optional edges as
well.[doc.rust-lang](https://doc.rust-lang.org/cargo/commands/cargo-metadata.html)

## Concrete architecture check

Place this in `Tool/CheckArchitecture.py`:

```
import json
import subprocess

metadata = json.loads(subprocess.check_output([
    "cargo", "metadata", "--format-version", "1",
    "--no-deps", "--locked"
]))

packages = {p["name"]: p for p in metadata["packages"]}
members = {
    p["name"]
    for p in metadata["packages"]
    if p["id"] in metadata["workspace_members"]
}

allowed = {
    "agentic-browser-cdp": set(),
    "agentic-chat-driver": set(),
    "agentic-design-verification": set(),
    "agentic-skill-contract": set(),
    "agentic-runtime": set(),
    "agentic-workflow": {
        "agentic-chat-driver",
        "agentic-design-verification",
        "agentic-skill-contract",
    },
    "agentic": {
        "agentic-browser-cdp",
        "agentic-chat-driver",
        "agentic-design-verification",
        "agentic-skill-contract",
        "agentic-runtime",
        "agentic-workflow",
    },
}

errors = []

for name in sorted(members):
    if name not in allowed:
        errors.append(f"{name}: missing architecture policy")
        continue

    for dependency in packages[name]["dependencies"]:
        target = dependency["name"]
        if target in members and target not in allowed[name]:
            errors.append(
                f"{name} -> {target}: forbidden "
                f"({dependency['kind'] or 'normal'} dependency)"
            )

capabilities = {
    "agentic-chat-driver",
    "agentic-design-verification",
    "agentic-skill-contract",
}

for name in capabilities & members:
    for dependency in packages[name]["dependencies"]:
        if dependency["name"] in {"tokio", "tokio-util"}:
            errors.append(f"{name}: executor-specific dependency")

if errors:
    raise SystemExit("\n".join(errors))
```

Apply the same direction rule to normal, build and dev dependencies.
Browser-backed capability integration tests belong in the root’s test targets,
not in a capability’s dev dependency on the adapter.

In `.github/workflows/Check.yml`:

```
- name: Check architecture
  run: python3 Tool/CheckArchitecture.py

- name: Check workspace
  run: cargo check --workspace --all-targets --all-features --locked

- name: Test workspace
  run: cargo test --workspace --all-features --locked
```

Also check each capability independently with `--no-default-features` to expose
accidental reliance on workspace-wide feature activation.

Add a second policy pass for external dependency allowlists. Otherwise someone
can evade the intended abstraction by adding `chromiumoxide` directly to a
capability without creating a forbidden workspace edge.

The compiler enforces access through declared dependencies and module
visibility; CI enforces which declarations are permitted. Neither a source grep
nor a Cargo feature is an adequate replacement for that policy.

## 6. Manifest field mapping and pre-warm admission

Keep logical capability contracts distinct from Rust crate names.

| TS field                       | Rust/workspace representation                                       | Harness interpretation                                      |
| ------------------------------ | ------------------------------------------------------------------- | ----------------------------------------------------------- |
| `name`                         | Preserve `@agentic/...` as logical name; add Cargo package identity | Stable module identity across migration                     |
| `kind`                         | JSON enum; optionally mirrored in `[package.metadata.agentic]`      | Capability, adapter, workflow, runtime/root                 |
| `version`                      | Cargo package version, checked against descriptor in CI             | Module release version—not automatically port compatibility |
| `provides`                     | Versioned logical contract IDs                                      | What can be satisfied by this component                     |
| `requires`                     | Versioned required contracts, with consumer semantics               | What root wiring must supply                                |
| `artefact`                     | Final release catalog entry with path, target, format and SHA-256   | Exact deployed executable/bundle identity                   |
| `state`                        | Serialized four-case enum                                           | Allowed pre-warm scope and required admission validation    |
| `cost.cold_ms`, `cost.warm_ms` | Measured estimates with provenance                                  | Scheduling estimates, not permission or hard budgets        |
| `verify`                       | Executable plus argument array and timeout                          | Prebuilt readiness/verification operation                   |

For your two browse consumers, `requires` can retain a common logical browse
family, but record the consumer profile or contract version. A shared string
does not mean the Rust traits must be shared. The root explicitly binds each
required profile and translates its types.

## State policy

| State             | Harness may prepare                                  | Admission must revalidate                                                     |
| ----------------- | ---------------------------------------------------- | ----------------------------------------------------------------------------- |
| `stateless`       | Immutable code/data caches, schemas, indexes         | Artifact/config/input identity; no hidden live session assumption             |
| `holds-transport` | Permitted transport/session resources                | Liveness, endpoint, ownership, target/context validity and lease exclusivity  |
| `holds-identity`  | Only explicitly authorized identity-scoped resources | Principal, tenant/account, credential/session validity and intended authority |
| `holds-workspace` | Workspace-scoped resources under policy              | Workspace identity, locks, lease, mutable revision and isolation              |

Apply them as follows:

- BrowserCDP: `holds-transport`; record attach/launch ownership separately.
- ChatDriver: `holds-transport`; additionally verify the expected
  product/session, mode and model on admission or use as appropriate.
- DesignVerification: `stateless`; borrow a leased browse port per invocation,
  retain no session afterward.
- SkillContract: `stateless`; immutable corpus caches must be keyed by corpus
  identity/hash.
- Runtime: `holds-workspace`; owns workspace-scoped policy state and cleanup
  records.

A material uncertainty: if ChatDriver retains an authenticated product session,
`holds-transport` alone may understate its admission requirements. Preserve your
existing class if necessary, but add explicit identity revalidation
requirements. The class is a preparation ceiling, not an exhaustive readiness
checklist.

Cost measurements should carry their basis—browser version, attach versus
launch, platform, sample statistic and measurement date. “Warm” is meaningful
only when the cached state passes admission again.

Verification should look like:

```
{
  "verify": {
    "executable": "Agentic",
    "args": ["verify", "--module", "@agentic/chat-driver"],
    "timeout_ms": 5000
  }
}
```

That is a proposed schema, not an inferred existing CLI. The release harness
invokes the shipped executable directly. Verification should report structured
readiness without silently doing an unbudgeted warm-up.

## House-style choices I would preserve—and exceptions I would make

## Preserve

```
# .cargo/config.toml
[build]
target-dir = "Target"
```

```
# Workspace Cargo.toml
[workspace]
resolver = "2"
members = ["Crate/*"]

[profile.release]
opt-level = 3
codegen-units = 1
debug = false
lto = true
panic = "abort"
strip = true
```

Use an explicit member list if your final directory nesting differs from the
sketch.

Keep `Source/Library.rs`, PascalCase paths, `Fn` for operations and `Struct` for
data/contracts:

```
#![allow(non_snake_case)]
#![allow(clippy::upper_case_acronyms)]

pub mod Fn;
pub mod Struct;
```

Use `#[path = "Index.rs"]` where needed; `Index.rs` is a house convention, not
automatic Rust module discovery.

## Exceptions worth making explicitly

1. Do not name every library `Library`. Use unique library target names such as
   `AgenticChatDriver`. Preserve the path, not an ambiguous crate identity.
2. Do not make each capability’s library root an executable entry point. Keep
   Tokio startup in `Agentic/Source/Main.rs`, delegating to the house-style
   command path. Multiple binary names may share that entry point if genuinely
   needed.
3. Do not emit `staticlib` and `cdylib` for every crate. Use `rlib` for internal
   crates. Add native-library outputs only for an actual FFI consumer; those
   outputs do not by themselves define a stable plugin ABI.
4. Treat `panic = "abort"` as incompatible with in-process panic recovery.
   Ordinary error and cancellation cleanup still works, but a panic cannot be
   recovered by a runtime supervisor in that process. If crash cleanup is
   mandatory, use an external supervisor and explicit child-process ownership
   records.
5. Do not inherit the entire dependency list. Capabilities should not acquire
   Tokio, Clap, Ratatui or browser libraries merely because the reference
   repository uses them.

These exceptions preserve the recognizable house layout while avoiding the parts
that would collapse your architectural boundaries or weaken the runtime’s
lifecycle guarantees.

Show the Cargo.toml for the workspace and crates Show how adapter-to-port
bindings look in the root Show how a capability crate implements a port Explain
how module.json fits into this design Show the file layout for the composition
root Ask a follow-up

## Step trace

The work the answer did, in the order it did it. Depth is nesting, so a step
inside a step is one level in.

| Depth | Step                                                                   | Duration | Icon         | Expanded |
| ----- | ---------------------------------------------------------------------- | -------- | ------------ | -------- |
| 0     | Finished                                                               | 13s      | chevron-down | true     |
| 1     | Researching Rust browser integration patterns for the workspace design | -        | world        | true     |
| 1     | Checking Rust browser integration and Cargo metadata patterns          | -        | book         | true     |

## Cited sources

| Source                                                                                                                            | Label                                                               |
| --------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------- |
| https://docs.rs/headless_chrome/latest/headless_chrome/browser/struct.Browser.html                                                | Browser in headless_chrome - Rust - Docs.rs                         |
| https://github.com/rust-lang/cargo/issues/10636                                                                                   | default features always resolved for dependency in workspace mode   |
| https://github.com/mattsse/chromiumoxide/issues/49                                                                                | Chrome stays idle infinitely #49 - mattsse/chromiumoxide - GitHub   |
| https://lib.rs/crates/headless_chrome_fork                                                                                        | headless_chrome_fork — WebSocket library in Rust // Lib.rs          |
| https://docs.rs/headless_chrome_fork/latest/headless_chrome/browser/struct.Browser.html                                           | Browser in headless_chrome::browser - Rust - Docs.rs                |
| https://dev.to/vhub_systems_ed5641f65d59/headless-browsers-in-rust-chromiumoxide-vs-headlesschrome-vs-the-python-alternative-25e5 | Headless Browsers in Rust: Chromiumoxide vs headless_chrome vs ...  |
| https://github.com/rust-headless-chrome/rust-headless-chrome/blob/master/src/browser/process.rs                                   | rust-headless-chrome/src/browser/process.rs at main - GitHub        |
| https://www.browsercat.com/docs/connect-with/headless-chrome                                                                      | Getting Started with Rust Headless Chrome - BrowserCat              |
| https://lib.rs/crates/spider_chromiumoxide_types                                                                                  | spider_chromiumoxide_types — Rust web dev library // Lib.rs         |
| https://github.com/rust-lang/cargo/issues/11133                                                                                   | Allow publishing with path dependencies without a version (RFC2906) |
| https://docs.rs/oxide-browser-sh                                                                                                  | oxide_browser_sh - Rust - Docs.rs                                   |
| https://crates.io/crates/cargo-workspace-deps                                                                                     | cargo-workspace-deps - crates.io: Rust Package Registry             |
| https://github.com/rust-lang/cargo/issues/5332                                                                                    | Allow to share versions of dependencies between unrelated Cargo ... |
| https://www.reddit.com/r/rust/comments/aqgsxk/announcing_the_headless_chrome_crate_puppeteer/                                     | Announcing the Headless Chrome crate ("Puppeteer for Rust")         |
| https://docs.rs/fob-browser-test                                                                                                  | fob_browser_test - Rust - Docs.rs                                   |
| https://docs.rs/chromiumoxide/latest/chromiumoxide/                                                                               | chromiumoxide - Rust                                                |
| https://doc.rust-lang.org/cargo/commands/cargo-metadata.html                                                                      | cargo metadata - The Cargo Book                                     |
| https://doc.rust-lang.org/cargo/commands/cargo-metadata.html                                                                      | doc.rust-lang                                                       |
| https://docs.rs/chromiumoxide/latest/chromiumoxide/                                                                               | docs                                                                |
| https://docs.rs/headless_chrome/latest/headless_chrome/browser/struct.Browser.html                                                | docs                                                                |
| https://doc.rust-lang.org/cargo/commands/cargo-metadata.html                                                                      | doc.rust-lang                                                       |
| https://github.com/rust-lang/cargo/issues/10636                                                                                   | github                                                              |
| https://github.com/mattsse/chromiumoxide/issues/49                                                                                | github                                                              |
| https://lib.rs/crates/headless_chrome_fork                                                                                        | lib                                                                 |
| https://docs.rs/headless_chrome_fork/latest/headless_chrome/browser/struct.Browser.html                                           | docs                                                                |
| https://dev.to/vhub_systems_ed5641f65d59/headless-browsers-in-rust-chromiumoxide-vs-headlesschrome-vs-the-python-alternative-25e5 | dev                                                                 |
| https://github.com/rust-headless-chrome/rust-headless-chrome/blob/master/src/browser/process.rs                                   | github                                                              |
| https://www.browsercat.com/docs/connect-with/headless-chrome                                                                      | browsercat                                                          |
| https://lib.rs/crates/spider_chromiumoxide_types                                                                                  | lib                                                                 |
| https://github.com/rust-lang/cargo/issues/11133                                                                                   | github                                                              |
| https://docs.rs/oxide-browser-sh                                                                                                  | docs                                                                |
| https://crates.io/crates/cargo-workspace-deps                                                                                     | crates                                                              |
| https://github.com/rust-lang/cargo/issues/5332                                                                                    | github                                                              |
| https://www.reddit.com/r/rust/comments/aqgsxk/announcing_the_headless_chrome_crate_puppeteer/                                     | reddit                                                              |
| https://docs.rs/fob-browser-test                                                                                                  | docs                                                                |

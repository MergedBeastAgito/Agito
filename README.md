# Agentic

Capabilities, adapters and the runtime that leases them. A local agent and swarm: it gathers links,
browses, researches, and organises that work. Nothing here knows about a particular product - the
product consumes this workspace and supplies its own composition root.

---

## The shape of it

One tree and one specification. `Crate/` is a Cargo workspace where the specification and the
implementation are the same artifact, and the scenarios under `scenarios/` are the evidence that
each crate answers the contract its file states. `Documentation/` records the decisions the tree
makes and why.

There used to be two trees. `Package/` held the same capabilities as TypeScript, `scenarios/` was
one input both implementations answered to, and `Scripts/` was a Node gate over the pair. That layer
was deleted once the browser expressions were compiled at build time and every scenario was answered
by a crate. It was not a predecessor kept for reference: it was a second implementation of the same
rules, a second toolchain and a second set of gates, and keeping it meant every behaviour had to be
written twice before either side could be believed. What it established is recorded in
`Documentation/Parity.md`; what it enforced is now a test in `Crate/Agentic/Test/`.

The one TypeScript that remains is under `Crate/*/Source/Script/`: the expressions a driver sends to
a page, compiled by that crate's `build.rs` into the JavaScript the page receives. That is not a
second implementation - it is the page's own code, in the language the page runs, checked by a
compiler instead of hidden inside a `format!` string.

The arrows point inward and nothing imports a root. A capability may import its own port and data
shapes; an adapter implements one port against one mechanism; a workflow composes capabilities; the
composition root binds ports to adapters. A facade re-exports ports and may import any of them, and
only the composition root imports the facade: a facade exists for the consumer outside this
workspace, and a crate inside it that reached for one would be importing its own re-export. A
capability that imports an adapter is a defect rather than a shortcut: it makes the capability
untestable without the provider, which is the one property this workspace exists to keep.
`Crate/Agentic/Test/Boundaries.rs` holds that rule against every crate's manifest and dependency
list, so it fails as a test rather than being a convention.

Every crate carries `module.json`. The field that does the most work is `state`, because it decides
what a harness may warm and what it must revalidate at admission:

- **`stateless`** - nothing to check.
- **`holds-transport`** - a liveness probe, because an open socket can be dead.
- **`holds-identity`** - a freshness check, because a token expires while pooled.
- **`holds-workspace`** - a reset proof, because a dirty workspace answers wrongly.

The lifecycle those states belong to is: prepare reusable infrastructure, lease an isolated context,
bind identity and task, validate credentials and connections and versions, execute within a fresh
budget, persist only authorised artefacts, then destroy or prove a reset. Preparation improves
latency and admission establishes correctness. They are different acts, and merging them is how a
warm pool starts returning wrong answers quickly.

---

## Crate/

A Cargo workspace in the house layout: one directory per crate under `Crate/`, `Source/Library.rs`
as the library root, `Fn/` for operations and `Struct/` for data and contracts, `Source/Script/` for
the TypeScript compiled into the JavaScript sent to a page, PascalCase module names, and one
`Target/` at the workspace root. `Crate/README.md` carries the per-crate dependency lists, and
`Documentation/Script.md` says how a script is compiled and why it is erased rather than transpiled.

- **`agentic-runtime`** - the execution core: `Call`, `Cancellation`, `Clock`, `Pace`, the
  `Bound`/`Race` combinators, and the policy engine. No dependencies.
- **`agentic-adapter`** - the port shape a substrate this workspace did not write implements: the
  `Adapter` trait, the claim check its manifest needs, and a scripted stand-in. No behaviour, and no
  substrate of its own.
- **`agentic-spacetime-catalog`** - the database model as data: six tables, seven rules, and no
  dependency at all, not even the server SDK. No dependencies.
- **`agentic-skill-contract`** - corpus discovery, validation and a stable digest.
- **`agentic-chat-driver`** - `agentic.chat.ask` over a page it declares itself. No browser.
- **`agentic-browser-cdp`** - the concrete substrate; the only crate that names `chromiumoxide`.
- **`agentic-design-verification`** - measurement and evidence, holding no design of its own.
- **`agentic-workflow`** - the bridge from the substrate to the consumer-owned ports, and one
  composition that uses four capabilities.
- **`agentic-sdk`** - the facade: the five crates a consumer would otherwise name, re-exported
  behind one dependency, with no behaviour of its own. `Crate/Sdk/Source/Library.rs` states what may
  appear there and why the substrate, the adapter seam and the policy engine do not.
- **`agentic`** - wiring, the manifest table and the executable.

**A capability's required surface stays runtime-neutral.** The four capability crates do not name
`tokio` and nothing they pull in resolves it: time arrives as a `Call` (a deadline and a
cancellation) and a `Pace` (a clock and a wait), both supplied by the composition root. **No
capability imports an adapter**: each declares the trait it needs, and the workflow owns the
bindings that implement them. Both rules are checkable with `cargo tree`, and both are checked by
`Crate/Agentic/Test/Boundaries.rs`.

The database catalogue is a crate, `agentic-spacetime-catalog`: the six tables as data, the seven
rules as functions over them, and the cases the TypeScript statement carried. It has no dependency
at all - no server SDK, no serialiser, no runtime - because the rules are what a module has to
satisfy rather than a client for one. `Documentation/Spacetime-Catalog.md` is the prose that says
what the crate states, and it records where the two disagreed when the port was made.

---

## scenarios/

One scenario file per capability, plus the timing files, read by the Rust parity suites. Each case
states a fixture, a sequence of steps and the expected result, the ordered effect list and the
number of reports. `Crate/BrowserCDP/Test/Parity.rs`, `Crate/ChatDriver/Test/Parity.rs` and
`Crate/SkillContract/Test/Parity.rs` read them and answer them through scripted fakes, so
`cargo test --workspace` is the whole of the scenario gate.

`effects` is the protocol the implementation spoke, not its answer: every launch, attach,
evaluation, write, send, release and report the fake observed, in order. A port that answers
correctly by the wrong route fails. A case names the unit case that proves it in its `rust` field,
and each parity suite refuses a case that names a test which does not exist.

These files used to be the input to a differential run: a Node referee drove the TypeScript packages
in process, asked cargo for the Rust half, and compared the two observation files. That referee is
gone with the TypeScript side, and with it the claim that the two implementations agree.
`Documentation/Parity.md` records what the comparison established while it existed and what replaced
it - the Rust tests, which are now the only statement of the contract.

---

## Fixture/

One crate outside the workspace, and its own workspace root: `Fixture/Sdk/` depends on `agentic-sdk`
and on nothing else this tree ships. It builds a session, opens `Page/Probe.html` and evaluates an
expression - the whole flow the facade exists to make possible from one dependency. `tokio` is the
only other dependency, because the browser adapter spawns its handler on it, and nothing else is
named: the evaluated value is read with `as_str` and `Display` rather than through a serialiser the
consumer would have to depend on.

```
cargo build --manifest-path Fixture/Sdk/Cargo.toml   # the facade alone
cargo run --manifest-path Fixture/Sdk/Cargo.toml     # needs a browser; AGENTIC_BROWSER names one
cargo tree --manifest-path Fixture/Sdk/Cargo.toml --depth 1
```

It is not a workspace member, and deliberately: a member would resolve its siblings through this
workspace's own graph, which is exactly the thing a consumer cannot do.

---

## The gates

Two checks that used to be Node scripts are Rust tests, because a gate that lives in the toolchain
being deleted is a gate that leaves with it:

- **`Crate/Agentic/Test/Boundaries.rs`** - reads every crate's `module.json` for its kind and its
  `Cargo.toml` for its edges, and fails when a capability names an adapter, an adapter names another
  adapter, anything names the composition root, or anything but the composition root names the
  facade. It also refuses a kind it does not know, so a new crate kind fails loudly rather than
  passing as "anything goes".
- **`Crate/Agentic/Test/Legend.rs`** - fails when a page under `Documentation/Diagram/` carries a
  Mermaid block without the shared legend, when a diagram has no frontmatter, theme or class
  annotation, when a `.svg` has no `.mmd` beside it, or when a render is a placeholder. A page added
  without a legend leaves its terms undefined for the next reader.

Both run in `cargo test --workspace`, which is why the gate below is one command. Two further gates
did not survive the deletion - the feed bundler's determinism and the mutation suite - and
`Documentation/Open-Questions.md` records each with what it held and what is owed.

---

## .hermes/

The record and the local tooling. `research/` holds the captures this workspace came out of, never
edited, with `catalog.mjs` indexing them into `Index.md`. `tools/` holds the feed, and every tool
there is a standalone script that takes a DevTools websocket URL - local tooling, not part of the
build and not required by any gate:

- **`bundle.mjs`** - turn a tree into deterministic batches: `tree.txt`, `batch-NN.txt` per payload,
  and a `manifest.json` of what went where and what was left out.
- **`attach.mjs`** - set those batches on a chat composer's hidden file input through
  `DOM.setFileInputFiles`, the only path that both sets the files and lets the page react to them.
- **`observe.mjs`** - watch a chat page and report only what changed, so a caller waits for the page
  to settle instead of sleeping for a fixed time.
- **`cdp-extract.mjs`** - serialise a rendered answer to Markdown with the maths kept as maths, read
  from `<annotation encoding="application/x-tex">` rather than from `innerText`.
- **`cdp-trace.mjs`** - read a step trace as the tree the page actually nests, and write `id` and
  `parent` on every line instead of counting depth.

---

## Documentation/

The design record. Every claim names its source - a capture in `.hermes/research/` - or is marked as
a decision, and a document that changes its mind says so rather than rewriting the earlier position.

- **`Scope.md`** - what this workspace is and is not, and the parity it claims with the consuming
  product.
- **`Design.md`** - the decisions that shape the tree, argued from the consumer's real surface
  rather than from a taxonomy.
- **`Observability.md`** - why the database is the surface, and where control flow belongs instead.
- **`Reflection.md`** - how a system that can rewrite its own capability definitions stays honest
  about what it is running.
- **`Review.md`** - the external review's verdict and the defect it found.
- **`Rust.md`** - the port: layout, manifest, and the questions it left open.
- **`Script.md`** - how a page script is compiled, which compiler was chosen, and the measurements
  behind it.
- **`Spacetime-Catalog.md`** - the catalogue as rules and data, and why it lives here rather than in
  a module that imports the server SDK.
- **`Parity.md`** - what the scenarios establish, what the differential run established while it
  existed, and what replaced it.
- **`Research.md`** - the index over the captures: what each thread settled, what it left open, and
  which document consumed it.
- **`Open-Questions.md`** - every question still open, with the experiment that would settle it and
  what it blocks.
- **`Diagram/`** - the same design as storyboards: one page per category, each ending with the
  states nobody has built yet, indexed by `Diagram/Index.md`.

---

## Before you start

- **Rust ≥ 1.88** - `Cargo.toml` pins `edition = "2024"` and `rust-version = "1.88"`. That is the
  only toolchain: there is no `node_modules`, no package manager and no formatter to install.

## The gate

```
cargo test --workspace      # every suite, the parity scenarios, the boundaries and the diagrams
cargo clippy --workspace --all-targets -- -D warnings
cargo fmt --all -- --check
```

`cargo test --workspace` never starts Chromium and never opens a socket: a page is a scripted
responder, a browser is a scripted launcher, and a deadline passes because the test advanced a clock
it owns.

---

## Deliberately not here

Domain policy and write authority stay with their consumers until a second workflow demonstrates a
stable contract: store, model, command, vote, ingest, board synchronisation, gateway maintenance. A
generic name is not a port.

# Agentic

Capabilities, adapters and the runtime that leases them. A local agent and swarm: it gathers links,
browses, researches, and organises that work. Nothing here knows about a particular product - the
product consumes this workspace and supplies its own composition root.

---

## The shape of it

Two trees and one specification. `Package/` states what each capability must do, in TypeScript that
runs directly on Node. `Crate/` is the port of that specification into a Cargo workspace.
`Documentation/` records the decisions the two share, `scenarios/` is the evidence that they agree,
and `Scripts/` is the gate that keeps the arrows pointing inward.

The arrows point inward and nothing imports a root. A capability may import its own port and data
shapes; an adapter implements one port against one mechanism; a workflow composes capabilities; the
composition root binds ports to adapters. A capability that imports an adapter is a defect rather
than a shortcut: it makes the capability untestable without the provider, which is the one property
this workspace exists to keep.

Every package and every crate carries `module.json`. The field that does the most work is `state`,
because it decides what a harness may warm and what it must revalidate at admission:

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

## Package/

Six TypeScript packages, one per capability, each with a port, the data shapes it names and a
fake-based suite. They are the specification the Rust port mirrors: a change to a port or a failure
code belongs here first, and `scenarios/` is what holds the other tree to it.

- **`browser-cdp`** - the shared execution substrate: attach versus launch, bounded evaluation, and
  disposal that closes only what it launched.
- **`chat-driver`** - ask a chat product, correlate turns, verify mode and model, then read the
  answer and its step trace.
- **`design-verification`** - measure a built page and produce evidence; the expectations stay with
  the consumer.
- **`runtime`** - admission, leases with a fence, per-run budgets, cancellation, cleanup and
  lifecycle telemetry.
- **`skill-contract`** - discover, validate and package a skill corpus, with a stable digest.
- **`spacetime-catalog`** - the database model as rules and table descriptions: capabilities,
  generations, leases, probes, beliefs and trace.

The suites are fake-based throughout. A page is a scripted responder, a browser is a scripted
launcher, and a deadline passes because the test advanced a clock it owns. Nothing in this tree
starts Chromium or opens a socket.

---

## Crate/

The same specification as a Cargo workspace of seven crates, in the house layout: one directory per
crate under `Crate/`, `Source/Library.rs` as the library root, `Fn/` for operations and `Struct/`
for data and contracts, `Source/Script/` for the TypeScript compiled into the JavaScript sent to a
page, PascalCase module names, and one `Target/` at the workspace root. `Crate/README.md` carries
the per-crate dependency lists, and `Documentation/Script.md` says how a script is compiled and why
it is erased rather than transpiled.

- **`agentic-runtime`** - the execution core: `Call`, `Cancellation`, `Clock`, `Pace`, the
  `Bound`/`Race` combinators, and the policy engine. No dependencies.
- **`agentic-skill-contract`** - corpus discovery, validation and a stable digest.
- **`agentic-chat-driver`** - `agentic.chat.ask` over a page it declares itself. No browser.
- **`agentic-browser-cdp`** - the concrete substrate; the only crate that names `chromiumoxide`.
- **`agentic-design-verification`** - measurement and evidence, holding no design of its own.
- **`agentic-workflow`** - the bridge from the substrate to the consumer-owned ports, and one
  composition that uses four capabilities.
- **`agentic`** - wiring, the manifest table and the executable.

**A capability's required surface stays runtime-neutral.** The three capability crates do not name
`tokio` and nothing they pull in resolves it: time arrives as a `Call` (a deadline and a
cancellation) and a `Pace` (a clock and a wait), both supplied by the composition root. **No
capability imports an adapter**: each declares the trait it needs, and the workflow owns the
bindings that implement them. Both rules are checkable with `cargo tree`, and both are checked in
`Crate/README.md`'s verification block.

`Package/spacetime-catalog` has no crate yet. The port covers the five packages that carry a port;
the catalogue is rules and table descriptions with nothing to execute, and a crate for it would be a
shape the TypeScript side has not settled.

---

## scenarios/

One scenario file per capability, read by both implementations, with the ordered effect list each
one produced. `node scenarios/run.mjs --compare` runs both sides and reports every disagreement;
`scenarios/mutate.mjs` applies the mutations both suites must reject, so a green run is not merely a
green run. `Documentation/Parity.md` says exactly what that evidence establishes and what it does
not.

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
- **`Spacetime-Catalog.md`** - the catalogue as rules and data, and why it lives here rather than in
  a module that imports the server SDK.
- **`Parity.md`** - the evidence that the two implementations are the same program.
- **`Research.md`** - the index over the captures: what each thread settled, what it left open, and
  which document consumed it.
- **`Open-Questions.md`** - every question still open, with the experiment that would settle it and
  what it blocks.
- **`Diagram/`** - the same design as storyboards: one page per category, each ending with the
  states nobody has built yet, indexed by `Diagram/Index.md`.

---

## Scripts/

The gate. Plain Node, no build step and no dependency of their own:

- **`Boundaries.mjs`** - walks every package's source and refuses an import that reverses an arrow
  or crosses a boundary its `module.json` does not declare.
- **`Feed.mjs`** - drives `.hermes/tools/bundle.mjs` over a fixture and asserts that it splits,
  excludes and orders deterministically. A bundler that is not deterministic is a bundler whose
  batches cannot be compared between runs.
- **`Legend.mjs`** - fails when a page under `Documentation/Diagram/` carries a Mermaid block
  without the shared legend, because a page added without one leaves its terms undefined for the
  next reader.

---

## .hermes/

The record and the tooling. `research/` holds the captures this workspace came out of, never edited,
with `catalog.mjs` indexing them into `Index.md`. `tools/` holds the feed, and every tool there is a
standalone script that takes a DevTools websocket URL:

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

## Before you start

- **Node ≥ 22.23.0** - the suites run TypeScript directly by type stripping, so there is no compile
  step and no `tsc` in the gate.
- **pnpm** - `pnpm install` reads `pnpm-workspace.yaml`, which lists the root and `Package/*`.
- **Rust ≥ 1.88** - `Cargo.toml` pins `edition = "2024"` and `rust-version = "1.88"`.

## The gate

```
npm run check          # the boundaries and the bundler
npm run Test           # the six TypeScript suites, fakes only
npm run Format:Check   # prettier, the same rules the sibling repositories use
cargo test --workspace
```

`cargo test --workspace` never starts Chromium and never opens a socket: a page is a scripted
responder, a browser is a scripted launcher, and a deadline passes because the test advanced a clock
it owns.

---

## Deliberately not here

Domain policy and write authority stay with their consumers until a second workflow demonstrates a
stable contract: store, model, command, vote, ingest, board synchronisation, gateway maintenance. A
generic name is not a port.

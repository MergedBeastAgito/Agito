![Merged Beast Agito](https://raw.githubusercontent.com/MergedBeastAgito/.github/refs/heads/main/Sukuna_summons_Merged_Beast_Agito.webp)

# Agito

Capabilities, adapters and the runtime that leases them. A local agent and swarm: it gathers links,
browses, researches, and organises that work. Nothing here knows about a particular product - the
product consumes this workspace and supplies its own composition root.

---

## The shape of it

One tree and one specification. `Crate/` is a Cargo workspace where the specification and the
implementation are the same artifact: each crate's `module.json` states what it is and what it
provides, and `agito manifests` prints that table from the binary. `Documentation/` records the
decisions the tree makes and why.

There used to be two trees. `Package/` held the same capabilities as TypeScript, `Scenario/` was one
input both implementations answered to, and `Script/` was a Node gate over the pair. That layer was
deleted once the browser expressions were compiled at build time. It was not a predecessor kept for
reference: it was a second implementation of the same rules, a second toolchain and a second set of
gates, and keeping it meant every behaviour had to be written twice before either side could be
believed. What it established is recorded in `Documentation/Parity.md`.

The verification layer went the same way: the per-crate test suites, the parity corpus, the
out-of-workspace fixture crate and the path audit are gone, and `Documentation/Parity.md` records
what they held. What remains is what ships - the crates, their manifests, the `agito` binary and the
corpus under `Documentation/Skill` that `agito skills` validates.

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
untestable without the provider, which is the one property this workspace exists to keep. Each
crate's `module.json` states the ports it provides and requires, and `agito verify` reports a module
whose manifest and code disagree.

Every crate carries `module.json`. The field that does the most work is `state`, because it decides
what a harness may warm and what it must revalidate at admission:

- **`Stateless`** - nothing to check.
- **`HoldTransport`** - a liveness probe, because an open socket can be dead.
- **`HoldIdentity`** - a freshness check, because a token expires while pooled.
- **`HoldWorkspace`** - a reset proof, because a dirty workspace answers wrongly.

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

- **`agito-runtime`** - the execution core: `Call`, `Cancellation`, `Clock`, `Pace`, the
  `Bound`/`Race` combinators, and the policy engine. No dependencies.
- **`agito-adapter`** - the port shape a substrate this workspace did not write implements: the
  `Adapter` trait, the claim check its manifest needs, and a scripted stand-in. No behaviour, and no
  substrate of its own.
- **`agito-lane`** - `agito.lane.decide`: a template corpus as data, selected by predicate, composed
  into a decision-model request and read back as typed answers under a metered budget. The round
  trip leaves through a `Poster` the crate declares and never implements, so the suite answers from
  a fixture and the crate opens no socket.
- **`agito-spacetime-catalog`** - the database model as data: six tables, seven rules, and no
  dependency at all, not even the server SDK. No dependencies.
- **`agito-skill-contract`** - corpus discovery, validation and a stable digest.
- **`agito-chat-driver`** - `agito.chat.ask` over a page it declares itself. No browser.
- **`agito-browser-cdp`** - the concrete substrate; the only crate that names `chromiumoxide`.
- **`agito-design-verification`** - measurement and evidence, holding no design of its own.
- **`agito-workflow`** - the bridge from the substrate to the consumer-owned ports, and one
  composition that uses four capabilities.
- **`agito-sdk`** - the facade: the six crates a consumer would otherwise name, re-exported behind
  one dependency, with no behaviour of its own. `Crate/SDK/Source/Library.rs` states what may appear
  there and why the substrate, the browser's own adapter seam and the policy engine do not. The
  sixth is the adapter port, which a substrate outside this tree implements - the one consumer the
  facade cannot reach any other way.
- **`agito`** - wiring, the manifest table and the executable.

**A capability's required surface stays runtime-neutral.** The five capability crates do not name
`tokio` and nothing they pull in resolves it: time arrives as a `Call` (a deadline and a
cancellation) and a `Pace` (a clock and a wait), both supplied by the composition root. **No
capability imports an adapter**: each declares the trait it needs, and the workflow owns the
bindings that implement them. Both rules are checkable with `cargo tree` and stated by each crate's
`module.json`.

The database catalogue is a crate, `agito-spacetime-catalog`: the six tables as data, the seven
rules as functions over them, and the cases the TypeScript statement carried. It has no dependency
at all - no server SDK, no serialiser, no runtime - because the rules are what a module has to
satisfy rather than a client for one. `Documentation/Spacetime-Catalog.md` is the prose that says
what the crate states, and it records where the two disagreed when the port was made.

---

## Build/

One file, and it is included rather than imported: `Crate/Chat/Driver/build.rs` and
`Crate/Workflow/build.rs` each carry a doc comment, `include!("../../../Build/Script.rs")` and one
`main` that calls `CompileScripts`, so the rule for what a script may contain and how it becomes
bytes is written once and cannot drift between the two crates.

`Build/Script.rs` erases rather than transpiles. The file is parsed by a real TypeScript parser, the
spans of the TypeScript-only syntax are found in the AST, and exactly those bytes are removed -
every other byte, including indentation, quotes and comments, is the byte the page already received.
A file with no type syntax compiles to itself, which is why moving the expressions to `.ts` changed
nothing a page sees. What it refuses matters as much as what it emits: an unrecognised construct is
a build error rather than a silently mangled expression, the erased result must still parse as
JavaScript, and every `__PARAM_Name__` marker must have a matching `declare const`, so the parameter
contract cannot drift from the expression it describes. Types are not checked - a transpiler parses,
it does not infer - and `Documentation/Script.md` records what a `tsc --noEmit` pass costs and why
it is not a gate.

---

## What was verified, and is not any more

There is no test suite, no parity corpus and no consumer fixture in this tree. `cargo build`,
`cargo clippy` and `cargo fmt --check` are what a change is held to; nothing else runs, and no
workflow step asserts anything the build did not produce.

The corpus under `Scenario/` stated 82 cases across the browser, the chat driver and the skill
contract, and three parity suites answered them through scripted fakes. `Fixture/SDK/` was a crate
outside the workspace that depended on `agito-sdk` alone, and it existed to prove the facade from a
consumer's side. `Script/AuditPaths.sh` refused an absolute machine path in a tracked file. All
three are deleted; `Documentation/Parity.md` records what they held and what the deletion took.

---

## .hermes/

Removed. The owner's working archive - the research captures, the review bundles and the CDP tooling
that produced them - lived here untracked, and the tree no longer carries it. What the captures
settled survives in `Documentation/Research.md`, which indexes them by id.

The tooling it held is owed a home as verbs rather than scripts: `bundle` (a tree into deterministic
batches), `attach` (set those batches on a composer's file input through `DOM.setFileInputFiles`),
`observe` (report only what changed, so a caller waits for a page to settle), `cdp-extract`
(serialise a rendered answer to Markdown with the maths kept as maths) and `cdp-trace` (read a step
trace as the tree the page nests).

---

## Documentation/

The design record. Every claim names its source, or is marked as a decision, and a document that
changes its mind says so rather than rewriting the earlier position. The research captures the
documents once cited lived under `.hermes/` and were removed with it; what each settled survives in
the document that consumed it.

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
- **`Parity.md`** - what the parity corpus established, what the differential run established while
  it existed, and what the deletion of both took with it.
- **`Research.md`** - the index over the captures: what each thread settled, what it left open, and
  which document consumed it.
- **`Open-Questions.md`** - every question still open, with the experiment that would settle it and
  what it blocks.
- **`Style.md`** - the naming convention: one concept per name, a verb at the leaf, singular role
  folders, and the boundary where something outside the tree fixes a spelling.
- **`Diagram/`** - the same design as storyboards: one page per category, each ending with the
  states nobody has built yet, indexed by `Diagram/Index.md`.

---

## Before you start

- **Rust ≥ 1.88** - `Cargo.toml` pins `edition = "2024"` and `rust-version = "1.88"`, and it is the
  only toolchain the build needs. A Node layer exists for formatting alone: `package.json` carries
  `prettier` and `prettier.config.mjs` holds its settings, so Markdown and JSON are formatted with
  it. Nothing at run time and nothing at build time depends on it, and the build works with no
  `node_modules` present.

## The build

```
cargo build --workspace --release
cargo clippy --workspace --all-targets -- -D warnings
cargo fmt --all -- --check
cargo run -q -p agito -- manifests
```

There is no `cargo test` step: the suites, the corpus and the fixture crate are gone.
`agito manifests` prints the module table, and `agito skills` validates the corpus under
`Documentation/Skill` - that is the whole of what this tree checks about itself.

---

## Deliberately not here

Domain policy and write authority stay with their consumers until a second workflow demonstrates a
stable contract: store, model, command, vote, ingest, board synchronisation, gateway maintenance. A
generic name is not a port.

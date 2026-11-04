# Rust workspace

Nine crates: the port shapes and the failure classification this workspace settled on, and now the
only statement of them. The TypeScript packages they were ported from were deleted once the page
expressions were compiled at build time and every scenario was answered here -
`Documentation/Parity.md` records what the comparison established while it existed, and what
replaced it.

The layout follows the house style of `github.com/PlayForm/Run`: one directory per crate under
`Crate/`, `Source/Library.rs` as the library root, `Fn/` for operations and `Struct/` for data and
contracts, PascalCase module names, and one `Target/` at the workspace root.

`Source/Script/` is where the TypeScript lives that the driver and the workflow send to a page: one
file per expression, compiled by the crate's `build.rs` into the JavaScript a page receives and
baked in by `script!`. A file's `declare const __PARAM_Name__: T;` lines are both the parameter
types and the list the build holds a call site to - a parameter declared `string` is escaped into a
JavaScript string literal, any other declared type is spliced as code - so an editor, a linter and a
compiler see the expression that a `format!` used to hide, and the compiled bytes are the bytes the
page was already being sent. `Build/Script.rs` is the one statement of how a script is compiled;
both `ChatDriver` and `Workflow` include it from their `build.rs`. `Documentation/Script.md` records
the compiler choice and the measurements behind it.

The TypeScript parser is a **build-only** dependency (`[build-dependencies]` in those two crates,
pinned exactly): nothing at run time links it, no `node_modules` is involved, and a build works
offline once the crates are fetched.

---

## Crates

**`agentic-runtime`** - `Crate/Runtime/`, library `AgenticRuntime`. The execution core: `Call`,
`Cancellation`, `Clock`, `Pace`, the `Bound`/`Race` combinators, the std `BlockOn`, and the policy
engine (admission, leases with a fence, per-run budgets, release that never fails). Dependencies:
none. An optional `tokio` feature adds a `Pace` implementation and is off by default.

**`agentic-spacetime-catalog`** - `Crate/SpacetimeCatalog/`, library `AgenticSpacetimeCatalog`. The
database model as data and rules: six tables described as records, seven reducers over a five-member
`Context`, and the three refusal codes a module's manifest declares. Dependencies: none - not the
server SDK, not a serialiser, not the runtime, because a rule is what a module has to satisfy rather
than a client for one.

**`agentic-skill-contract`** - `Crate/SkillContract/`. Corpus discovery, validation and a stable
digest. Dependencies: `agentic-runtime`, `serde`, `serde_json`, `sha2`.

**`agentic-chat-driver`** - `Crate/ChatDriver/`. `agentic.chat.ask` over a page it declares itself.
Dependencies: `agentic-runtime`, `serde_json`. No browser, no executor.

**`agentic-browser-cdp`** - `Crate/BrowserCDP/`. The concrete substrate: attach versus launch,
bounded evaluation, disposal that closes only what it launched. Dependencies: `agentic-runtime`,
`serde_json`, `tokio`, `tokio-util`, `futures-util`, `chromiumoxide`. The only crate that names
`chromiumoxide`.

**`agentic-design-verification`** - `Crate/DesignVerification/`. Measurement and evidence, holding
no design of its own. Dependencies: `agentic-runtime`, `serde_json`.

**`agentic-workflow`** - `Crate/Workflow/`. The bridge from the substrate to the two consumer-owned
ports, and one composition that uses four capabilities. Dependencies: `agentic-runtime`,
`agentic-chat-driver`, `agentic-design-verification`, `agentic-skill-contract`,
`agentic-browser-cdp`, `serde_json`.

**`agentic-sdk`** - `Crate/Sdk/`. The facade: the stable surface of the five crates a consumer would
otherwise name, re-exported behind one dependency. It adds no behaviour - every item is a path over
somebody else's - and its dependency list is the enforcement: five ports, no substrate, no executor.
Dependencies: `agentic-runtime`, `agentic-browser-cdp`, `agentic-chat-driver`,
`agentic-design-verification`, `agentic-skill-contract`.

**`agentic`** - `Crate/Agentic/`. Wiring, the manifest table and the executable. Dependencies: all
seven above, `clap`, `tokio`, `serde_json`.

---

## The three rules the dependency graph enforces

**A capability's required surface is runtime-neutral.** `agentic-chat-driver`,
`agentic-skill-contract`, `agentic-design-verification` and `agentic-spacetime-catalog` do not name
`tokio` in their dependency lists, and nothing they pull in resolves it. Time arrives as a `Call` (a
deadline and a cancellation) and a `Pace` (a clock and a wait) supplied by the composition root.

**No capability imports an adapter.** `agentic-chat-driver` declares its own `Page` trait and
`agentic-design-verification` its own `Browse`; `agentic-workflow` owns the bindings that implement
them over `agentic-browser-cdp`'s `Browse`. There is no shared ports crate, and the translations are
deliberately not factored into a "universal browser request" type.

**A facade names ports and nothing else.** `agentic-sdk`'s dependency list is five crates and no
substrate, no executor and no serialiser, and the direction is one-way: the facade may import any
port, and only the composition root may import the facade. A crate inside the workspace that reached
for it would be importing its own re-export.

All three are checkable: the first two with `cargo tree`, the third by `Crate/Sdk/Test/Facade.rs`
reading the facade's own dependency list, and the direction by `Test/Boundaries.rs` with every other
arrow.

---

## Verification

```
cargo test --workspace                                   # every suite, no browser, no network
cargo run -p agentic -- --help                           # the composition root's commands
cargo build --manifest-path Fixture/Sdk/Cargo.toml       # a consumer of the facade alone
cargo tree -p agentic-chat-driver | grep -c tokio         # 0
cargo tree -p agentic-chat-driver | grep -c browser       # 0
cargo clippy --workspace -- -D warnings
cargo fmt --check
```

The tests are fake-based: a page is a scripted responder, a browser is a scripted launcher, and a
deadline passes because the test advanced a clock it owns. `cargo test` never starts Chromium and
never opens a socket. The one thing that does open a real browser is `Fixture/Sdk/`, and it is run by
hand rather than by the suite: it exists to prove that a consumer naming one dependency can perform
the whole flow, and a machine without Brave should fail that by name rather than turn every other
test red.

Two of those tests are the gates that used to be Node scripts: `Test/Boundaries.rs` reads every
crate's manifest and dependency list and fails on an edge the design does not allow, and
`Test/Legend.rs` holds `Documentation/Diagram/` to the shared legend. Both are described in the
workspace README.

`agentic verify --module <name>` is the manifest check: it compares the errors a manifest names
against the codes its crate's own type declares, so a manifest that has drifted from its code fails
rather than being read by a harness.

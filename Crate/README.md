# Rust workspace

Every crate here is a statement of the port shapes and the failure classification this workspace
settled on, and now the only statement of them. The TypeScript packages they were ported from were
deleted once the page expressions were compiled at build time and every scenario was answered here -
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
both `Crate/Chat/Driver` and `Crate/Workflow` include it from their `build.rs`. `Documentation/Script.md` records
the compiler choice and the measurements behind it.

The TypeScript parser is a **build-only** dependency (`[build-dependencies]` in those two crates,
pinned exactly): nothing at run time links it, no `node_modules` is involved, and a build works
offline once the crates are fetched.

---

## Crates

**`agito-runtime`** - `Crate/RunTime/`, library `RunTime`. The execution core: `Call`,
`Cancellation`, `Clock`, `Pace`, the `Bound`/`Race` combinators, the std `Block`, and the policy
engine (admission, leases with a fence, per-run budgets, release that never fails). Dependencies:
none. An optional `tokio` feature adds a `Pace` implementation and is off by default.

**`agito-adapter`** - `Crate/Adapter/`. The port a third-party substrate implements: `Adapter`
(`Port`, `Call`, `Dispose`), `Serves`, and a scripted `StandIn` for a consumer's own tests. It ships
no implementation - the workspace already has one substrate - and declares no error codes, because a
code belongs to the adapter that reports it rather than to the shape it implements. Dependencies:
`agito-runtime`, `serde_json`.

**`agito-lane`** - `Crate/Lane/`, library `Lane`. `agito.lane.decide`: a template corpus
as data - loaded, selected by predicate, composed into a decision-model request and read back as
typed answers under a metered budget. The round trip leaves through a `Poster` the crate declares
and never implements, so the suite answers from a fixture and the crate itself opens no socket.
Dependencies: `agito-runtime`, `serde`, `serde_json`.

**`agito-spacetime-catalog`** - `Crate/Spacetime/Catalog/`, library `Catalog`. The
database model as data and rules: six tables described as records, seven reducers over a five-member
`Context`, and the three refusal codes a module's manifest declares. Dependencies: none - not the
server SDK, not a serialiser, not the runtime, because a rule is what a module has to satisfy rather
than a client for one.

**`agito-skill-contract`** - `Crate/Skill/Contract/`. Corpus discovery, validation and a stable
digest. Dependencies: `agito-runtime`, `serde`, `serde_json`, `sha2`.

**`agito-chat-driver`** - `Crate/Chat/Driver/`. `agito.chat.ask` over a page it declares itself.
Dependencies: `agito-runtime`, `serde_json`. No browser, no executor.

**`agito-browser-cdp`** - `Crate/Browser/CDP/`. The concrete substrate: attach versus launch,
bounded evaluation, disposal that closes only what it launched. Dependencies: `agito-runtime`,
`serde_json`, `tokio`, `tokio-util`, `futures-util`, `chromiumoxide`. The only crate that names
`chromiumoxide`.

**`agito-design-verification`** - `Crate/Design/Verification/`. Measurement and evidence, holding
no design of its own. Dependencies: `agito-runtime`, `serde_json`.

**`agito-workflow`** - `Crate/Workflow/`. The bridge from the substrate to the two consumer-owned
ports, and one composition that uses four capabilities. Dependencies: `agito-runtime`,
`agito-chat-driver`, `agito-design-verification`, `agito-skill-contract`,
`agito-browser-cdp`, `serde_json`.

**`agito-sdk`** - `Crate/SDK/`. The facade: the stable surface of the six crates a consumer would
otherwise name, re-exported behind one dependency. It adds no behaviour - every item is a path over
somebody else's - and its dependency list is the enforcement: six ports, no substrate, no executor.
The sixth is `agito-adapter`, which ships no port of its own but is the shape a substrate outside
this tree implements: a third-party adapter depends on the facade and on nothing else, so the port
it implements has to be reachable here. Dependencies: `agito-runtime`, `agito-browser-cdp`,
`agito-chat-driver`, `agito-design-verification`, `agito-skill-contract`, `agito-adapter`.

It does not carry the four ports outside that flow - `agito.lane.decide`, `agito.catalog.rules`,
`agito.workflow`, `agito.compose` - and that is curation rather than an omission. Those four are
what the composition root asks for rather than what a consumer calls, and `agito-workflow` is the
sharpest case: it depends on `agito-browser-cdp`, so re-exporting it would hand every consumer the
substrate choices this crate exists to hide. The set is exactly the six, and widening it is an edit
to the facade rather than a line that arrives with the next crate.

**`agito`** - `Crate/Agito/`. Wiring, the manifest table and the executable. Dependencies: all
nine above, `clap`, `tokio`, `serde_json` and `libc` - the binary holds no adapter, because
`agito verify` decides whether a manifest is well formed without running the code behind it.

---

## The three rules the dependency graph enforces

**A capability's required surface is runtime-neutral.** `agito-chat-driver`,
`agito-skill-contract`, `agito-design-verification`, `agito-spacetime-catalog` and
`agito-lane` do not name `tokio` in their dependency lists, and nothing they pull in resolves it.
Time arrives as a `Call` (a deadline and a cancellation) and a `Pace` (a clock and a wait) supplied
by the composition root.

**No capability imports an adapter.** `agito-chat-driver` declares its own `Page` trait and
`agito-design-verification` its own `Browse`; `agito-workflow` owns the bindings that implement
them over `agito-browser-cdp`'s `Browse`. There is no shared ports crate, and the translations are
deliberately not factored into a "universal browser request" type. The one shared trait in the tree,
`agito-adapter`'s `Adapter`, is not a capability's port: it is the shape a substrate _outside_
this workspace implements, so that a third party's driver can be held as `Arc<dyn Adapter>` without
the composition root naming the crate that implements it. `Documentation/Adapter.md` is the
contract.

**A facade names ports and nothing else.** `agito-sdk`'s dependency list is six crates and no
substrate, no executor and no serialiser, and the direction is one-way: the facade may import any
port, and only the composition root may import the facade. A crate inside the workspace that reached
for it would be importing its own re-export. A crate _outside_ the workspace is the case the facade
exists for, and it is the one a third-party adapter is: it names the facade, implements the port the
facade re-exports, and names nothing else.

All three are checkable with `cargo tree`: the facade's own dependency list is what a consumer
resolves, and the arrow direction is the graph it prints.

---

## Verification

```
cargo build --workspace --release                        # the tree, and the binary
cargo run -p agito -- --help                           # the composition root's commands
cargo run -p agito -- manifests                        # the module table
cargo tree -p agito-chat-driver | grep -c tokio         # 0
cargo tree -p agito-chat-driver | grep -c browser       # 0
cargo clippy --workspace --all-targets -- -D warnings
cargo fmt --all -- --check
```

There is no test suite. The per-crate suites, the parity corpus and the out-of-workspace fixture
crate are deleted, and what is left to check is that the tree builds, that the dependency graph
still shows no capability naming an adapter, and that the manifest table still prints.

`agito verify --module <name>` is the manifest check: it compares the errors a manifest names
against the codes its crate's own type declares, so a manifest that has drifted from its code fails
rather than being read by a harness. A module the binary does not carry - a third-party adapter - is
read from a `module.json` under `--adapter` or `AGITO_ADAPTER_PATH`, and its report says
`code: null` because there is no code here to compare against. `Documentation/Adapter.md` states
what that report does check.

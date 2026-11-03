# Rust workspace

The TypeScript toolkit in `packages/` rebuilt as seven crates, keeping the port shapes and the
failure classification the TypeScript suites already enforce. The TypeScript packages stay as they
are: they are the specification, not a predecessor to delete.

The layout follows the house style of `github.com/PlayForm/Run`: one directory per crate under
`Crate/`, `Source/Library.rs` as the library root, `Fn/` for operations and `Struct/` for data and
contracts, PascalCase module names, and one `Target/` at the workspace root.

---

## Crates

**`agentic-runtime`** - `Crate/Runtime/`, library `AgenticRuntime`. The execution core: `Call`,
`Cancellation`, `Clock`, `Pace`, the `Bound`/`Race` combinators, the std `BlockOn`, and the policy
engine (admission, leases with a fence, per-run budgets, release that never fails). Dependencies:
none. An optional `tokio` feature adds a `Pace` implementation and is off by default.

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

**`agentic`** - `Crate/Agentic/`. Wiring, the manifest table and the executable. Dependencies: all
six above, `clap`, `tokio`, `serde_json`.

---

## The two rules the dependency graph enforces

**A capability's required surface is runtime-neutral.** `agentic-chat-driver`,
`agentic-skill-contract` and `agentic-design-verification` do not name `tokio` in their dependency
lists, and nothing they pull in resolves it. Time arrives as a `Call` (a deadline and a
cancellation) and a `Pace` (a clock and a wait) supplied by the composition root.

**No capability imports an adapter.** `agentic-chat-driver` declares its own `Page` trait and
`agentic-design-verification` its own `Browse`; `agentic-workflow` owns the bindings that implement
them over `agentic-browser-cdp`'s `Browse`. There is no shared ports crate, and the translations are
deliberately not factored into a "universal browser request" type.

Both are checkable, and the checks are the first two commands below.

---

## Verification

```
cargo test --workspace                                   # 161 tests, no browser, no network
cargo run -p agentic -- --help                           # the composition root's commands
cargo tree -p agentic-chat-driver | grep -c tokio         # 0
cargo tree -p agentic-chat-driver | grep -c browser       # 0
cargo clippy --workspace -- -D warnings
cargo fmt --check
```

The tests are fake-based, as the TypeScript suites are: a page is a scripted responder, a browser is
a scripted launcher, and a deadline passes because the test advanced a clock it owns. `cargo test`
never starts Chromium and never opens a socket.

`agentic verify --module <name>` is the manifest check: it compares the errors a manifest names
against the codes its crate's own type declares, so a manifest that has drifted from its code fails
rather than being read by a harness.

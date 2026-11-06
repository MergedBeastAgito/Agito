![Merged Beast Agito](https://raw.githubusercontent.com/MergedBeastAgito/.github/refs/heads/main/Sukuna_summons_Merged_Beast_Agito.webp)

# Agito

Agito drives a browser and the chat products in it from Rust. One binary opens a page and evaluates
an expression in it, asks a chat product a question and reads the answer back, measures what a page
rendered, and validates a corpus of skill documents. The same work is a Cargo workspace a product
embeds through one crate.

Every call is bounded. A deadline and a cancellation travel with each operation, a session attached
to a browser somebody else owns is disposed without closing it, and each crate states in its own
`module.json` what it provides, what it requires and what has to be checked before it is reused.

---

## What it does

`agito` is one binary with six verbs. Three of them need no browser; the other three build a session
and hand it to a capability.

- **`agito browse`** - open a page and evaluate one expression in it, then print the value.
  `--attach` uses a browser already listening on a CDP endpoint instead of launching one, and
  `--bind` takes the tab that browser already has, by target id or URL prefix, instead of opening a
  new one. The answer is JSON: the URL that answered, whether the session was launched or attached,
  and the value. The whole operation is bounded by `--timeout`.

- **`agito chat`** - ask a chat product one question and print the answer once it settles. The
  driver reads the composer before it types: the mode and the model the page reports are checked
  against what the call demands, and a mismatch is a refusal rather than a question sent to the
  wrong place. It waits for the product's own completion signal plus stable text, and returns the
  answer with the model, the mode, the step trace that produced it and the sources it cited.
    - `--format text` is what the page rendered; `markdown` is the page's own serialisation, so a
      table keeps its columns, a code block keeps its fence and a formula keeps its TeX; `both`
      takes the two from a single read, so the pair cannot describe two different moments.
    - `--format note` prints a research note: frontmatter, a table of the same facts, then the
      transcript. A note is read as Markdown whatever the selector file says, because a formula read
      as rendered text arrives one glyph per line.
    - `--clear` empties a composer left holding a draft, using the platform's own select-all chord
      and a delete, and exits non-zero when text remains. `--probe` reports what the composer row
      holds and types nothing, which is how a build that moved the model control is diagnosed.
    - `--observe` streams every read of the thread to stderr while the answer is being produced.

- **`agito design`** - measure a page's named elements and print what was read. Each `--selector` is
  answered with `present` and the value it carried, so a selector that matches nothing is an
  observation rather than a failure. An absent element always carries a null value, so a stale
  number cannot travel with `present: false`.

- **`agito skills`** - discover, validate and digest a corpus of skill documents. It checks that
  each `SKILL.md` parses, that its `name` matches its directory, that every file under `Reference/`
  is named in `Reference/Index.md`, and that the corpus index names every skill. It prints the root,
  the skill count, any findings and a digest; `--json` prints the whole record. No browser is
  involved. Absent a root argument it reads beside the crate, then `AGITO_SKILL_PATH`, then the
  installed location.

- **`agito manifests`** - print the module table: every crate this workspace ships with the port it
  provides, the verbs it answers, its state class and the origin `workspace`, followed by every
  third-party module the adapter roots declare, with the file each row was read from.

- **`agito verify`** - report one module's readiness and whether its manifest still matches its
  code. The error codes a `module.json` declares are compared against the codes its crate defines,
  so a manifest that has drifted fails rather than being read by a harness. A module the binary does
  not carry is read from its own `module.json` under `--adapter` or `AGITO_ADAPTER_PATH` and
  reported with `code: null`, because there is no code here to compare against.

---

## Build it

Rust 1.88 or newer: `Cargo.toml` pins `edition = "2024"` and `rust-version = "1.88"`. A browser is
needed by `browse`, `chat` and `design` only; the three checking verbs run on a machine with none.

```sh
cargo build --workspace --release
cargo run -q -p agito -- manifests
```

`cargo clippy --workspace --all-targets -- -D warnings` and `cargo fmt --all -- --check` are the
other two checks the tree holds itself to. A Node toolchain is present for `prettier` alone, which
formats Markdown and JSON; nothing at run time or build time depends on it, and the build works with
no `node_modules` present.

---

## Use it

Read one value out of a page this tool opens:

```sh
agito browse --url https://example.com --expression 'document.title'
```

Take over the tab a running browser already has, rather than opening another one:

```sh
agito browse --attach http://127.0.0.1:9222 --bind example.com --expression 'document.title'
```

Ask a chat product a question and keep the answer as a research note:

```sh
agito chat --url https://chat.example/search/THREAD_ID --question "What changed in the spec?" \
	--format note --thread THREAD_ID --title "Spec delta"
```

Measure what a page actually rendered:

```sh
agito design --url http://127.0.0.1:9999 --selector 'h1' --selector '.card'
```

Check a corpus and the module table:

```sh
agito skills Documentation/Skill
agito manifests
agito verify --module @agito/browser-cdp
```

---

## The crates

Eleven crates under `Crate/`: the capabilities, one substrate, the port a third-party substrate
implements, one workflow, the facade and the composition root.

- **`agito-runtime`** (`Crate/RunTime`) - the execution core. `Call` carries a deadline and a
  cancellation, `Pace` carries a clock and a wait, `Bound` and `Race` combine calls, and the policy
  engine holds admission, leases with a fence, per-run budgets and a release that never fails. No
  dependencies; an optional `tokio` feature supplies a `Pace` and is off by default.

- **`agito-browser-cdp`** (`Crate/Browser/CDP`) - the concrete substrate. Attach to a browser or
  launch one, evaluate an expression under a budget, drive the protocol's input domain, and dispose
  of a session while closing only the browser it launched. The only crate that names
  `chromiumoxide`.

- **`agito-chat-driver`** (`Crate/Chat/Driver`) - `agito.chat.ask` over a page it declares itself:
  the mode and model preconditions, the typing path that is a real edit, completion, the answer read
  as text or Markdown, the step trace and the research note. No browser of its own.

- **`agito-design-verification`** (`Crate/Design/Verification`) - measurement and evidence, holding
  no design of its own. `Measure` reads a page and returns one entry per selector; `Check` is pure
  and answers a verdict per invariant. Nothing here names a selector, a length or a colour.

- **`agito-skill-contract`** (`Crate/Skill/Contract`) - corpus discovery, validation and a stable
  digest over names, descriptions and reference lists, so the digest changes when a skill is added
  or renamed and not when only its body text is.

- **`agito-lane`** (`Crate/Lane`) - `agito.lane.decide`: a template corpus as data, selected by
  predicate, composed into a decision-model request and read back as typed answers under a metered
  budget. The round trip leaves through a `Poster` the crate declares and never implements, so the
  crate itself opens no socket.

- **`agito-spacetime-catalog`** (`Crate/Spacetime/Catalog`) - the database model as data: six
  tables, seven rules over a five-member context, and the refusal codes a module declares. No
  dependency at all, not even the server SDK, because a rule is what a module has to satisfy rather
  than a client for one.

- **`agito-adapter`** (`Crate/Adapter`) - the port a substrate this workspace did not write
  implements: the `Adapter` trait, the `Serves` check and a scripted `StandIn` for a consumer's own
  tests. It ships no behaviour and no substrate of its own.

- **`agito-workflow`** (`Crate/Workflow`) - the bridge from the substrate to the ports a capability
  declares, and one composition that uses four capabilities.

- **`agito-sdk`** (`Crate/SDK`) - the facade: the six crates a consumer would otherwise name,
  re-exported behind one dependency. It adds no behaviour, and its dependency list is the
  enforcement - no substrate, no executor, no serialiser.

- **`agito`** (`Crate/Agito`) - the composition root: the wiring, the manifest table and the
  executable.

---

## The manifest

Every crate carries a `module.json` beside its `Cargo.toml`: a name, a kind, a version, the port it
provides, the ports it requires, the artefact it builds, a state class, a cost and the command that
verifies it. `agito manifests` prints that table from the binary, and `agito verify` compares the
errors a manifest names against the codes its crate defines.

The state class decides what may be reused and what has to be revalidated at admission:

- **`Stateless`** - nothing to check.
- **`HoldTransport`** - a liveness probe, because an open socket can be dead.
- **`HoldIdentity`** - a freshness check, because a token expires while pooled.
- **`HoldWorkspace`** - a reset proof, because a dirty workspace answers wrongly.

---

## Embed it

A consumer names one crate:

```toml
[dependencies]
agito-sdk = { path = "../Agito/Crate/SDK" }
```

`agito-sdk` re-exports the port traits, the data they take and return, the constructor that binds a
port to a substrate, the manifest table and the `script!` macro, which turns the files under a
crate's `Source/Script/` into the expression string `Evaluate` takes.

A substrate this workspace did not write - another protocol, another device, a game port - is an
`Adapter`: it states the port it provides, answers one request under the caller's deadline, and
disposes without failing. It depends on the facade and on nothing else.

The worked example is [`Use/NFSU2`](https://github.com/MergedBeastAgito/Use/tree/Current/NFSU2): a
driver that holds the browser port and a decision model, and a wasm reader that reads the game's
memory from inside the page.

---

## Layout

```
Crate/                      the workspace, one directory per crate
  Adapter/                  the port a third-party substrate implements
  Agito/                    the composition root and the `agito` binary
  Browser/CDP/              the CDP substrate
  Chat/Driver/              asking a chat product a question
  Design/Verification/      measuring a page
  Lane/                     decision-model requests
  RunTime/                  calls, budgets, leases, admission
  SDK/                      the facade
  Skill/Contract/           skill corpus discovery and validation
  Spacetime/Catalog/        the database model as data and rules
  Workflow/                 the bridge from the substrate to the ports
Build/Script.rs             how a page expression is compiled, included by two build.rs
Documentation/              the design record, the skill corpus and the diagrams
```

Every crate follows one layout: `Source/Library.rs` as the library root, `Fn/` for operations,
`Struct/` for data and contracts, `Source/Script/` for the TypeScript a page receives, PascalCase
module names, and one `Target/` at the workspace root.

---

## Documentation

`Documentation/` holds the design record, the skill corpus `agito skills` validates and the
diagrams. `Documentation/Scope.md` covers what the workspace does and does not take on,
`Documentation/Design.md` records the decisions behind the tree, and `Crate/README.md` carries the
per-crate dependency lists and the three rules the dependency graph enforces.

---

## Licence

CC0 1.0 Universal. See [`LICENSE`](LICENSE).

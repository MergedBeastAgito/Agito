---
name: crate-extension
description: Use when adding a crate to this workspace. Layout, manifest, member entry, arrows.
version: 1.0.0
license: MIT
platforms: [macos, linux]
metadata:
    hermes:
        tags: [rust, cargo, workspace, module-json, manifest, crate]
        related_skills: [embedded-source, script-compilation]
---

# Crate Extension

A crate that compiles is not a crate this workspace can see. `agito manifests` prints its row from
the `module.json` beside its `Cargo.toml`, `agito verify --module` compares that file against the
crate's own `Code` type, and the root `members` list is the only statement of what ships - so a
crate added without those three builds, passes its own tests, and is missing from every gate that
decides whether the tree is coherent.

## Where a capability belongs

Stop at the first rung that fits. Each rung below the first costs files that have to be built and
verified forever.

1. **The crate that already owns the neighbouring nouns.** List the crates and their
   `Source/Struct/` trees before proposing anything. A formatter for one chat product's output is
   `Struct/Note.rs` and `Fn/Note.rs` in `Crate/Chat/Driver`, which already holds `Format`,
   `Settlement` and `Fn/Read.rs`. The missing piece is one noun and one verb, not a crate.
2. **A new crate under `Crate/`**, when the concern has no owner - a new substrate, protocol or
   device.
3. **Nothing else.** Not a script beside the crate, not a helper under a tooling directory, not a
   copy outside the repository. A file that is not in a crate is invisible to `agito manifests`,
   drifts from the code it duplicates, and is the first thing deleted when the tree is cleaned.

**Diagnostic that finds rung 1 fast:** grep the crates for the vocabulary of the job (`Format`,
`Settle`, `Read`, `Note`, `Parse`). A crate that already parses the same class of thing is where the
job goes. A capability needing its own executable or its own language runtime is rung 2; "the same
thing, one more product" is rung 1. Language follows the rung: Rust in `Crate/`, TypeScript only
where a page executes it.

**The API is the capability; a flag is only a door to it.** Other crates import this one and may
need a shape the command line never exposes, so the capability lands in the public surface first and
a flag is a thin selector over the same call. A flag that reaches a private path passes every test
and is still unusable by the composition root.

## Layout

```
Crate/<Group>/<Leaf>/
	Cargo.toml
	module.json
	Source/Library.rs        # crate doc, `pub mod Fn; pub mod Struct;`, re-exports
	Source/Main.rs           # only when the crate ships a binary
	Source/Fn.rs             # `pub mod <Verb>;` per verb
	Source/Fn/<Verb>.rs
	Source/Struct.rs         # `pub mod <Noun>;` per noun
	Source/Struct/<Noun>.rs
	Source/Script/           # only when the crate sends TypeScript to a page
	Test/
```

The group folder carries the object and the leaf carries the library name, because a Rust crate root
cannot be nested: `Crate/Browser/CDP` is library `CDP`, `Crate/Skill/Contract` is `Contract`. The
package id is the lowercase kebab form - `agito-browser-cdp` - and the directory never is.
PascalCase throughout, with `#![allow(non_snake_case)]` at the crate root; `Documentation/Style.md`
states the rule and its boundaries.

## `Cargo.toml`, and the two edits outside the crate

```toml
[package]
name = "agito-<kebab>"
version.workspace = true
edition.workspace = true
rust-version.workspace = true
license.workspace = true
publish.workspace = true
description = "One sentence: what the crate is, and what it deliberately is not."

[lib]
name = "<Leaf>"
path = "Source/Library.rs"

[dependencies]
agito-runtime = { workspace = true }

[lints.rust]
non_snake_case = "allow"
non_upper_case_globals = "allow"

[lints.clippy]
upper_case_acronyms = "allow"
```

Then two edits to the root `Cargo.toml`, both written out by hand:

- **`members`** - one line per crate, in alphabetical order. A glob is a second statement of what
  ships that changes when a directory is added: `Crate/*` stops at the group folders, and
  `Crate/*/*` reaches the crates but says nothing about which ones are meant to be there.
- **`[workspace.dependencies]`** - an entry with a version beside its path, tracking
  `[workspace.package] version`. `cargo package` refuses a path dependency that names no version,
  and the release workflow runs it.

## `module.json`

```json
{
	"Name": "@agito/<kebab>",
	"Kind": "Capability",
	"Version": "0.1.3",
	"Provide": {
		"Port": "agito.<area>.<verb>",
		"Error": ["<code>", "<code>"]
	},
	"Require": { "Port": ["agito.runtime", "agito.browse"] },
	"Artefact": {
		"Entry": "Target/debug/lib<Leaf>.rlib",
		"Hash": null,
		"RunTime": "rust>=1.88"
	},
	"State": "Stateless",
	"Cost": { "ColdMs": 30, "WarmMs": 5, "Network": false },
	"Verify": "agito verify --module @agito/<kebab>"
}
```

- **`Kind` is a closed vocabulary** - `RunTime`, `Capability`, `Adapter`, `Workflow`, `Composition`,
  `Facade`, in that layer order. A kind outside it is a manifest the binary cannot reason about.
- **`State` is one of four** - `Stateless`, `HoldTransport`, `HoldIdentity`, `HoldWorkspace`. The
  enum variants are `HoldsTransport` and so on; the manifest string drops the `s` and `verify`
  compares the string, so copy the four from a working manifest rather than from the enum.
- **`Provide.Error` is a claim, not a comment.** `verify` compares it with `Code::All()` of the
  crate's own type. A crate with no error vocabulary - a workflow, the composition root, the adapter
  port - declares `"Error": []`, and its manifest must too.
- **`Provide.Verb`** names a second call the crate accepts beside its port: `agito.chat.note` beside
  `agito.chat.ask`. A verb declared there that nothing reads is a promise made to nobody.
- **Keys are PascalCase singular** - `Provide`, `Require`, `Error`, `Verb`, `Port` - while dotted
  port ids and scoped package ids stay lowercase, because they are wire identifiers rather than
  names a reader scans.
- **`Artefact.Entry` names the debug artefact.** `.cargo/config.toml` sets `target-dir = "Target"`,
  so a release build writes `Target/release/` and a check looking for the manifest's entry after
  `cargo build --release` finds nothing. That is the profile, not a broken build.

**A shipped crate must be added to the readiness table.** `Crate/Agito/Source/Fn/Module/Verify.rs`
holds a `Declared` function that maps each `@agito/<kebab>` to its crate's `Code::All()`. A crate
missing from that match is reported as foreign - `code: null`, checked only against the kind
vocabulary, the state classes and the shape of its error list. Nothing fails; the comparison that
makes the error list a claim never runs.

## The arrows

- **A capability's required surface is runtime-neutral.** Time arrives as a `Call` (a deadline and a
  cancellation) and a `Pace` (a clock and a wait) supplied by the composition root. No `tokio` in
  the dependency list, and nothing the crate pulls in resolves one.
- **A capability never names an adapter.** A crate that needs a page declares its own trait -
  `Page`, `Browse` - and the bindings over `agito-browser-cdp` live in `Crate/Workflow`. There is no
  shared ports crate, and the translations are deliberately not factored into one universal request
  type.
- **A facade names ports and nothing else.** `agito-sdk` re-exports six crates and no substrate, no
  executor and no serialiser; only the composition root imports it.
- **A substrate from outside this workspace implements `Crate/Adapter`'s `Adapter`**, declares
  `"Kind": "Adapter"` in its own manifest, and is read from `--adapter` or `AGITO_ADAPTER_PATH`.

## Gates

```sh
cargo build --workspace --release
cargo clippy --workspace --all-targets -- -D warnings
cargo fmt --all -- --check
cargo run -q -p agito -- manifests                  # one row per module, Ready: true
cargo run -q -p agito -- verify --module <id>       # the manifest against the crate's own codes
cargo tree -p <crate> | grep -c tokio               # 0 for a capability
```

The tree is held to three things: it builds, the graph still shows no capability naming an adapter,
and the manifest table still prints. Run a first release build in the background - a crate that
pulls `chromiumoxide` takes minutes - and check for the artefact rather than blocking a turn.

## Pitfalls

- **A capability with no flag is invisible work.** A noun and a verb can land, pass 47 tests, and
  still be unreachable because `--help` lists only the old values for the flag that would select it,
  and the manifest declares no verb. Wire the flag and the `Provide.Verb` entry in the same change
  that adds the capability, and verify from `--help` plus one real invocation: a suite proves the
  function, never that anything can call it.
- **A corrected path beside a legacy default leaves the defect alive.** A crate can hold the
  repaired rendering next to the old one with the old one as the default, so every caller that omits
  the flag still gets exactly the output the fix was written to remove. Make the corrected path the
  default, or remove the legacy one - and when both must exist, say in the flag's own help which is
  which.
- **`pub mod Decision;` plus `pub use ...::Decision::Decision;` in one scope is a name collision**
  (`E0255`). Keep `Struct.rs` to `pub mod` lines and put the re-exports in `Library.rs`.
- **An untracked file beside the crate does not survive the next commit.** Dropping a helper into
  the tree looks like it landed - the path exists, the tool runs - until somebody else's commit
  lands and the file is gone, with no error and no diff to explain it.
- **No identifier and no target detail ships in the crate.** An account id, a token or a personal
  path is read from the environment, and its absence gets its own failure variant with a message
  (`Fail::NoAccount` -> `"no account id"`), never a literal default and never a panic. A manifest,
  port or observation type that names an address, a selector or a platform is an adapter wearing the
  schema's clothes.

## Reference

`Reference/Index.md` - the file list for this skill.

## Claims and how to test them

| Claim                             | Probe                                                     | Pass condition                         |
| --------------------------------- | --------------------------------------------------------- | -------------------------------------- |
| The layout is the house layout    | `ls Source/{Library,Fn,Struct}.rs module.json`            | all present, no `src/` tree            |
| The crate is a workspace member   | `grep -n '<Group>/<Leaf>' Cargo.toml`                     | one hand-written `members` line        |
| It builds under its library name  | `cargo build -p agito-<kebab> --release`                  | `Finished`, `Target/release/lib<Leaf>` |
| The manifest is well formed       | `cargo run -q -p agito -- verify --module @agito/<kebab>` | `Ready: true`                          |
| The error list is the crate's own | read `Provide.Error` beside `Code::All()`                 | same codes, same count                 |
| It is not reported as foreign     | the same `verify` report                                  | `code` is a list, not `null`           |
| The arrows hold                   | `cargo tree -p agito-<kebab> \| grep -c tokio`            | `0` for a capability                   |
| The capability is reachable       | `agito <verb> --help`, then one invocation                | the new flag is listed and works       |
| The corrected path is the default | run the verb with no behaviour flag                       | the corrected shape comes out          |

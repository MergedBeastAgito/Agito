---
name: workspace
description: Use when orienting in this workspace. The crates, the manifests, and the four gates.
version: 1.0.0
license: MIT
platforms: [macos, linux]
metadata:
    hermes:
        tags: [workspace, cargo, manifests, crates, gates]
        related_skills: [skill-authoring, crate-extension]
---

# Workspace

A change made against a guessed layout is a change made twice: a crate added at the wrong depth, a
gate run against the wrong package id, or an adapter compiled against a facade that does not export
the port it implements. The layout, the manifest keys and the four gates are fixed, and knowing them
is the difference between a build that answers and an afternoon of guessing.

## Three repositories, nested

A parent repository holds two submodules:

- `Agito` - the protocol and the Cargo workspace. This is the tree every other statement here is
  about.
- `Use` - the consumer side, holding `NFSU2` and the `Stem/` reference checkouts.

`Use/NFSU2` is its own repository (`MergedBeastAgito/NFSU2-Use`), not a folder of the parent, and
`Use/Stem/` holds the upstream recompiler checkouts. The parent tracks neither one's contents: each
carries its own history, so a path inside a submodule belongs to that repository and a link to it
names that repository rather than a path under the parent.

## The workspace

`Crate/<Group>/<Leaf>`, where the leaf is the library name - Rust cannot nest a crate root, so the
group folder carries the object and the leaf carries the name. Eleven crates, one binary, one
facade:

| Directory                   | Package id                  | Library        | Kind        |
| --------------------------- | --------------------------- | -------------- | ----------- |
| `Crate/RunTime`             | `agito-runtime`             | `RunTime`      | RunTime     |
| `Crate/Adapter`             | `agito-adapter`             | `Adapter`      | Capability  |
| `Crate/Lane`                | `agito-lane`                | `Lane`         | Capability  |
| `Crate/Spacetime/Catalog`   | `agito-spacetime-catalog`   | `Catalog`      | Capability  |
| `Crate/Skill/Contract`      | `agito-skill-contract`      | `Contract`     | Capability  |
| `Crate/Chat/Driver`         | `agito-chat-driver`         | `Driver`       | Capability  |
| `Crate/Browser/CDP`         | `agito-browser-cdp`         | `CDP`          | Adapter     |
| `Crate/Design/Verification` | `agito-design-verification` | `Verification` | Capability  |
| `Crate/Workflow`            | `agito-workflow`            | `Workflow`     | Workflow    |
| `Crate/Agito`               | `agito`                     | `Agito`        | Composition |
| `Crate/SDK`                 | `agito-sdk`                 | `SDK`          | Facade      |

`Crate/Agito` builds the binary `agito`. Inside a crate: `Source/Library.rs` as the library root,
`Source/Fn/` for operations, `Source/Struct/` for data and contracts, `Source/Script/` for the
TypeScript a page receives.

Two boundary rules hold across the set: a capability never imports an adapter, and the facade is
named only from outside the tree. A capability that imports an adapter is untestable without the
provider, which is the property the split exists to keep.

## The manifest

Every crate carries a `module.json` beside its `Cargo.toml`. Keys are PascalCase and name an action
in the singular; values are the closed vocabulary the binary reads:

| Key        | Carries                                                                 |
| ---------- | ----------------------------------------------------------------------- |
| `Name`     | the module id, scoped and lowercase - `@agito/browser-cdp`              |
| `Kind`     | `Capability`, `Adapter`, `Workflow`, `Composition`, `Facade`, `RunTime` |
| `Version`  | the crate version, tracking `[workspace.package] version`               |
| `Provide`  | `Port` and the `Error` codes the module reports                         |
| `Require`  | the ports or endpoints it needs - `agito.browse`, `env:CDP_URL`         |
| `Artefact` | `Entry`, `Hash`, `RunTime`                                              |
| `State`    | `Stateless`, `HoldTransport`, `HoldWorkspace` - what may be reused      |
| `Cost`     | `ColdMs`, `WarmMs`, `Network`, and the `Process` it spawns              |
| `Verify`   | the command that proves the module still matches its manifest           |

The module id and the Cargo package are two names for one crate and they are not always equal: the
composition root's package is `agito` and its module id is `@agito/composition`. `agito manifests`
prints the table from the binary, and the facade is the one module absent from it, because a facade
that provides no port has no row.

## The four gates

```sh
cargo build --release -p agito
agito manifests
agito verify --module <id>
agito skills Documentation/Skill
```

The first proves the workspace resolves and the binary links; the second prints the module table and
every third-party module the adapter roots declare; the third reports one module's readiness and
whether its manifest still matches its code; the fourth validates the skill corpus. Run them from
`Agito/`, with the binary at `Target/debug/agito` or `Target/release/agito`.

There is no test suite and no CI check gate. Correctness is the build,
`cargo clippy --workspace --all-targets -- -D warnings`, `cargo fmt --all -- --check`, and a manual
pass.

## The consumer

`Use/NFSU2/App/driver` is the third-party adapter and the shape a consumer takes: a library named
`Agito<Name>`, a thin binary beside it, one page expression per file under `Source/Script/`. It
builds against `agito-sdk` and `agito-lane` by relative path and nothing else from this tree, and
its `module.json` declares `@agito/nfsu-driver` as an `Adapter` in state `HoldTransport`, requiring
the ports `agito.browse` and `agito.lane.decide`.

Two dependencies and not three is deliberate. The facade carries the ports a browsing flow needs; it
does not carry `agito-lane`, because `agito.lane.decide` is what a composition root asks for rather
than what every consumer calls. A consumer that drives a decision model therefore names that crate
from a path beside the facade's, and the path is the one assumption the crate makes about where it
sits.

## Naming

`Documentation/Style.md` is the whole rule, and it has a boundary rather than exceptions: PascalCase
for folders, files, modules, types, functions and JSON keys; acronyms fully cased (`SDK`, `URL`,
`CDP`, `ID`, `JSON`, `WASM`); Cargo package ids lowercase kebab because Cargo requires it; dotted
port ids lowercase because they are wire identifiers. A name that would carry two concepts is two
names, and therefore two directories.

## Claims and how to test them

| Claim                           | Test                                                                         |
| ------------------------------- | ---------------------------------------------------------------------------- |
| The binary builds from the root | `cargo build --release -p agito` exits `0` and `Target/release/agito` exists |
| The module table is readable    | `agito manifests` prints one row per module with its port and state          |
| One module is verified          | `agito verify --module @agito/browser-cdp` exits `0` and names that module   |
| Every crate declares a manifest | `find Crate -maxdepth 3 -name module.json \| wc -l` answers `11`             |
| The corpus is valid             | `agito skills Documentation/Skill` prints no findings                        |
| A capability imports no adapter | `cargo tree -p agito-chat-driver \| grep -c browser` answers `0`             |

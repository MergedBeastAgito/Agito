# AGENTS.md

Orientation for an agent working in this repository. The rules live here; the procedures live in
`Documentation/Skill/`, one directory per skill, each with a `SKILL.md`. **Read the skill before the
task, not after it fails.**

## The skills, and when to load them

| Task                                                     | Load                                                 |
| -------------------------------------------------------- | ---------------------------------------------------- |
| Read one value out of a live page                        | `Documentation/Skill/Browser-Eval/SKILL.md`          |
| Reach a running browser, or bind the tab a person has    | `Documentation/Skill/Browser-Connect/SKILL.md`       |
| Ask a chat product a research question                   | `Documentation/Skill/Chat-Research/SKILL.md`         |
| The chat page moved its composer or model control        | `Documentation/Skill/Chat-Selectors/SKILL.md`        |
| Weigh a research answer rather than trust it             | `Documentation/Skill/Research-Conduct/SKILL.md`      |
| Several page operations in order, one budget each        | `Documentation/Skill/Bounded-Sequences/SKILL.md`     |
| One call that spans a loop, and a deadline that expired  | `Documentation/Skill/Bounded-Calls/SKILL.md`         |
| Batch-run destructive actions and verify each one        | `Documentation/Skill/Batch-Mutation-Verify/SKILL.md` |
| Drive a canvas game or app                               | `Documentation/Skill/Canvas-Driving/SKILL.md`        |
| Scrape a paginated list                                  | `Documentation/Skill/Paginated-Scrape/SKILL.md`      |
| Generate a gated multi-step UI action                    | `Documentation/Skill/UI-Step-Sequence/SKILL.md`      |
| A page expression in TypeScript, compiled at build time  | `Documentation/Skill/Script-Compilation/SKILL.md`    |
| Add a crate to this workspace                            | `Documentation/Skill/Crate-Extension/SKILL.md`       |
| Rust code that carries another language's source         | `Documentation/Skill/Embedded-Source/SKILL.md`       |
| Prove a change in a tree with no test command            | `Documentation/Skill/Verification-Harness/SKILL.md`  |
| Prove cargo, TOML or regex behaviour instead of guessing | `Documentation/Skill/Cargo-Probe/SKILL.md`           |
| A document links a submodule                             | `Documentation/Skill/Submodule-Links/SKILL.md`       |
| Author or render a diagram                               | `Documentation/Skill/Diagrams/SKILL.md`              |
| Write, move or check a skill in this corpus              | `Documentation/Skill/Skill-Authoring/SKILL.md`       |
| What this workspace is, and the gates it runs            | `Documentation/Skill/Workspace/SKILL.md`             |
| Grade this driver against a harness                      | `Documentation/Skill/Bench-Harness/SKILL.md`         |

Each directory carries a `Reference/Index.md` when it has archived files; a skill's body holds the
rules and points at the worked instance rather than inlining it.

These are boilerplate: they describe _how this kind of repository works_, never a particular
account, project or machine. Anything specific belongs in `.env` or a gitignored local file.

## Repository shape

```
Crate/                eleven crates; see Crate/README.md for the dependency lists
  <Group>/<Leaf>/     Cargo.toml, module.json, Source/{Library.rs,Fn,Struct,Script}
Build/Script.rs       the one statement of how a page expression is compiled
Documentation/        Design.md, Scope.md, Style.md, Diagram/, Skill/
Target/               build output, gitignored
```

A crate's leaf is its library name: `Crate/Browser/CDP` is library `CDP`. A crate whose name would
carry two concepts is two directories. Every crate carries a `module.json` beside its `Cargo.toml`.

## Standing rules

1. **Do not commit unless asked.** Leave changes in the working tree; the owner sweeps them.
2. **There is no verification layer.** No test suite, no CI check gate, no contract script:
   correctness is the build, clippy, formatting, and a manual pass. A green check nobody can read is
   worse than no check.
3. **Nothing machine-specific in tracked files.** No account names, absolute paths or tokens. A path
   is resolved relative to the crate, then from an environment variable, then from one installed
   location.
4. **PascalCase everywhere; acronyms fully uppercase** - `CLI`, `URL`, `API`, `JSON`, `WASM`, `CDP`,
   `SDK`. Folders, files, identifiers, prose. `Documentation/Style.md` is the whole rule.
5. **A capability never imports an adapter**, and the facade is named only from outside the tree. A
   capability that imports an adapter is untestable without the provider, which is the property this
   workspace exists to keep.
6. **A link to a submodule names its own repository**, never a path inside the parent: GitHub has no
   folder there to open.
7. **The repository is public.** No internal process artifacts, no scraped page bodies, no
   third-party documents pasted into tracked files.
8. **Write documentation, not transcripts.** Ideas arrive in conversation; they are restated as
   declarative prose. No quotation marks around a borrowed phrase, no first-person voice, no
   attributed fragments - a document states what is true, not who said it.

## Verifying a change

| Change                | Check                                                        |
| --------------------- | ------------------------------------------------------------ |
| Rust under `Crate/`   | `cargo build --workspace --release`                          |
| Lints                 | `cargo clippy --workspace --all-targets -- -D warnings`      |
| Formatting            | `cargo fmt --all -- --check`                                 |
| The module table      | `cargo run -q -p agito -- manifests`                         |
| One module's manifest | `cargo run -q -p agito -- verify --module <id>`              |
| The skill corpus      | `cargo run -q -p agito -- skills Documentation/Skill`        |
| Documentation only    | `node node_modules/prettier/bin/prettier.cjs --check <file>` |
| A page expression     | it is compiled by the crate's `build.rs`; a build proves it  |

## Local setup, once per clone

```sh
cargo build --workspace --release
cargo run -q -p agito -- manifests      # the module table, and the proof the binary works
```

No `node_modules` is needed for the build: Node is present for `prettier` alone, and a build works
with none installed.

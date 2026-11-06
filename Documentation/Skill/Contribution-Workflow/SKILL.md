---
name: contribution-workflow
description: Use when a change arrives, or a design decision must be recorded.
version: 1.0.0
license: MIT
platforms: [macos, linux]
metadata:
    hermes:
        tags: [workflow, issue, card, documentation, build, decision]
        related_skills: [skill-authoring, workspace]
---

# Contribution workflow

A change that arrives without a record leaves the next person nothing to read: no statement of the
defect, no command that shows it, no sentence saying what was decided. This skill is how work enters
this repository and how a design decision is written down.

## How work arrives

An issue first, then a card.

- **An issue** states the defect or the decision in the words of whoever found it, with the command
  or the reading that shows it. A defect without a reproduction is a rumour.
- **A card** is the unit of work: one owner, one deliverable, and a body that says how the result
  will be checked.

Work that starts without an issue is fine. Work that ends without one is the case this rule exists
to prevent.

## How a decision is recorded

This repository has no RFC corpus. A decision that changes the design is a paragraph in
`Documentation/`, and the paragraph names the claim it rests on - a measurement, a command, or an
explicit statement that the decision is the tree's own.

A document that changes its mind says so rather than rewriting the earlier position.
`Documentation/Observability.md` and `Documentation/Review.md` are both corrections of an earlier
pass, and both record the pass they correct.

## What a change is held to

None of these needs the network:

```sh
cargo build --workspace --release
cargo clippy --workspace --all-targets -- -D warnings
cargo fmt --all -- --check
cargo run -q -p agito -- manifests
```

There is no separate test step. A change is held to compiling, to clippy silence, to formatting that
matches `rustfmt.toml`, and to the module table still printing. No workflow step asserts anything a
build did not produce.

A new crate needs a `module.json` before it needs anything else, because that table is what
`agito manifests` and `agito verify` read. `Documentation/Skill/Crate-Extension/SKILL.md` is the
procedure.

## Spelling

`Documentation/Style.md` is the whole naming rule: PascalCase folders, files, identifiers and prose;
acronyms fully uppercase; one concept per file; a serialized key PascalCase and singular. A rename
is mechanical - `git mv` the path, then follow every import, the `module.json` that names the
package, and the paths quoted in `Documentation/` and `README.md`.

A link is a name too: a link to a submodule names that submodule's repository, never a path inside
the parent. `Documentation/Skill/Submodule-Links/SKILL.md` is the procedure.

## Claims and how to test them

| Claim                                 | Test                                                    |
| ------------------------------------- | ------------------------------------------------------- |
| The tree compiles                     | `cargo build --workspace --release`                     |
| No lint is outstanding                | `cargo clippy --workspace --all-targets -- -D warnings` |
| Formatting matches the pinned rustfmt | `cargo fmt --all -- --check`                            |
| Every crate still declares a module   | `cargo run -q -p agito -- manifests`                    |
| A crate's manifest matches its code   | `cargo run -q -p agito -- verify --module <id>`         |

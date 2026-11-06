# Contributing

Welcome. Before you start, read the [Code of Conduct](CODE_OF_CONDUCT.md) - the pledge, the
standards, the enforcement ladder and the address to report to all live there, and there is one copy
of them on purpose. By participating in this community you agree to abide by it. This file says the
rest: how work arrives, what a change has to pass, and how things are spelled.

---

## How work arrives

An issue first, then a card. An issue states the defect or the decision in the words of whoever
found it, with the command or the reading that shows it; a card is the unit of work - one owner, one
deliverable, and a body that says how the result will be checked. Work that starts without an issue
is fine. Work that ends without one leaves the next person nothing to read.

This repository has no RFC corpus. A decision that changes the design is a paragraph in
`Documentation/`, with the claim it rests on named as a source or marked as a decision. The existing
documents show the shape, and a document that changes its mind says so rather than rewriting the
earlier position: `Documentation/Observability.md` and `Documentation/Review.md` are both
corrections of an earlier pass, and both record the pass they correct.

---

## The build

A change is ready when these pass, and none of them needs the network:

```
cargo build --workspace --release
cargo clippy --workspace --all-targets -- -D warnings
cargo fmt --all -- --check
cargo run -q -p agito -- manifests
```

There is no test suite. The per-crate suites, the parity corpus under `Scenario/`, the
out-of-workspace fixture crate and the path audit were deleted with the rest of the verification
layer, and no workflow step asserts anything a build did not produce. What a change is held to is
that it compiles, that clippy is silent, that the formatting matches `rustfmt.toml`, and that the
module table still prints - a new crate needs a `module.json` before it needs anything else, because
that table is what `agito manifests` and `agito verify` read.

---

## Naming

One convention, everywhere in the tree: folders, files, identifiers and prose.

| Kind       | Rule                                                             | Example                                                |
| ---------- | ---------------------------------------------------------------- | ------------------------------------------------------ |
| Folder     | PascalCase when it names a role; lowercase where a tool fixes it | `Crate/Browser/CDP/Source`; `module.json`, `Cargo.toml` |
| File       | PascalCase, one concept per file                                 | `Port.rs`, `Library.rs`, `Scope.md`                    |
| Acronym    | fully uppercase, in names and prose                              | `CLI`, `URL`, `API`, `JSON`, `CDP`                     |
| Identifier | PascalCase for types and values; camelCase for locals            | `CDPClient`, `cdpValue`                                |

A compound takes the acronym whole: `CdpClient` is `CDPClient`, and a leaf keeps it - `CDP`,
`CDPSession`.

`Documentation/Style.md` is the whole rule, including the parts this table is too short to carry: a
verb names the leaf of a `Fn/` path, a folder names one role in the singular, a serialized key is
PascalCase and singular (`Provide`, not `Provides`), and a crate directory whose name would carry two
concepts is two directories - `Crate/Chat/Driver`, whose library is `Driver`.

Two kinds of name sit outside the rule because something else already fixes their spelling: the
ecosystem's root files (`module.json`, `Cargo.toml`, `Cargo.lock`) and wire-format field names such
as `webSocketDebuggerUrl`. A third is narrower than it looks: a script under `Source/Script/` is
TypeScript that the crate's `build.rs` compiles, and its path is the argument the `script!` macro
takes - `script!("Expression/ReadAnswers")` - so the file and the macro call carry the same
PascalCase name.

The tree has one shape: `Crate/<Name>/{Cargo.toml, module.json, Source/}`, or
`Crate/<Group>/<Name>/…` where the crate's name would otherwise carry two concepts. It is strict
because a module path is a file path: `Crate/<Name>/Source/Library.rs` is the library root, `Fn/`
holds operations, `Struct/` holds data and contracts, and every module name matches its file.

A rename is mechanical: `git mv` the path, then follow every import, the `module.json` that names
the package, and the paths quoted in `Documentation/` and `README.md`. A half-renamed tree does not
pass the gate, which is the check.

---

## Attribution

This document follows the structure of the one in the sibling TimeLine repository, and both adapt
the [`Contributor Covenant`][homepage], version 2.1, available at
[`https://www.contributor-covenant.org/version/2/1/code_of_conduct.html`][v2.1]. Community Impact
Guidelines were inspired by [Mozilla's code of conduct enforcement ladder][Mozilla CoC].

For answers to common questions about this code of conduct, see the FAQ at
[`https://www.contributor-covenant.org/faq`][FAQ]. Translations are available at
[`https://www.contributor-covenant.org/translations`][translations].

[homepage]: https://www.contributor-covenant.org
[v2.1]: https://www.contributor-covenant.org/version/2/1/code_of_conduct.html
[Mozilla CoC]: https://github.com/mozilla/diversity
[FAQ]: https://www.contributor-covenant.org/faq
[translations]: https://www.contributor-covenant.org/translations

Thank you for being part of our community and helping us create a safe and respectful environment
for everyone!

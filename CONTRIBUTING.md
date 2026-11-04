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

## The gate

A change is ready when these pass, and none of them needs the network:

```
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
cargo fmt --all -- --check
```

`cargo test --workspace` is the one that matters most: it refuses an import that reverses an arrow
(`Crate/Agentic/Test/Boundaries.rs`), it holds every diagram to the shared legend
(`Crate/Agentic/Test/Legend.rs`), and it answers every scenario in `scenarios/` through the fakes. A
new crate needs a `module.json` before it needs a test, because the boundary check reads the
manifest rather than the code.

The suites are fake-based by design: a page is a scripted responder, a browser is a scripted
launcher, and a deadline passes because the test advanced a clock it owns. A test that needs a real
browser belongs in an ad-hoc run under `.hermes/tools/`, not in the gate.

---

## Naming

One convention, everywhere in the tree: folders, files, identifiers and prose.

| Kind       | Rule                                                             | Example                                 |
| ---------- | ---------------------------------------------------------------- | --------------------------------------- |
| Folder     | PascalCase when it names a role; lowercase where a tool fixes it | `Crate/BrowserCDP/Source`, `scenarios/` |
| File       | PascalCase, one concept per file                                 | `Port.rs`, `Library.rs`, `Scope.md`     |
| Acronym    | fully uppercase, in names and prose                              | `CLI`, `URL`, `API`, `JSON`, `CDP`      |
| Identifier | PascalCase for types and values; camelCase for locals            | `CDPClient`, `cdpValue`                 |

A compound takes the acronym whole: `CdpClient` is `CDPClient`, and a file or crate named after one
keeps it - `BrowserCDP`, `CDPSession`.

Two kinds of name sit outside the rule because something else already fixes their spelling: the
ecosystem's root files (`module.json`, `Cargo.toml`, `Cargo.lock`) and wire-format field names such
as `webSocketDebuggerUrl`. A third is narrower than it looks: a script under `Source/Script/` is
TypeScript that the crate's `build.rs` compiles, and its path is the argument the `script!` macro
takes - `script!("Expressions/ReadAnswers")` - so the file and the macro call carry the same
PascalCase name.

The tree has one shape: `Crate/<Name>/{Cargo.toml, module.json, Source/, Test/}`. `Source/` holds
the code and `Test/` the suites. It is strict because a module path is a file path:
`Crate/<Name>/Source/Library.rs` is the library root, `Fn/` holds operations, `Struct/` holds data
and contracts, and every module name matches its file.

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

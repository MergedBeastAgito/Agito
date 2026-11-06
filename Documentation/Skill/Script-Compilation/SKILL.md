---
name: script-compilation
description: Use when page scripts are TypeScript compiled at build time.
version: 1.0.0
license: MIT
platforms: [macos, linux]
metadata:
    hermes:
        tags: [rust, typescript, build-script, codegen, escaping, oxc]
---

# Script Compilation

The expressions this workspace sends to a page are TypeScript files under
`Crate/<Name>/Source/Script/`. A crate's `build.rs` turns each one into the JavaScript the page
receives, and `script!` bakes the result into the binary. The file the compiler parses is the file
the page runs; the only difference between them is that the page's copy has no type syntax and no
declarations.

This is not a transpiler, not a type checker and not a bundler. It is a parser used for its spans:
the TypeScript-only bytes are found in the AST and deleted, and every other byte survives because it
is never re-emitted.

---

## The shape

- **One file per expression.** `Crate/Chat/Driver/Source/Script/Expression/ReadAnswers.ts` and the
  ten beside it; `Crate/Workflow/Source/Script/Measure.ts`. The path under `Source/Script/` is the
  path the macro takes, minus the extension: `script!("Expression/ReadAnswers")`.
- **A second file only when the first is pinned.** `Expression/ReadBoth.ts` reads the thread with
  the same code as `ReadAnswers.ts` and returns the page's Markdown beside the text, because the two
  single-format expressions are the bytes a page already receives and an expression that gained a
  field would not be that expression. The two files are written so the thread read is the same text
  in both, from its first line to the object each one returns: a change to one that is not a change
  to the other is a change to what a page receives.
- **One rule, included twice.** `Crate/Chat/Driver/build.rs` and `Crate/Workflow/build.rs` are each
  three lines around `include!("../../../Build/Script.rs")`, and each calls
  `CompileScripts("Source/Script", "Script")`. The rule for what a script may contain lives in
  `Build/Script.rs` and cannot drift between the two crates.
- **Two artefacts per script.** `OUT_DIR/Script/Expression/ReadAnswers.js` is the page's copy;
  `.params` is the parameter contract, one `Name:kind` line each. A `.js` file whose bytes are
  unchanged is not rewritten, so a no-op edit leaves its mtime alone.
- **The macro** is `RunTime::script!`, which expands to `include_str!` of the compiled file in the
  **caller's** `OUT_DIR` - `env!("OUT_DIR")` inside a `macro_rules!` expands where the macro is
  called, which is how each crate reads the scripts its own build compiled.

A file with no type syntax compiles to itself. That is the property to check first when a byte
comparison fails: if the output differs from the input, something is re-printing the file.

---

## The compiler: OXC's parser, pinned, used for spans only

`oxc_parser`, `oxc_ast`, `oxc_ast_visit`, `oxc_span` and `oxc_allocator`, all pinned `=0.157.0` in
the workspace `Cargo.toml` and named as build-dependencies by the two crates that compile scripts.
The pin is exact because the erasure walks this parser's AST: a minor release that renames a node
must be a deliberate upgrade, not a broken build. Nothing here is linked into the binary at run
time.

`Documentation/Script.md` records the measurements that chose it - OXC's parser alone against OXC's
own transform, SWC, `deno_ast`, esbuild and `tsc`, on one 1,898-byte expression - and every
alternative that generates code from an AST re-printed the file: a semicolon after a spliced marker,
a re-indent, a dropped comment block, `"use strict";` at byte 0. Those are changes in what a page
receives, which is what this build is not allowed to make.

---

## What is erased, and what is refused

Erased, by span, keeping every byte outside the span:

- `declare const __PARAM_Name__: T;` - read first, for the parameter contract, then removed with the
  whole line **and one following blank line**. A declaration block that keeps its trailing blank
  line leaves a leading newline in the artefact.
- every `x: T` annotation - the span includes the colon;
- every `x as T` - from the end of the expression to the end of the type.

Refused, by name, with the file and the line: `satisfies`, a non-null assertion, a `<T>x` assertion,
generic type parameters, a type instantiation, and the TypeScript-only statements this build has no
erasure for - type alias, interface, enum, namespace, global, external module, import-equals. A
refusal is a build error, never a guess: an erasure that takes a bracket with it produces an
expression that parses and does the wrong thing.

After cutting, the text is parsed **again as JavaScript** before it is written. That is the check
that catches a cut which ate a bracket, and it runs at build time rather than on a page.

---

## A parameter is a declared identifier, and the kind is the type

`{{name}}` cannot be compiled - it is not parseable TypeScript - so a placeholder is an identifier
marker, declared where a reader and the build both see it:

```ts
declare const __PARAM_Scroll__: string; // data: escaped on substitution
declare const __PARAM_Pattern__: RegExp | null; // code: spliced as it stands
```

- **`string` is data.** The value becomes an escaped JavaScript string literal, so a selector
  holding a quote, a backslash or a newline cannot end the string it sits in.
- **Any other declared type is code.** A function, a `RegExp`, an array literal - spliced as it
  stands. Do not guess the kind from the value: the declaration is the only statement of it.
- **A declaration with no type is refused**, because it is neither data nor code.
- **A marker with no declaration, a declaration with no marker, and a declaration written twice are
  each a build error** naming the position.
- **A marker is an identifier, so it can only sit where an identifier is valid.** It cannot be
  spliced into a property key position - `{ text: Seen(Last), __PARAM_X__ marks: ... }` is a parse
  error before any substitution runs, and a fragment that would be valid after substitution is not
  the file the compiler sees. A value position, an argument, or a whole expression is where a marker
  belongs; a field that has to be _added_ to an object the script already returns is a second file,
  not a parameter.
- **Substitution is one pass, left to right.** A value is written into the output and never scanned
  again, so a question whose text contains `__PARAM_Seen__` cannot have another parameter's code
  spliced into the middle of its string.
- **An unfilled marker stays.** `Unfilled` reports it, and a page receiving one throws
  `ReferenceError: __PARAM_Scroll__ is not defined` rather than matching nothing.

`Escape` is held to `serde_json` byte for byte over every control character and every non-ASCII
character, with one deliberate exception: `U+2028` and `U+2029` stay raw, as JSON leaves them and as
a JavaScript literal has accepted them since ES2019. Escaping them would change the bytes of an
expression a page already receives for a hazard no current engine has.

---

## A call site cannot forget a parameter

`script!("Expression/ReadSources"; Answer = &Selectors.Answer, Sources = &Selectors.Sources)` is
compared against the script's own declarations in a `const` context, so a missing argument, an extra
one or the same one twice is a `cargo build` error at the call site rather than a `ReferenceError`
in a page:

```
error[E0080]: evaluation panicked: script parameters: a declared parameter is missing
  --> Crate/Chat/Driver/Source/Fn/Expression.rs:<line>:2
```

`panic!` in a `const fn` cannot format, so the message names the problem and the compiler's error
names the call site. The check compares the count as well as the names, or a repeated name would
pass.

---

## The bytes are held by construction, and the fixture is gone

The compiled artefact must equal the bytes the page was already receiving. That used to be held by
two byte-identity fixtures - a map of expression name to bytes for the chat driver, dumped from the
pre-compilation `Expressions.rs`, and the same for the workflow's one measurement expression - and
both are deleted with the rest of the verification layer.

What holds the rule now is the walker itself. The erasure removes exactly the bytes of the
type-syntax spans and emits every other byte, so a compiler that re-indented, re-quoted, appended a
semicolon or dropped a comment cannot be written without changing that walk. The rule is structural
rather than asserted: nothing compares the output to a recorded baseline any more, so a change to
the walker is a change to what a page receives and nothing will say so.

The fixture also covers the _set_ of expressions: the test asserts that the fixture and the driver
build the same number of names, so an expression added without a baseline fails rather than going
unheld.

---

## Determinism is part of the contract

The output is a pure function of the source: no timestamps, no absolute paths, no iteration over a
`HashMap`. `Sources` walks the tree and sorts, so file order is stable; `Write` compares bytes and
returns without touching the file when they match. Touching every script and rebuilding therefore
leaves every `.js` and `.params` with an identical hash **and an identical mtime** - which is what
stops a no-op edit from rebuilding everything downstream.

---

## What the build does not check

Types. OXC parses; it does not infer. A `declare const __PARAM_Scroll__: string;` is a declaration a
type checker would hold a call site to, and nothing in this build is that checker. What the build
buys is that the expression is parsed - a syntax error, an unsupported construct, a
marker/declaration disagreement or an erasure that left invalid JavaScript fails the build - not
that its types are consistent.

`tsc --noEmit` over the scripts as one program is not a gate either, and `Documentation/Script.md`
records why: the same marker declared in several files is a global redeclaration because each script
is its own page evaluation rather than a module, the bodies are unannotated, and `querySelector`
returns `Element` where the expression uses `HTMLElement` members. Making that pass green would
change what the scripts are, not how they are compiled.

---

## Reference

`Reference/Index.md` - the file list for this skill.

`Documentation/Script.md` - the compiler comparison with its measurements, the parameter contract,
and the determinism check.

---

## Claim → test

| Claim                                          | How it was checked, or what is left                                                                                                                                                                        |
| ---------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| The compiled artefact is the old bytes         | the chat driver's suite (deleted); nothing compares the output to a baseline now                                                                                                                           |
| The workflow's expression is held too          | the workflow's suite (deleted)                                                                                                                                                                             |
| Every marker is filled, and the check bites    | the chat driver's suite (deleted); `AssertParameters` still refuses a call site that omits one                                                                                                             |
| A malformed script names its position          | write `Crate/Chat/Driver/Source/Script/Expression/Broken.ts` holding an unclosed call, run `cargo build -p agito-chat-driver`, require `Source/Script/Expression/Broken.ts:<line>:<col>: Unexpected token` |
| A call site cannot forget a parameter          | delete one argument from a `script!` call, run `cargo build -p agito-chat-driver`, require `E0080` naming that call site                                                                                   |
| A rebuild that changes nothing touches nothing | `touch` the `.ts` files, rebuild, and compare the mtime list under `Target/debug/build/agito-chat-driver/*/out/Script/` before and after                                                                   |
| Escaping matches the old encoder               | the runtime's suite (deleted); `Escape` is what a page receives, unasserted                                                                                                                                |
| The build needs no node and no network         | `cargo build --workspace --offline` succeeds                                                                                                                                                               |

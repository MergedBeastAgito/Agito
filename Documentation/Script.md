# Script Compilation

The expressions the driver and the workflow send to a page are TypeScript files under
`Source/Script/`. A crate's `build.rs` compiles them into the JavaScript a page receives, and
`script!` bakes the result into the binary. This document records which compiler was chosen, what it
measured against the alternatives, and why the compilation erases type syntax instead of
transpiling.

**Source:** measured in this repository, on this machine, with the commands in
[Commands](#commands). The compiler crates are `oxc_*` 0.157.0; the alternatives are the versions
their `cargo add` resolved to on the same day.

## The shape

- **One file per expression**, `Source/Script/Expressions/ReadAnswers.ts` and so on, and one for the
  workflow's `Source/Script/Measure.ts`. The path under `Source/Script/` is the path the macro
  takes: `script!("Expressions/ReadAnswers")`.
- **A second file only when the first is pinned.** `Expressions/ReadBoth.ts` carries the same thread
  read as `Expressions/ReadAnswers.ts` and adds the page's Markdown to what it returns, because the
  two single-format expressions are what a page already receives: an expression that gained a field
  would not be that expression. The duplication is written so the thread read is the same text in
  both files.
- **A `build.rs` per owning crate**, three lines around `Build/Script.rs`, which is the one
  statement of what a script may contain and how it becomes bytes. `ChatDriver` and `Workflow` both
  include it, so the rule cannot drift between them.
- **The macro**, `AgenticRuntime::script!`, expands to `include_str!` of the compiled file in the
  caller's `OUT_DIR` and, when parameters are given, to `FillDeclared` with the script's own
  declared parameter list.
- **The compiled artefact is JavaScript, not a template**: the parameter markers are valid
  identifiers, so the file the compiler parses is the file the page runs, and the only difference
  between the two is the markers.

---

## The compiler: OXC's parser, used for its spans

- **OXC, parser only** - `oxc_parser` with `oxc_ast`, `oxc_ast_visit`, `oxc_span`, `oxc_allocator`.
  199 packages, **18.9 s** to build cold (37.6 s of CPU), **120 µs** per file warm, 1,565 bytes out.
  Byte-identical to the JavaScript the page was already being sent. **This is the choice.**
- **OXC's own TypeScript transform** - adding `oxc_semantic`, `oxc_transformer`, `oxc_codegen`. 303
  packages, 38.4 s cold (69.3 s CPU), 212 µs per file, 1,519 bytes out. It re-prints the file: the
  first difference is at byte 9, where it appends a semicolon after the spliced marker
  (`__PARAM_Serialise__;`) and re-indents to one tab.
- **SWC** - `swc_ecma_parser`, `swc_ecma_transforms_typescript`, `swc_ecma_codegen`, `swc_common`,
  `swc_ecma_ast`. 340 packages, 47.7 s cold (96.3 s CPU), 298 µs per file, 1,805 bytes out. First
  difference at byte 3.
- **`deno_ast`** - the SWC wrapper. 460 packages, 74.9 s cold (122.6 s CPU), 639 µs per file, 1,673
  bytes out. First difference at byte 3. Its dependency tree is the largest of the four and the
  slowest, for the same output as SWC.
- **`esbuild` 0.28.2** - a 10.6 MB standalone binary, so not embeddable in a Rust build. 15.9 ms
  mean per invocation over 20 runs (12.3 ms median), 1,246 bytes out, first difference at byte 9: it
  drops the five-line comment block above the read, re-indents to two spaces, rewrites `undefined`
  to `void 0` and appends semicolons. No type check.
- **`tsc` 6.0.3** - a node shim over a 24 MB package, 1,248 ms per invocation, 1,713 bytes out,
  first difference at byte 0 (`"use strict";`). It does type check, and it emits despite errors.
- **Biome** - not a candidate: the crates.io publication is `biome_js_parser` 0.5.7, a 2022
  parser-only snapshot with no TypeScript transform. The transformer is not published as a library.
- **Bun's transpiler and `tsgo`** - not candidates either: neither is installed here, both are
  Zig/Go binaries rather than Rust crates, and both would put a download in the path of
  `cargo build`.

Every candidate above produced the same output on every run - five runs each, compared by hash - so
determinism did not discriminate between them. The discriminator was the bytes themselves.

The cold times are from an empty `target/` with `RUSTC_WRAPPER=` empty, so a warm `sccache` does not
flatter them; the warm times are 1,000 in-process iterations over the same 1,898-byte source. What a
developer pays day to day is neither: the build script over all ten `ChatDriver` scripts, the
13,367-byte serializer included, runs in **about 10 ms**.

---

## Why erasure, not a transpiler

Every candidate above that generates code from an AST rewrites the whole file. That is what a
transpiler is for, and it is the one thing this card could not allow: the compiled expression had to
be the bytes a page already received, because those bytes are what the parity scenarios and the
recorded baselines were taken against, and because "this diff is a change in what the page receives"
is a property worth keeping.

So the compilation is the smallest transformation that turns TypeScript into JavaScript: the file is
parsed by a real TypeScript parser, the spans of the TypeScript-only syntax are found in the AST,
and exactly those bytes are removed. Indentation, quotes, comments and semicolons are untouched
because they are never re-emitted. A file with no type syntax at all compiles to itself.

- **What is erased** - `declare const __PARAM_Name__: T;` statements (read first, for the parameter
  contract), every `x: T` annotation, and every `x as T` assertion.
- **What is refused** - everything else TypeScript-only that this build has no test for:
  `satisfies`, non-null assertions, `<T>x` assertions, generic type parameters, type aliases,
  interfaces, enums, namespaces and import-equals. A refusal is a build error naming the file and
  the line, not a silently mangled expression.
- **What is checked afterwards** - the erased text is parsed again as JavaScript, so an erasure that
  took a bracket with it cannot reach a page; and every `__PARAM_Name__` marker must have a
  declaration of the same name while every declaration must be used.

---

## The parameter contract

An expression is parameterised: selectors, the question's text, a regex source, a spliced
serializer. The declaration is written in the script, in TypeScript, where a reader and the build
both see it:

```ts
declare const __PARAM_Scroll__: string;
declare const __PARAM_Pattern__: RegExp | null;
```

- **The kind is the declared type.** A parameter declared `string` is data: it is substituted as an
  escaped JavaScript string literal, so a selector holding a quote, a backslash or a newline cannot
  end the string it sits in. Any other declared type is code - a function, a `RegExp`, an array
  literal - and is spliced as it stands.
- **The names are checked at compile time.** `script!("Expressions/ReadAnswers"; Scroll = ...)` is
  compared against the script's own declarations in a `const` context: a missing parameter, an extra
  one, or the same one twice is a `cargo build` error pointing at the call site, not a
  `ReferenceError` in a page.
- **Substitution is one pass, left to right.** A value is written into the output and never scanned
  again, so a question whose text happens to contain `__PARAM_Seen__` cannot have another
  parameter's code spliced into the middle of its string.
- **An unfilled marker stays.** A marker with no value is left in place and reported by
  `AgenticRuntime::Unfilled`, which the tests use; a page receiving one throws
  `ReferenceError: __PARAM_Scroll__ is not defined` rather than matching nothing.
- **Escaping is held to `serde_json`.** `Escape` produces exactly what the JSON encoder these
  expressions were built with produced, byte for byte, for every control character and every
  non-ASCII character - the one exception being `U+2028` and `U+2029`, which JSON leaves raw and
  which a JavaScript string literal has accepted raw since ES2019.

---

## Determinism

The output is a pure function of the source: no timestamps, no absolute paths, no iteration over a
`HashMap`. A file whose compiled bytes are unchanged is not rewritten, so a rebuild after a no-op
edit leaves the artefact's mtime alone and nothing downstream rebuilds. Verified by touching every
script, rebuilding, and comparing the hashes and the mtimes of every compiled artefact of the
driver: ten expressions, each with its JavaScript and its parameter list, with identical hashes and
identical mtimes.

---

## What the build does not check

- **Types.** OXC parses; it does not infer. `declare const __PARAM_Scroll__: string;` is a
  declaration a type checker would hold a call site to, and nothing in the build is that checker.
  What the build buys is that the expression is _parsed_ - a syntax error, an unsupported construct,
  or a marker/declaration disagreement fails the build - not that its types are consistent.
- **A `tsc` pass is not a gate, and is not currently meaningful.** Run over all the scripts as one
  program, `tsc --noEmit --target es2022 --lib es2022,dom` reports 61 diagnostics, in three groups
  that are all artefacts of how the files relate rather than defects in an expression: the same
  `__PARAM_Name__` declared in several files is a global redeclaration (each script is its own page
  evaluation, not a module), the bodies are unannotated JavaScript so `--noImplicitAny` fires, and
  `querySelector` returns `Element` where the expression uses `HTMLElement` members such as
  `innerText` and `click`. Making that pass green means annotating every body and compiling each
  script as its own program - a change to what the scripts are, not to how they are compiled. Until
  then the declarations are documentation plus the build's parameter list, and the syntax check is
  the guarantee.

---

## Commands

The gates, from the workspace root:

```sh
# Build with no node_modules present and no network, from a clean Target/.
rm -rf Target && cargo build --workspace --offline

# The compilation itself: every expression under Source/Script/ is erased and emitted.
cargo build -p agentic-chat-driver -p agentic-workflow

# A malformed script names its file and line and fails the build.
printf '(() => {\n\treturn 1;\n)()\n' > Crate/ChatDriver/Source/Script/Expressions/Broken.ts
cargo build -p agentic-chat-driver   # error: Source/Script/Expressions/Broken.ts:3:1: Unexpected token

# A call site that disagrees with a script's declarations fails to compile, at the call site.
#   error[E0080]: evaluation panicked: script parameters: a declared parameter is missing
#   --> Crate/ChatDriver/Source/Fn/Expressions.rs:148:2

# The build itself: every script under Source/Script/ is erased and compiled, and a
# malformed one stops the build with its position named.
cargo build --workspace --offline
```

The measurements above were taken with spike crates in the session scratch directory, each built
from an empty `target/`, with the package count read from `cargo tree | wc -l`:

```sh
RUSTC_WRAPPER= /usr/bin/time -p cargo build   # cold, sccache defeated
```

The byte-identity fixture that used to hold the _before_ side - the expressions dumped from the
pre-card `Expressions.rs`, so a reformatting compiler would fail rather than pass - is deleted with
the rest of the verification layer. What still enforces the same rule is structural: the erasure
removes exactly the bytes of the type-syntax spans and emits every other byte, so a compiler that
re-indented or re-quoted could not be written without changing the walker.

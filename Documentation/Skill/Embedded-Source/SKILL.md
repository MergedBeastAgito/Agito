---
name: embedded-source
description: Use when Rust carries another language's source: a script, a query, a rule.
version: 1.0.0
license: MIT
platforms: [macos, linux, windows]
metadata:
    hermes:
        tags: [rust, escaping, format-string, typescript, codegen, editing]
        related_skills: [script-compilation, crate-extension]
---

# Embedded Source

Rust that carries another language's source as a string - a TypeScript expression sent to a page, a
CSS rule, a SQL statement, a shell command - compiles while the embedded source does not, so the
compiler cannot tell you the string is wrong. The failure arrives at the far end and arrives
silently: an escape level too deep, a brace the format macro ate, a stale path baked into the
binary. Every rule below is about keeping the embedded source somewhere a reader, a linter and a
compiler can all see it.

---

## Source lives in a file, never in a `format!` string

This tree's answer is a directory of TypeScript, one file per expression, under
`Crate/<Name>/Source/Script/`, compiled at build time and baked in by the `script!` macro:

```rust
pub fn ReadMode(Selectors:&Selectors) -> String { script!("Expression/ReadMode"; Mode = &Selectors.Mode) }
```

`Crate/Chat/Driver/build.rs` is three lines around `include!("../../../Build/Script.rs")` and calls
`CompileScripts("Source/Script", "Script")`; `Crate/Workflow/build.rs` is the same three lines, so
the rule for what a script may contain cannot drift between the two. The macro expands to
`include_str!` of the compiled file in the **caller's** `OUT_DIR` - `env!("OUT_DIR")` inside a
`macro_rules!` expands where the macro is called - plus a check of the call site against the
script's own declarations.

`Documentation/Script.md` records why the compilation is erasure rather than a transpiler, with the
measurements; `Script-Compilation/` is the skill for the compiler itself. This skill is the editing
discipline around it, and the general rule for the cases where a file is not the answer.

**Extracting a string to a file is a move, not a rewrite.** The emitted bytes must not change,
because those bytes are what a page already receives and what every recorded observation was taken
against. The probe is structural: a file with no type syntax compiles to itself, so if a script with
no `declare` lines comes out different from its source, something is re-printing the file.

---

## A parameter is a declared identifier, and its type is its kind

`{{name}}` cannot be compiled - it is not parseable TypeScript - so a placeholder is an identifier
declared in the script, where a reader and the build both see it:

```ts
declare const __PARAM_Scroll__: string; // data: escaped on substitution
declare const __PARAM_Pattern__: RegExp | null; // code: spliced as it stands
```

- **`string` is data.** The value becomes an escaped JavaScript string literal, so a selector
  holding a quote, a backslash or a newline cannot end the string it sits in.
- **Any other declared type is code.** A function, a `RegExp`, an array literal - spliced as it
  stands. Do not infer the kind from the value: the declaration is the only statement of it.
- **A declaration with no type is refused**, because it is neither data nor code.
- **A marker without a declaration, a declaration with no marker, and a declaration written twice
  are each a build error** naming the file and the position. So is a marker that never terminates.
- **A marker is an identifier, so it can only sit where an identifier is valid.** It cannot be
  spliced into a property key position; a field that has to be _added_ to an object the script
  already returns is a second file, not a parameter.
- **Substitution is one pass, left to right.** A value is written into the output and never scanned
  again, so a question whose text contains `__PARAM_Seen__` cannot have another parameter's code
  spliced into the middle of its string.
- **An unfilled marker stays in the artefact.** A page receiving one throws
  `ReferenceError: __PARAM_Scroll__ is not defined` rather than quietly matching nothing.

A call site cannot forget a parameter: `script!` compares its arguments against the script's
declarations in a `const` context, so a missing argument, an extra one or the same one twice is a
`cargo build` error at the call site -
`error[E0080]: evaluation panicked: script parameters: a declared parameter is missing` - rather
than a `ReferenceError` in a page. The comparison counts as well as names, or a repeated name would
pass.

---

## When the source has to stay inline

A small fragment - one `querySelector`, one property read - can stay a Rust literal. Three rules, in
order of how much they save.

**Quote so that nothing needs escaping.** Pick the embedded language's quoting that the Rust literal
does not already use. A Rust `"..."` literal containing `'TEXTAREA'` needs no escape at all, while
the same code with `"TEXTAREA"` needs `\"` at every occurrence - and each of those is a place the
escape level can go wrong.

```rust
// No backslashes anywhere in the format string.
format!("(() => {{ const C = document.querySelector({}); if (C.tagName === 'TEXTAREA') return 'ok'; }})()", Literal(&Selector))
```

**Double every brace in a `format!` literal.** The embedded language's object literals and blocks
are format placeholders to Rust: `{ block: 'end' }` inside a `format!` string is a compile error -
``invalid format string: expected `}`, found `b` `` - and the message points at the embedded code,
which reads as a syntax error in the wrong language. Write `{{` and `}}` for every brace that
belongs to the embedded source and `{}` only where an argument goes. When a brace-bearing expression
is simpler without the object, prefer the brace-free call: `scrollIntoView(false)` is
`{ block: 'end' }` with nothing to escape.

**Escape user data through one helper.** `serde_json::to_string` for a JavaScript literal, and one
`Escape` function for everything else, so a selector or a question cannot terminate the expression.
In this tree `RunTime::Escape` is held to `serde_json` byte for byte over every control character
and every non-ASCII character, with one deliberate exception: `U+2028` and `U+2029` stay raw, as
JSON leaves them and as a JavaScript literal has accepted them since ES2019.

---

## Verify the bytes you wrote, not the source you re-read

An edit tool that round-trips escaping can land a different number of backslashes than you wrote,
and the file still looks plausible in a rendered diff. The mechanism is usually the tool call's own
JSON encoding: writing `\t` inside a JSON string argument delivers a literal TAB byte, so `'\t'` in
Rust source becomes a char literal holding a tab (`character constant must be escaped`) and `"\\t"`
becomes a backslash followed by a tab. Print the line as bytes before trusting it:

```sh
python3 -c 'print(repr(open("<file>").read().split("\n")[<line>-1]))'
# or: sed -n '<line>p' <file> | od -c | head
```

A target of `\"` (backslash, quote) is correct in a Rust literal; three backslashes before the quote
is a bug that compiles in some positions and produces invalid source in others.

- **Repair by exact run, not by re-issuing the patch.** Replace the specific bad sequence everywhere
  at once (`replace_all` over the over-escaped run) and re-verify the bytes. Re-sending the whole
  line invites the same round-trip that broke it.
- **Prefer a raw string in the Rust source** (`r"\t"`) so no escape level remains to go wrong.
- **Match on a fragment with no backslashes.** A keyword, ` });`, `{block:` matches reliably; a
  fragment containing the escapes you are trying to fix is exactly the one fuzzy matching is most
  likely to miss.
- **When the tool cannot tell two lines apart**, do not force it: write the replacement with a real
  script that asserts one occurrence and re-checks the bytes afterwards.

---

## The macro's path is a string, so a rename is invisible

`script!("Expression/ReadAnswers")` and `Source/Script/Expression/` are joined only inside
`include_str!`, so renaming a directory - or splitting a flat one into subdirectories - leaves every
call site on the old name and the compiler never sees the disagreement. The failure is:

```
couldn't read .../out/Script/Expression/ReadAnswers.js: No such file or directory
```

which reads as a broken build script rather than a stale literal. Read the stale segment out of the
error, compare it with `ls Source/Script`, fix **every** call site (a partial rename compiles only
when the last one matches), and fix the path examples in the doc comments in the same pass, because
that is where the next copy comes from.

---

## A formatter must not own the script sources

Once the compiled artefact is pinned to what a page receives, the script sources are pinned inputs:
a formatter run over them changes the thing under test. `.prettierignore` carries
`Crate/*/Source/Script/` for exactly this reason, under the rule the file states at the top - one
formatter per file type, and Rust is rustfmt's (`*.rs`), TOML is nobody's.

If a formatter has already rewritten a script, restore it by content rather than by
`git checkout --`: read `git show HEAD:<path>` and write each file back through a script that lists
its targets, supports a dry run, and ends by asserting `git status --short -- <paths>` is empty. Do
not re-record the expectation to match the formatted file - that turns the assertion into the
formatter compared with itself.

---

## A return value is not the effect

The embedded source's own `true` means the call ran, not that the host's state changed. An editor
that keeps a document model of its own - ProseMirror, Lexical, a `contenteditable` inside a
framework - can report a successful insert while its model still holds the previous value; a click
can report success while a control stayed disabled. The lie lands at the far end, and the next
action operates on the stale value.

So the host verifies by reading the state back through the same channel that will use it, and
refuses when it disagrees:

- after an insert, read the field and compare it with what was meant to be written; a mismatch is a
  failure with its own code, not a retry and not a warning;
- before an irreversible action, assert the precondition on the same read that triggers it, so the
  check and the act see one instant;
- replace an existing value through the embedded language's own edit path - select, delete, insert -
  rather than writing over its storage, because a write bypasses the model that decides what to
  submit.

An expression that returns whether it _found_ its target is a diagnostic; one that returns whether
it _changed_ anything is the one worth trusting.

---

## A value computed on both sides must agree on its units

When the host and the embedded source each compute the same identity - a hash of a turn, a length, a
key derived from text - they must agree on how it is counted, or every comparison fails for reasons
that look like logic bugs.

JavaScript's `Text.length` counts UTF-16 code units; Rust's `chars().count()` counts scalar values.
They agree on ASCII and diverge on the first emoji, so a mark built as `len + ':' + head` in
JavaScript and mirrored in Rust matches until an answer contains one, then never matches again.
Count code points on both sides (`[...Text].length`) or compare a value only one side computes.
Slicing has the same defect: `slice(0, 48)` counts code units while `chars().take(48)` counts scalar
values.

---

## Reference

`Reference/Index.md` - the file list for this skill.

`Documentation/Script.md` - the compiler comparison with its measurements.

---

## Claims and how to test them

| Claim                                                  | Probe                                                                                           | Pass condition                                |
| ------------------------------------------------------ | ----------------------------------------------------------------------------------------------- | --------------------------------------------- |
| No source is hidden in a format string                 | `grep -rn 'format!(r#"(()' Crate/*/Source Crate/*/*/Source`                                     | no hits; the expression is a `script!`        |
| The literal carries the intended bytes                 | `python3 -c 'print(repr(...))'` on the line                                                     | one backslash before a quote, not three       |
| Braces are placeholders only where intended            | the crate compiles                                                                              | no `invalid format string`                    |
| The script is the bytes the page runs                  | compile a script with no `declare` lines                                                        | artefact equals source, byte for byte         |
| Every marker is declared and every declaration is used | `cargo build -p <crate>`                                                                        | build error naming the file and position      |
| A call site cannot forget a parameter                  | delete one argument from a `script!` call                                                       | `E0080` naming that call site                 |
| A malformed script fails the build                     | add `Source/Script/Expression/Broken.ts` with an unclosed call                                  | `...Broken.ts:<line>:<col>: Unexpected token` |
| A rename reaches every call site                       | `grep -rn 'script!(' Crate/<Name>/Source` after `ls Source/Script`                              | no argument names a directory that is gone    |
| The emitted source parses in its own language          | run it - page evaluate, `node -e`, `sqlite3 :memory:`                                           | a value, not an exception                     |
| User data cannot break out                             | pass `'`, `"` and `;` through the helper                                                        | the expression still evaluates                |
| Both sides count the same way                          | build the identity on text with an emoji                                                        | host and page produce the same mark           |
| A no-op rebuild touches nothing                        | `touch` the `.ts` files, rebuild, compare mtimes under `Target/debug/build/<pkg>/*/out/Script/` | identical hashes and mtimes                   |

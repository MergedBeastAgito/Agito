# Restructure inventory

Every path that names `rust/`, `packages/` or `src/`, the manifest lines the moves force, and every
JavaScript expression currently living inside a Rust string literal. Phase 1 of the restructure
wrote this and changed nothing; the phases that follow record their own gates under
[After](#after---phase-2-crate-at-the-root).

The scan is of the tracked tree (`git ls-files`, `git grep`), read from the repository root and
excluding `Target/`, `node_modules/` and the generated bundles' payloads, which no layout rule
governs.

---

## Baseline, before any change

```
cd rust && cargo test --workspace     # 177 passed, exit 0
node --test "packages/*/Test/*.test.ts"   # 240 tests, 240 pass, 0 fail
node Scripts/Boundaries.mjs           # RESULT: every import points inward
node Scripts/Feed.mjs                 # RESULT: the bundler is deterministic and splits as asked
node Scripts/Legend.mjs               # RESULT: 23 diagram(s) and 7 page(s), every diagram typed and rendered
```

Per target, so a move that silently drops one is visible: `Composition` 12, `Browse` 27 and `Parity`
1, `Chat` 30 and `Parity` 1, `Verification` 28, `Runtime` 40, `SkillContract` 24 and `Parity` 2,
`Workflow` 12, seven doc-test targets at 0.

The ten expressions the driver and the workflow build were also dumped byte for byte before anything
moved, into `.hermes/tmp/expr-before/`, by a harness copied into `Crate/Agentic/tests/`, run, and
removed again. That directory is the authority for Phase 4's "a move is not a rewrite" gate; a
rebuild of it after the move diffs clean or the phase is wrong.

---

## A. Paths naming `rust/`

Twenty-six tracked files name it. Three of those matches are not this repository's path at all -
`r/rust/…` inside a captured Reddit URL - and are marked _external_.

| File                                                                                                  | Occurrences   | What it is                                                                           |
| ----------------------------------------------------------------------------------------------------- | ------------- | ------------------------------------------------------------------------------------ |
| `.gitignore`                                                                                          | 2             | `rust/Target/` and the comment explaining it                                         |
| `.prettierignore`                                                                                     | 1             | `rust/Target/`                                                                       |
| `.vscode/settings.json`                                                                               | 3             | `rust/Target/**` twice, `rust/Cargo.lock` once                                       |
| `README.md`                                                                                           | 3             | the shape paragraph, the `## rust/` section, `rust/README.md` twice                  |
| `CONTRIBUTING.md`                                                                                     | 0             | names `packages/` and `src/` only                                                    |
| `Documentation/Parity.md`                                                                             | 1             | the Rust half's glob, `rust/Crate/*/Test/Parity.rs`                                  |
| `Documentation/Skill/Bounded-Sequences/SKILL.md`                                                      | 2             | `rust/Target/debug/agentic browse …`                                                 |
| `Documentation/Skill/Browser-Connect/SKILL.md`                                                        | 1             | same command line                                                                    |
| `Documentation/Skill/Browser-Eval/SKILL.md`                                                           | 1             | same command line                                                                    |
| `Documentation/Skill/Chat-Research/SKILL.md`                                                          | 1             | `rust/Target/debug/agentic chat …`                                                   |
| `Documentation/Skill/Index.md`                                                                        | 2             | `rust/Target/debug/agentic skills …`                                                 |
| `scenarios/run.mjs`                                                                                   | 1             | `cwd: join(Root, "rust")` - the Rust half's working directory                        |
| `scenarios/mutate.mjs`                                                                                | 6             | `cwd: at("rust")` and five `file: "rust/Crate/…"` mutation targets                   |
| `scenarios/lib/skill.mjs`                                                                             | 1             | a comment naming `rust/Crate/SkillContract/Test/Parity.rs`                           |
| `scenarios/{browser-cdp,chat-driver,skill-contract}.json`                                             | 1 each        | the `$comment` naming the Rust half                                                  |
| `packages/spacetime-catalog/Test/Destination.test.ts`                                                 | 1             | a comment naming `rust/Crate/Runtime/Test/Runtime.rs`                                |
| `rust/README.md`                                                                                      | 0             | the port's own map; it moves                                                         |
| `.hermes/notes/convention-inventory.md`                                                               | 16            | the previous pass's record - see [Deliberately not edited](#deliberately-not-edited) |
| `.hermes/bundle/{manifest.json,tree.txt}`                                                             | 83 each       | generated by `bundle.mjs` - see [Generated](#generated)                              |
| `.hermes/bundle/batch-0{2,3,4,5}.txt`                                                                 | 4, 30, 30, 22 | generated payloads                                                                   |
| `.hermes/bundle/batch-01.txt`, `.hermes/research-bundle/batch-01.txt`, `.hermes/research/4ffde6d5.md` | 0             | _external_ - `r/rust/` in a Reddit URL                                               |

`scenarios/run.mjs` and `scenarios/mutate.mjs` are the two that a `grep rust/` misses: they spell
the directory as a bare `"rust"` argument to `join` and `at`. They are the ones that would leave the
differential runner executing `cargo` in a directory that no longer exists, so they are named here
rather than left to the slash.

---

## B. Paths naming `packages/`

Twenty-eight tracked files. The set that matters is small; the bundles are the bulk.

| File                                                                                                                                        | Occurrences | What it is                                                                          |
| ------------------------------------------------------------------------------------------------------------------------------------------- | ----------- | ----------------------------------------------------------------------------------- |
| `pnpm-workspace.yaml`                                                                                                                       | 1           | `- "packages/*"`                                                                    |
| `package.json`                                                                                                                              | 1           | `Test`: `node --test "packages/*/Test/*.test.ts"`                                   |
| `Scripts/Boundaries.mjs`                                                                                                                    | 1           | `readdirSync(join(Root, "packages"))` and the two `join(Root, "packages", …)` walks |
| `scenarios/lib/{browse,chat,skill}.mjs`                                                                                                     | 2 each      | the three `../../packages/<name>/src/…` imports                                     |
| `scenarios/mutate.mjs`                                                                                                                      | 6           | `file: "packages/…"` mutation targets                                               |
| `scenarios/{browser-cdp,chat-driver,skill-contract}.json`                                                                                   | 1 each      | the `$comment` naming the TypeScript half                                           |
| `CONTRIBUTING.md`                                                                                                                           | 3           | the layout table, the lowercase-package paragraph                                   |
| `README.md`                                                                                                                                 | 4           | the shape paragraph, the `## packages/` section, the pnpm bullet                    |
| `Documentation/{Parity,Spacetime-Catalog}.md`                                                                                               | 2 each      | the differential run's two halves; the catalog's file list                          |
| `Crate/{BrowserCDP,ChatDriver,SkillContract}/Test/Parity.rs`, `Crate/Runtime/{Test/Runtime.rs,Source/Struct/StandIn.rs}`, `Crate/README.md` | 1 each      | doc comments naming the TypeScript half                                             |
| `.hermes/bundle/*`, `.hermes/research*`, `.hermes/notes/convention-inventory.md`                                                            | 1-55        | generated or historical                                                             |

---

## C. Paths naming `src/`

`src/` is the TypeScript packages' source directory, and it appears in three kinds of place:

- **inside a package, a relative import**: `packages/*/Test/*.ts` import `../src/index.ts`,
  `../src/Port.ts`, `../src/Session.ts`, `../src/Transport.ts`, `../src/Expressions.ts`. Nine files.
- **outside a package, a relative path across the tree**: `scenarios/lib/{browse,chat,skill}.mjs`
  import `../../packages/<name>/src/<File>.ts`. Three files.
- **in prose, as a bare `src/<File>.ts`**: `Documentation/Spacetime-Catalog.md` (6) and
  `CONTRIBUTING.md` (1). No directory is spelled, so the rename to `Source/` has to reach these by
  reading them, not by rewriting a prefix.

`packages/design-verification/Test/Verification.test.ts:445` resolves `new URL("../src/", …)` to
enumerate the package's own source files - a directory path, not an import, and the only one that
would pass a type check while pointing at nothing.

---

## D. Manifest lines that must change

**`rust/Cargo.toml`** moves to `Cargo.toml` unchanged. Both halves of it are already relative to the
workspace root, which is the point of the move:

```
members = ["Crate/*"]                                    # stays
agentic-runtime = { path = "Crate/Runtime" }             # stays, ×6
```

`rust/.cargo/config.toml` (`[build] target-dir = "Target"`) moves to `.cargo/config.toml`. Its
`target-dir` is relative to the file's parent, so `Target/` stays one directory at the new root -
and `rust/Target/`, which holds the build that produced the baseline above, stops being reachable.

`pnpm-workspace.yaml` needs `packages/*` → `Package/*`. `package.json` needs the same in the `Test`
glob. No package's own `package.json` changes: none of them names a path, and `main` is already
`Target/index.js`.

---

## E. The embedded JavaScript

Ten expressions, nine in `Crate/ChatDriver/Source/Fn/Expressions.rs` and one in
`Crate/Workflow/Source/Fn/Measure.rs`. Each is a `format!` whose arguments are already JavaScript
literals (`serde_json::to_string`), so the substitution is a text replacement of a _literal_, not a
quoting decision made at call time.

| Symbol                  | Signature            | Parameters, in the order they appear             | Bytes       |
| ----------------------- | -------------------- | ------------------------------------------------ | ----------- |
| `ReadMode`              | `(&Selectors)`       | `mode`                                           | 338         |
| `PressMode`             | `(&Selectors)`       | `mode`                                           | 416         |
| `ReadModel`             | `(&Selectors)`       | `model`                                          | 261         |
| `TypeQuestion`          | `(&Selectors, &str)` | `composer`, `text`                               | 1103        |
| `PressSubmit`           | `(&Selectors, &str)` | `composer`, `expected`, `submit`                 | 600         |
| `ReadAnswers`           | `(&Selectors)`       | `scroll`, `answer`, `busy`, `pattern`, `turn` ×2 | 1775 / 1839 |
| `ReadCollapse`          | `(&Selectors)`       | `answer`, `collapse`                             | 433         |
| `ReadTrace`             | `(&Selectors)`       | `answer`, `trace`                                | 511         |
| `ReadSources`           | `(&Selectors)`       | `answer`, `sources`                              | 378         |
| `MeasurementExpression` | `(&[String])`        | `selectors` (one JSON array)                     | 539         |

Every one of them is parameterised today; none is a constant. Three shapes the templating has to
carry, and they are not the same shape:

- **A selector or a text**: `Literal(&Selectors.Mode)`, `Literal(Text)` - a JSON string, already
  escaped, inserted where a string literal belongs.
- **A pattern that is sometimes not a pattern**: `ReadAnswers` builds `null` when
  `Selectors.Done.trim()` is empty and `new RegExp(<literal>)` otherwise. The placeholder is a whole
  expression, not a string, which is why it cannot be one of the quoted parameters.
- **A JSON array**: `MeasurementExpression` serialises `&[String]` and splices it as a literal.

Two facts about the current bytes that a rewrite would lose and a move must keep:

- `ReadAnswers` emits `const Turn = "" === '' ? Answer : Answer.closest("");` when `Turn` is empty.
  It reads like a bug and it is the page's own statement either way; the expression is moved, so the
  line moves with it. Changing it is not this card.
- `MeasurementSelectors` reads the selector list _back out of the built expression_ by finding
  `return ` and `.map(Read)`. Whatever the placeholder is called, the substituted text has to leave
  those two markers where they are, or the reader stops working while every test that only checks
  the expression still passes.

The `Done` parameter means each expression has two byte-shapes, not one: with an empty `Done`
(`null`) and with a real one (`new RegExp(...)`). Both are in the dump.

---

## F. What moves, and every removal by name

The moves are `git mv`, one per path, and nothing else:

- `rust/Crate` → `Crate`
- `rust/.cargo` → `.cargo`
- `rust/Cargo.toml` → `Cargo.toml`
- `rust/README.md` → `Crate/README.md` - the port's map stays with the port, and the root already
  has a `README.md` that describes the whole workspace and points at it.
- `packages` → `Package`
- `Package/<name>/src` → `Package/<name>/Source`, six packages, file names unchanged.

Every removal, named because a glob delete is how a live file disappears:

- `rust/` itself, once it is empty of tracked files.
- `rust/.gitignore` - its one rule was `/Target/`, and the root `.gitignore` already anchors
  `/Target/` at the new workspace root, so keeping it would leave a second file claiming to govern a
  directory that no longer exists.
- `rust/Target/` - build output, untracked and gitignored, and unreachable from the new root.
- `rust/Cargo.lock` - not tracked (the root ignores it), but moved rather than deleted so the gate
  below resolved the versions the baseline resolved.
- `Crate/Agentic/tests/ExpressionDump.rs` and its now-empty `tests/` - the Phase 4 harness, copied
  in, run, and removed again. Named here so the removal is a decision and not a leftover.

---

## Generated

`.hermes/bundle/` is `bundle.mjs`'s output, and its own manifest records the flags it was built
with: root the repository root, `--include ".rs,.md"`, `--max-files 30`, `--max-bytes 200000`, the
default exclude list. It names `docs/…` today, which the previous pass already made stale, and it
will name `rust/…` after this one. It is regenerated with those flags rather than edited, and the
regeneration is compared against the old tree for anything other than a rename before it is kept.

`.hermes/research-bundle/` has no generator in this tree - it is the older feed's output, tracked as
the record - and its only `rust/` match is external. Left alone.

---

## Deliberately not edited

- `.hermes/notes/convention-inventory.md` is the previous pass's record: it describes the tree as it
  was when that pass ran, including the `rust/` nesting it deliberately left in place (its F7) and
  the lowercase `packages/` it left in place (its F11). This card reverses both, and a record that
  rewrote itself to agree with the later decision would stop being a record.
- `.hermes/research/*.md` are captures: their bytes are what was recorded, including the `r/rust/…`
  URLs that made them show up in the scan. Never edited.
- `scenarios/generated/*.json` are `generate.mjs`'s output and name no path.
- `Documentation/Research.md` and four other documents link `.hermes/research/rust-4ffde6d5.md`,
  which does not exist - the file is `.hermes/research/4ffde6d5.md`. Observed here because the scan
  found it; it is a broken link from an earlier pass, not a path this one moves, and fixing it is a
  different edit.

---

## After - Phase 2, `Crate/` at the root

One `git mv` per path, nothing renamed by script: `rust/Crate` → `Crate`, `rust/.cargo` → `.cargo`,
`rust/Cargo.toml` → `Cargo.toml`, `rust/README.md` → `Crate/README.md`, and the two removals named
in [F](#f-what-moves-and-every-removal-by-name). `rust/` is gone.

**The manifest was verified, not assumed.** `members = ["Crate/*"]` and the six `path = "Crate/…"`
entries were already relative to the workspace root, so the move took them with it: `cargo metadata`
resolves seven crates and `cargo test --workspace` builds all seven from the new root. `.cargo`'s
`target-dir = "Target"` is relative to the file's parent, so `Target/` is one directory at the
root - the same name, one level up, and the old one was deleted rather than left as a second build
tree.

**Two defects the move exposed, both real and both fixed.**

- The three parity suites located the scenarios with
  `Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../scenarios")`, which counted the `rust/`
  level. From `Crate/<Name>` it resolves above the repository, every scenario file is silently
  dropped by the `.filter(|Path| Path.exists())` in front of it, and the suite fails with "no
  scenario files were found" - loudly, which is why it was found in the first `cargo test`. Now
  `../../scenarios`.
- `scenarios/run.mjs` ran `cargo test -p <crate> --test parity`. The previous pass renamed that
  target to `Test/Parity.rs` with `name = "Parity"`, so cargo answers "no test target named `parity`
  … a target with a similar name exists: `Parity`". The rename had left the referee broken and
  nothing ran it. Now `--test Parity`, in the call and in the message beside it.

**Every reference to `rust/`, updated.** `.gitignore`, `.prettierignore`, `.vscode/settings.json`,
`README.md` (the shape paragraph, the `## Crate/` heading, `Crate/README.md` twice, the toolchain
bullet, the gate command), `Documentation/Parity.md`, the five `Documentation/Skill/**` command
lines, `scenarios/run.mjs`, `scenarios/mutate.mjs` (the `cwd` and five mutation targets),
`scenarios/lib/skill.mjs`, the three scenario `$comment`s, and one comment in
`Package/spacetime-catalog/Test/Destination.test.ts`. `Crate/README.md` kept the prose it had; only
its `packages/` mention changed in Phase 3.

**Gate.**

```
cargo test --workspace                  # 177 passed, exit 0, from the new root
cargo clippy --workspace -- -D warnings # green
Target/debug/agentic                    # the path the five skill command lines name, now correct
node scenarios/run.mjs --compare        # 79/82 agreed, 3 unported, 0 differing
```

The 177 is the baseline's 177, over the same ten targets. The differential run is the one that
proves the two fixed paths: it executes `cargo test --test Parity` in the repository root and reads
all three scenario files.

---

## After - Phase 3, `Package/` beside it

`git mv packages Package`, then `git mv Package/<name>/src Package/<name>/Source` for all six
packages, file names unchanged.

**The edits the rename forced, in the order a reader meets them.**

- `pnpm-workspace.yaml` - `packages/*` → `Package/*`.
- `package.json` - the `Test` glob.
- `Scripts/Boundaries.mjs` - the three `join(Root, "packages", …)` walks and the message prefix.
- Nine `Package/*/Test/*.ts` imports - `../src/` → `../Source/`, including the
  `new URL("../src/", import.meta.url)` in `Verification.test.ts` that enumerates the package's own
  source and would have type-checked while pointing at nothing.
- Three `scenarios/lib/*.mjs` imports - `../../packages/<name>/src/` →
  `../../Package/<name>/Source/`.
- `scenarios/mutate.mjs` - the package directory and the six `file:` targets.
- `CONTRIBUTING.md` - the naming table's example and the paragraph under it, which had said that
  lowercase package directories and their `src/` were an ecosystem exception. Both halves of that
  claim are now false, so the paragraph is rewritten rather than patched: the package's _own_ name
  stays lowercase because npm fixes it, and `Package/`, `Source/` and `Test/` follow the rule.
- `README.md`, `Documentation/Parity.md`, `Documentation/Spacetime-Catalog.md` (including its six
  bare `src/<File>.ts` mentions, which no prefix rewrite would have reached), and five Rust doc
  comments that name the TypeScript half.

**Gate.**

```
node Scripts/Boundaries.mjs             # RESULT: every import points inward
node Scripts/Feed.mjs                   # RESULT: the bundler is deterministic and splits as asked
node Scripts/Legend.mjs                 # RESULT: 23 diagram(s) and 7 page(s), every diagram typed and rendered
pnpm run check                          # the three gates above, exit 0
node --test "Package/*/Test/*.test.ts"  # 240 tests, 240 pass, 0 fail
node scenarios/mutate.mjs --check       # five mutations, both suites reject each one
```

There is no `tsc` in this tree - the suites run TypeScript directly by type stripping, and the root
`README.md` says so - so `pnpm run check` plus the 240 tests is the type gate this card can hold
itself to. The sibling card is the one that decides whether a compiler arrives.

---

## After - Phase 4, the JavaScript as source

Ten files, one per expression: nine under `Crate/ChatDriver/Source/Script/` and one at
`Crate/Workflow/Source/Script/Measure.js`. They were **generated from the dump, never retyped**: the
inverse of the substitution above - replace the exact literal a `format!` argument produced with a
`{{name}}` placeholder - then re-substituted and compared against the dump for both the empty and
the filled `Done`. Ten templates, ten round trips, zero differences. The generator lives in
`.hermes/tmp/` (scratch) and is described by its own doc comment; it is not part of the tree.

**The Rust side stops building strings.** Each function is now
`Fill(<Name>Script, &[(parameter, value), …])`, with the value still a JavaScript literal produced
by the same `Literal` function as before. The parameters are the ones tabulated in
[E](#e-the-embedded-javascript); `Label()` is gone, because the snippet it returned is written into
the two scripts that use it, and a script is now a whole file rather than an assembly.

**Where the filler lives.** `AgenticRuntime::Fill` and `AgenticRuntime::Unfilled`, in
`Crate/Runtime/Source/Fn/Script.rs`. Both the capability and the workflow need the same substitution
rule and both already depend on the runtime; the alternative was two copies of it or a new crate,
and a new crate is what the sibling card would have to delete again. `Unfilled` exists so the rule
is testable without a browser: it names the placeholder a string still carries.

**The placeholder that is a failure, not a literal.** One new test in each of the two crates, in a
`#[cfg(test)] mod` inside the module that owns the scripts, because what it asserts is a property of
a private `include_str!` constant and an integration test in `Test/` cannot see one. Each test does
two things: every parameter a function declares appears in its file as `{{name}}`, and every
expression the function builds has nothing left unfilled. The second half alone would pass on a
template with no placeholders at all, which is why the raw script is asserted to carry one first.

**Gate - the move, not a rewrite.** The same harness that produced the baseline ran again after the
extraction and the two directories were diffed:

```
diff -r .hermes/tmp/expr-before .hermes/tmp/expr-after
# identical: 19 files, every expression byte for byte
cargo test --workspace                  # 179 passed, exit 0
node scenarios/run.mjs --compare        # 79/82 agreed, 3 unported, 0 differing
```

**The count is 179, not 177, and that is the one place this card departs from "same test count".**
The two additions are the tests above, which Phase 4 requires in the same sentence that asks for the
extraction; no existing test was changed, skipped or removed, and the ten pre-existing targets still
report the numbers they reported before the move (12, 27+1, 30+1, 28, 40, 24+2, 12). The two new
ones land in the `unittests Source/Library.rs` targets that already existed with zero tests, so no
target appeared either.

---

## Found while running the gates

**A stale mutation, fixed.** `scenarios/mutate.mjs --check` refused to run its second mutation:
`Package/chat-driver/Source/Driver.ts contains 0 copies of the text to mutate`. The `find` string
expected a `probe<unknown>(` call wrapped over four lines; the file has carried it on one line since
before this card - `git show HEAD:packages/chat-driver/Source/Driver.ts` shows the single-line
form - so the suite was already broken and nothing had run it since the previous pass renamed the
paths around it. The `find` and its matching `replace` were realigned to the text the file has; all
twelve edits now match exactly once and the five mutations are rejected by both suites. It is a
repair of a gate this card touches, not a rewrite of a mutation: the defect each one injects is
unchanged.

**Not fixed, and recorded rather than quietly carried.**

- `scenarios/browser-cdp.json`'s `$comment` says `scenarios/ts.mjs` drives the TypeScript half. That
  file does not exist; the driver is `scenarios/lib/browse.mjs`. A comment naming a file that is not
  there, from the same family as the ones this card updated - but the path is wrong rather than
  moved, so it is a different edit.
- Five documents link `.hermes/research/rust-4ffde6d5.md`, and the file is
  `.hermes/research/4ffde6d5.md`. The scan found it because the capture's name starts with `rust`;
  the capture itself is never edited.
- `cargo fmt --check` reports diffs in files this card never touched, before and after, because the
  house style (tabs, `Name:Type` with no space, PascalCase) is not rustfmt's output.
  `Crate/README.md` lists it as a verification command and it cannot be one.

---

## A concurrent card in the same workspace

`t_8c607c7e` - serialising an answer's DOM to Markdown - was running against this same directory
while this card ran, and it writes the same files: it added `Struct/Format.rs`, a `Format` field on
`Selectors`, a `Format` parameter to `ReadAnswers`, and `Source/Script/Extract.js` inside the
`Source/Script/` directory this card created. Nothing of the extraction was lost, and the two
changes compose: the serialiser is spliced through two more placeholders (`{{serialise}}`,
`{{seen}}`) whose empty form reproduces the recorded bytes, which is why the byte-identity diff
above still passes with `Format::Text`. It is recorded here because a reader comparing this note
with the tree will find a tenth script and a `format` parameter that this card did not put there,
and because two cards editing one file is the thing to know before a third one starts.

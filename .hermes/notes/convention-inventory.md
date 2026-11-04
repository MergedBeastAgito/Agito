# Convention inventory

Every place this workspace's layout departs from the house layout, and what each departure is:
`rename`, `merge`, `delete`, or `leave with a reason`. Phase 1 of this pass changed nothing; the
moves it authorised are recorded in [After](#after), which is where the test counts before and after
live.

The scan is of the tracked tree at the repository root, read with `git ls-files` and `find`. It
excludes `Target/`, `node_modules/`, `.hermes/tmp/`, `.hermes/research/` and the generated bundles,
which the card puts out of scope and which no layout rule governs.

---

## The references, as read from their trees

The card names `PlayForm/LFC` as the layout this repository is meant to follow, with
`PlayForm/Cargo/Run` adding the `Source/{Fn,Struct}` split. Both were read directly rather than
taken from the card's summary, because the summary is a sketch and the trees are the rule.

**`PlayForm/LFC`** - one workspace root, eight crates, nothing duplicated:

- `Cargo.toml` holds `[workspace] members = ["Crate/Binary", …]`, `resolver = "2"`, and
  `[workspace.package] edition = "2024"`, `publish = false`, `version = "0.0.1"`.
- `.cargo/config.toml` is two lines: `[build] target-dir = "Target"`.
- Every crate is `Crate/<Name>/{Cargo.toml, Source/}`. No crate carries `Test/`, `tests/`, or a
  `[[test]]` stanza - the reference has no tests at all.
- One documentation root, `Documentation/`, holding `Documentation/StateMachine.md`.
- `Concept/Registry.tsv` and `Catalog/LFC.catalog` are data directories, not layout: they are this
  product's knowledge inputs.
- Root also carries `Mandate.md`, `README.md`, `LICENSE`, `CODE_OF_CONDUCT.md`, `CONTRIBUTING.md`,
  and a **tracked** `Cargo.lock`.
- Its `.gitignore` lists `Target/` **and** `target/`. The working tree carries an untracked
  lowercase `target/` beside `Target/` - a build that ran before the config was read. The reference
  therefore shows the exact failure this repository's `Scripts/Feed.mjs` fixture guards against ("A
  Rust workspace builds into `Target/`, and the exclude list is written by hand in lower case. On a
  case-sensitive volume those are two directories"), and this tree does **not** repeat it:
  `find rust -maxdepth 1 -name target` prints nothing.

**`PlayForm/Cargo/Control`** - the house sibling that does carry tests, and the source of the form
this tree already uses:

- `Crate/ControlPane/Test/Harness.rs`, declared as `[[test]]` / `name = "Harness"` /
  `path = "Test/Harness.rs"`.
- One documentation root, `Documentation/`, with PascalCase section directories (`Design/`,
  `Research/`, `Provenance/`, `SimulatedTurns/`). No `docs/`.
- A root `Test/` exists for its Python suites - so a root test directory is house-compatible, and
  this tree carrying none is not a deviation.

**`PlayForm/Cargo/Run`** - one crate, no workspace, and no tests either:

- `Source/Library.rs` is the `[lib]` and every `[[bin]]`; `Source/Fn/` holds functions and
  `Source/Struct/` holds types.
- `autobins = false`, `autotests = false`, `autoexamples = false`, `autobenches = false`.
- `.cargo/config.toml` sets `[build] target-dir = "Target"` plus the release profile.
- No `Documentation/`, no `Test/`, no `tests/`: Run says nothing about either, so it cannot be the
  reference for documentation roots.

---

## Where the references disagree, and which one this tree follows

**Tests have a location, and it is `Test/` with an explicit stanza.** LFC has no test directory;
Control declares each suite by name and path. This tree follows Control, and already does so in all
seven crates. LFC's silence is not a rule - a repository that tests needs a location - and the
explicit stanza is the stronger form because it is the only thing that can keep a second location
from being legal.

**The lockfile is local-only here, tracked there.** LFC commits `Cargo.lock`; this root ignores it,
with the reason written in `.gitignore`: "a clone resolves its own versions, and a committed lock
made the tree fight `cargo update` for no benefit." Followed here: ignore. Recorded, not changed.

**`publish = false` is absent here, present there.** See R1 below.

**`Concept/` and `Catalog/` have no analogue, and are not imported.** Those are LFC's product data.
This tree's data directory is `scenarios/`, which plays the same role for a different contract, so
the shape is already represented and a second data root would be the duplication this pass exists to
remove.

---

## Baseline, before any change

Run from `rust/`, after `source ~/.envsh`:

```
cargo test --workspace
```

Exit `0`, **177 tests**, over ten test targets:

- `agentic`: `Test/Composition.rs` 12, `Source/Library.rs` 0, `Source/Main.rs` 0
- `agentic-browser-cdp`: `Test/Browse.rs` 27, `tests/parity.rs` 1
- `agentic-chat-driver`: `Test/Chat.rs` 30, `tests/parity.rs` 1
- `agentic-design-verification`: `Test/Verification.rs` 28
- `agentic-runtime`: `Test/Runtime.rs` 40
- `agentic-skill-contract`: `Test/SkillContract.rs` 24, `tests/parity.rs` 2
- `agentic-workflow`: `Test/Workflow.rs` 12
- seven `Doc-tests` targets, 0 each

And from the repository root:

```
node Scripts/Boundaries.mjs
```

prints `RESULT: every import points inward`, exit `0`.

`rust/README.md` claimed `161 tests` beside that command. The command says `177`; the number was
stale and is corrected in this pass (see [Also corrected](#also-corrected)).

---

## Findings

### F1, F2, F3 - a second test directory beside `Test/`

`rename`.

- `rust/Crate/BrowserCDP/tests/parity.rs`
- `rust/Crate/ChatDriver/tests/parity.rs`
- `rust/Crate/SkillContract/tests/parity.rs`

Three crates carry two test roots at once. The `tests/` copy is not dead: Cargo's `autotests`
defaults to true, so `tests/parity.rs` is a live, auto-discovered target that runs beside the
declared `Test/<Name>.rs`. The house form has no auto-discovered home at all - Control declares
every suite - which is why the duplicate could appear without anyone editing a manifest.

Rule: one location per concept; the house form is `Crate/<Name>/Test/<Name>.rs` plus a `[[test]]`
stanza naming it (Control's `Crate/ControlPane/Test/Harness.rs`). Each file moves to
`Test/Parity.rs` in its crate, PascalCase like the eight `Test/` files already there, and each crate
gains `[[test]]` / `name = "Parity"` / `path = "Test/Parity.rs"`, because `Test/` is not
auto-discovered and the target would otherwise silently stop running - a rename that quietly deletes
a test is worse than the duplication.

`Parity` is not a new name: it is what the target is already called on the command line, and it
matches `packages/*/Test/Parity.test.ts`, the other half of the same differential run.

### F4 - two documentation roots

`merge`.

- `docs/` - ten markdown files (`Design`, `Observability`, `Open-Questions`, `Parity`, `Reflection`,
  `Research`, `Review`, `Rust`, `Scope`, `Spacetime-Catalog`) plus the `diagram/` tree.
- `Documentation/` - `Skill/` with five skill directories and `Index.md`.

Both references carry exactly one documentation root, so a second is a deviation with no defence:
the same design record lives in two places, and the newer one (`Documentation/`) is the one the
skill corpus validator reads.

Rule: one documentation root, PascalCase, as in `LFC/Documentation/` and `Control/Documentation/`.
The ten files move to `Documentation/` keeping their names, and `docs/diagram/` becomes
`Documentation/Diagram/` - `Diagram` is a documentation _section_, so it takes the same PascalCase
form as the `Skill/` section beside it. Because the tree keeps its depth, every relative link inside
the moved files (`ring.svg`, `../.hermes/research/…`, `../../Review.md`) resolves unchanged; only
references that spell the path from the repository root are edited.

### F5 - lowercase numbered folders inside the diagram tree

`leave with a reason`.

`Documentation/Diagram/{00-rings,10-runtime,20-browser,30-chat,40-database,50-feed}/` keep their
names. The numbering is the sort key the index reads them in, every folder name is quoted as a link
from `Documentation/Diagram/Index.md`, and neither reference has a diagram tree - so there is no
rule to import, and inventing one would be exactly the "do not invent conventions the reference does
not have" failure. The card's merge also asks for existing names to be kept.

This is the one place the PascalCase rule does not reach, and it is recorded rather than argued
away: if the rule is meant to cover data-shaped folders, the follow-up is `00-Rings` … `50-Feed`,
six `git mv`s and six link edits, with no relative link breaking because the depth does not change.

### F6 - lowercase `.mmd` / `.svg` leaf names

`leave with a reason`.

`disappearing-ring.mmd`, `lifecycle.svg`, and twenty-one more pairs. The render contract is
`line9 render <file>.mmd --out <file>.svg`, and each page quotes both names as link text. Renaming
forty-six files would change no reader's behaviour and would rewrite every page's link list; the
names are the render's identity, not a directory convention.

### F7 - the workspace is nested at `rust/`, not at the repository root

`leave with a reason`.

LFC puts `Cargo.toml` and `Crate/` at the root. This root carries two ecosystems - `packages/` for
the TypeScript specification and `rust/` for the port - so hoisting the workspace would put a
`Crate/` beside a `packages/`, a `Cargo.toml` beside a `package.json`, and rewrite every path in
`README.md`, `CONTRIBUTING.md`, `Scripts/`, `scenarios/` and `packages/`. The reference is
single-ecosystem and has nothing to say about a repository that is not. The nesting is also what
keeps the rule "one `Target/` at the workspace root" literally true.

### F8 - `rust/README.md`

`leave with a reason`.

Neither reference carries a README inside a workspace directory. This one is the port's map: the
crate table, the two dependency rules, and the verification commands. It is additive, not a second
home for anything - `README.md` at the root describes the whole workspace and points here. LFC's
nearest equivalent is `Mandate.md`, which is repo governance rather than a layout slot.

### F9 - `module.json` in all seven crates

`leave with a reason`.

A deliberate extension: `agentic verify --module <name>` reads it, and `Scripts/Boundaries.mjs`
takes the import arrows from it. The reference has no analogue because it has no capability
manifest; removing them would delete the thing that makes "no capability imports an adapter"
checkable.

### F10 - `scenarios/` is lowercase at the root

`leave with a reason`.

Both implementations read it and its path is part of a shared contract: it is spelled in the
`$comment` of all three scenario files, in `scenarios/lib/{browse,chat,skill}.mjs`, in
`packages/{browser-cdp,chat-driver,skill-contract}/Test/Parity.test.ts`, and in the three Rust
parity suites. A lowercase name here is a decision about a data corpus that two languages share, not
an oversight, and renaming it is a contract change that belongs to its own card. Its role in this
tree is LFC's `Catalog/`: data the program reads, not prose.

### F11 - `packages/` is lowercase

`leave with a reason`.

The card puts it out of scope, and the reference crates say nothing about a TypeScript package tree.
Its own house style is `packages/<name>/src`, lowercase by ecosystem convention.

### F12 - `[workspace.package]` has no `publish = false`

`leave with a reason`, with a recommendation.

LFC carries `publish = false` in `[workspace.package]`; `rust/Cargo.toml` does not. This is a
missing manifest field the reference has, so it is recorded - but it is a publication policy, not a
layout rule, and Phase 2 is enumerated to the moves above. See R1.

### F13 - `Cargo.lock` is ignored here and tracked there

`leave with a reason`.

Covered under the reference disagreement above: `.gitignore` states the reason, and a clone
resolving its own versions is the decision this tree already made in writing.

### F14 - `.hermes/` has no analogue in either reference

`leave with a reason`.

It is this workspace's archive, not documentation: `research/` holds the captures the design record
cites, `tools/` the feed scripts, `notes/` this kind of note. `.gitignore` tracks `research/`,
`bundle/`, `tools/` and ignores `tmp/`. `bundle/` and `research-bundle/` are **generated** snapshots
that name the pre-change paths; `Scripts/Feed.mjs` asserts `bundle.mjs`'s output byte for byte, so
they are regenerated by the tool and never hand-edited. Recorded as a finding rather than edited:
after Phase 2 those snapshots are stale, and refreshing them is one command, not a rename.

### Compliant, checked rather than assumed

- **One build directory.** `rust/.cargo/config.toml` sets `[build] target-dir = "Target"`, and
  `find rust -maxdepth 1 -name target` prints nothing.
- **PascalCase test files.**
  `Test/{Browse,Chat,Composition,Fake,Runtime,SkillContract,Verification,Workflow}.rs`.
  `Test/Fake.rs` is a helper module (`mod Fake;` in `Test/Chat.rs`), not a stray target.
- **`Source/{Fn,Struct}` split**, one file per operation or type, all PascalCase, matching Run.
- **One `[[test]]` stanza per testing crate**, in the form Control uses.
- **`Scripts/`** is a role directory, singular and PascalCase.
- **Root files.** `README.md`, `CONTRIBUTING.md`, `CODE_OF_CONDUCT.md`, `LICENSE` all have an
  analogue in a reference; `Mandate.md` is the one LFC file with no counterpart, and it is
  governance rather than layout.

---

## Recommendations not taken

**R1 - add `publish = false` to `[workspace.package]`.** It is the reference's guard against a
`cargo publish` that would push seven internal crates to a registry under a licence they only carry.
Not done here because it changes publication behaviour rather than layout, and this pass is scoped
to renames and merges. One line, one follow-up card.

**R2 - regenerate the feed bundles** once the moves land, so `.hermes/bundle/` and
`.hermes/research-bundle/` name paths that exist. Not done here for the reason in F14: they are tool
output, and `Scripts/Feed.mjs` is the gate that owns them.

---

## After

**The moves.** One `git mv` per path, each followed by the manifest edit it forced. Nothing was
renamed by script, and no file's contents changed as part of a move.

- `rust/Crate/BrowserCDP/tests/parity.rs` → `rust/Crate/BrowserCDP/Test/Parity.rs`; `Cargo.toml`
  gained `[[test]]` / `name = "Parity"` / `path = "Test/Parity.rs"`; the emptied `tests/` removed.
- `rust/Crate/ChatDriver/tests/parity.rs` → `rust/Crate/ChatDriver/Test/Parity.rs`, same stanza.
- `rust/Crate/SkillContract/tests/parity.rs` → `rust/Crate/SkillContract/Test/Parity.rs`, same
  stanza.
- The ten `docs/*.md` → `Documentation/*.md`, names unchanged.
- `docs/diagram/` → `Documentation/Diagram/`: 53 files (23 `.mmd`, 23 `.svg`, six pages, one index),
  every name unchanged, for the reasons in F5 and F6.
- `docs/` no longer exists; `find rust/Crate -type d -name tests` prints nothing.

**Every reference to an old path, updated.** The files are listed because a half-renamed tree is the
failure this pass exists to prevent:

- `README.md` - the shape paragraph, the `## Documentation/` heading, `Documentation/Parity.md`, the
  `Diagram/` bullet, the `Legend.mjs` bullet.
- `CONTRIBUTING.md` - three occurrences, including the rule that a rename follows every quoted path.
- `.prettierignore` - the comment naming `Documentation/Research.md`.
- `Scripts/Legend.mjs` - the doc comment, the `Root` URL (`../Documentation/Diagram`), and the
  "holds no `.mmd` at all" message.
- `Documentation/{Design,Open-Questions,Research,Review,Rust,Spacetime-Catalog}.md` and
  `Documentation/Diagram/00-rings/README.md` - in-document cross references.
- `Documentation/Parity.md` - the Rust-side glob (`rust/Crate/*/Test/Parity.rs`) and the
  verification block.
- `rust/Crate/Runtime/Test/Runtime.rs`, `rust/Crate/Runtime/Source/Struct/Authority.rs`,
  `rust/Crate/SkillContract/Test/Parity.rs`, `rust/Crate/ChatDriver/Test/Parity.rs` - doc comments.
- `scenarios/{browser-cdp,chat-driver,skill-contract}.json` - the `$comment` in each, and
  `scenarios/lib/skill.mjs`.
- `packages/{browser-cdp,chat-driver,skill-contract}/Test/Parity.test.ts` - the comment naming the
  Rust half. The packages' own layout is out of scope; these are references to a path this pass
  moved, and a comment pointing at a file that no longer exists is the drift being removed.

Relative links inside the moved files needed no edit: the tree kept its depth, so `ring.svg`,
`../.hermes/research/research-575cb198.md` and `../../Review.md` resolve exactly as before.

**Deliberately not edited.** `.hermes/bundle/` and `.hermes/research-bundle/` are generated
snapshots (F14, R2); `.hermes/tmp/` is gitignored scratch; `.hermes/research/` holds the captures,
which are never edited. Each of them still names the old paths, and each is refreshed by a tool
rather than by hand.

**Counts, before and after.** Same command, same machine, `cargo test --workspace` in `rust/`:

- before: **177** tests over ten targets, exit `0`.
- after: **177** tests over ten targets, exit `0`.

Per target, unchanged: `Composition` 12; `Browse` 27 and `Parity` 1; `Chat` 30 and `Parity` 1;
`Verification` 28; `Runtime` 40; `SkillContract` 24 and `Parity` 2; `Workflow` 12; seven doc-test
targets at 0.

The three parity targets are the ones at risk, and they are the reason the stanza is not optional:
`tests/parity.rs` was auto-discovered, `Test/Parity.rs` is not. A move without the stanza would have
deleted three tests and still printed a green run of 172.

**Gates.**

```
node Scripts/Boundaries.mjs              # RESULT: every import points inward
node Scripts/Feed.mjs                    # RESULT: the bundler is deterministic and splits as asked
node Scripts/Legend.mjs                  # RESULT: 23 diagram(s) and 7 page(s), every diagram typed and rendered
cargo test --workspace                   # 177 passed, exit 0
cargo clippy --workspace -- -D warnings  # green
pnpm run check                           # the three Scripts gates above, in one command, exit 0
pnpm run Test                            # 240 TypeScript tests, 240 passed, 0 failed
```

`cargo test --workspace` names `Test/Parity.rs` three times in its run - the three relocated targets
are the ones being discovered, which is the whole point of the stanza. `pnpm run Test` covers the
three `Test/Parity.test.ts` files whose comments name the Rust half.

The changed markdown was then run through the repository's own formatter
(`node_modules/.bin/prettier --write` over the changed files), because `proseWrap: "always"` reflows
the lines the longer path widened; the move itself stayed a `git mv`.

Two gates this tree does not pass, and did not pass before this card:

- `cargo fmt --check` reports diffs in files this pass never touched -
  `Agentic/Source/Fn/Composition.rs`, `Runtime/Source/Struct/Port.rs`,
  `BrowserCDP/Source/Struct/Chrome.rs` among them - because the house style (tabs, `Name:Type` with
  no space, PascalCase) is not rustfmt's output. `rust/README.md` lists it as a verification
  command; it cannot be one.
- `cargo clippy --all-targets` reports six collapsible-`if` and one complex-type lint inside
  `ChatDriver/Test/Parity.rs`. The same code is at `HEAD:rust/Crate/ChatDriver/tests/parity.rs:454`,
  so the move relocated the lint and did not introduce it. The documented
  `cargo clippy --workspace -- -D warnings`, which does not lint test targets, is green.

---

## Also corrected

`rust/README.md` claimed `161 tests` beside `cargo test --workspace`. The command answers 177,
before and after this pass, so the number was stale rather than moved; it now says 177. One word,
and the gate above is what backs it.

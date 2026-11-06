# Skill corpus

The skills this repository ships, and the contract that keeps them loadable.

---

## The page

- `Browser-Connect/` - reaching a running browser over CDP from Rust: launch, attach, and binding
  the tab a person is already looking at, with the target enumeration a bind needs and the failure
  codes.
- `Browser-Eval/` - one expression, one budget: how to ask a live page a question and read the
  answer as data, and why a probe that mutates is not a probe.
- `Bounded-Calls/` - the `Call` that spans a loop: why its deadline is fixed when it is built, the
  three signs that an expired call is the caller's arithmetic rather than a slow page, and the
  handler task whose death leaves a connection that accepts commands and never answers them.
- `Bounded-Sequences/` - several operations in order: one budget per step, a stop on the first
  refusal, and verification by server-side state rather than by a click.
- `UI-Step-Sequence/` - a destructive UI flow as gated steps in one expression, with one report.
- `Batch-Mutation-Verify/` - a destructive action across many rows, each one verified by a server
  scalar rather than by the call returning.
- `Canvas-Driving/` - driving a canvas app over CDP: trusted input, real state, a closed loop.
- `Paginated-Scrape/` - reading a list to its end: paginate, key every row by a stable ID, export.

---

## The chat product

- `Chat-Research/` - asking a chat page a research question: the mode and model preconditions, the
  typing path that is a real edit, completion as the page's own signal plus stable text, and the
  observation stream.
- `Chat-Selectors/` - the strings the chat driver reads the page through, the ones it refuses to
  infer, and the scoping that makes each one correct.
- `Research-Conduct/` - weighing a research answer rather than trusting it: over-supply the
  inventory, read only where the verdict lands, and open the line the answer names before acting on
  it.

---

## The workspace

- `Workspace/` - orienting in this tree: the crates, the manifests, the gates, and the consumer that
  builds against the facade.
- `Crate-Extension/` - adding a crate: the layout, the `module.json`, the hand-written member entry,
  and the dependency arrows.
- `Embedded-Source/` - Rust carrying another language's source: a script, a query, a rule.
- `Script-Compilation/` - TypeScript in `Source/Script/`, compiled at build time and baked in by a
  macro: the `__PARAM_Name__` marker, and which parameters are data and which are code.
- `Skill-Authoring/` - writing, moving or checking a skill in this corpus: layout, digest, gate.
- `Contribution-Workflow/` - how a change arrives, and how a design decision is written down.
- `Diagrams/` - a page under `Documentation/Diagram`: the `.mmd` source and its `.svg` sibling.
- `Submodule-Links/` - a document links a submodule: name its own repository, never a path under the
  parent.

---

## Proof

- `Verification-Harness/` - proving a change in a tree that has no test command: assert, run, quote.
- `Cargo-Probe/` - cargo, TOML or regex behaviour: probe it in a scratch crate, do not guess.
- `Bench-Harness/` - what a harness has to measure to grade a driver that operates a page, the four
  decisions each number changes, and the fact that no harness exists in this tree.

---

## The contract

Each skill is a directory holding `SKILL.md` and, when it has supporting files,
`Reference/Index.md`. `agito skills Documentation/Skill` checks three rules and reports rather than
returns early:

- **parsable** - the frontmatter block is closed, states a non-empty `description`, and its `name`
  matches the directory lowercased;
- **cited** - every file under `Reference/` is named in `Reference/Index.md`;
- **listed** - this file names the skill, so nothing in the corpus is orphaned.

A finding is a rule, a skill and a sentence. An empty finding list is a valid corpus.

```sh
Target/debug/agito skills Documentation/Skill
Target/debug/agito skills Documentation/Skill --json
```

The digest the command prints is over each skill's name, description and reference file list - not
over its prose. So it changes when a skill is added, renamed, re-described or gains a reference, and
it does **not** change when only the body text does; two checkouts of the same corpus agree on it
because no path or modification time enters it.

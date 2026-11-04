# Skill corpus

The skills this repository ships, and the contract that keeps them loadable.

---

## The skills

- `Browser-Connect/` - reaching a running browser from Rust: launch, attach, and binding the tab a
  person is already looking at, with the target enumeration a bind needs and the three failure
  codes.
- `Browser-Eval/` - one expression, one budget: how to ask a live page a question and read the
  answer as data, and why a probe that mutates is not a probe.
- `Chat-Research/` - asking a web chat product a research question from Rust: the mode and model
  preconditions, the typing path that is a real edit, completion as the page's own signal plus
  stable text, and how to weigh what comes back.
- `Chat-Selectors/` - the eight strings the chat driver reads the page through, the three it refuses
  to infer, and the scoping that makes each one correct.
- `Bounded-Sequences/` - several operations in order: one budget per step, a stop on the first
  refusal, and verification by server-side state rather than by a click.

---

## The contract

Each skill is a directory holding `SKILL.md` and, when it has supporting files,
`Reference/Index.md`. `agentic skills Documentation/Skill` checks three rules and reports rather
than returns early:

- **parsable** - the frontmatter block is closed, states a non-empty `description`, and its `name`
  matches the directory;
- **cited** - every file under `Reference/` is named in `Reference/Index.md`;
- **listed** - this file names the skill, so nothing in the corpus is orphaned.

A finding is a rule, a skill and a sentence. An empty finding list is a valid corpus.

```sh
Target/debug/agentic skills Documentation/Skill
Target/debug/agentic skills Documentation/Skill --json
```

The digest the command prints is over each skill's name, description and reference file list - not
over its prose. So it changes when a skill is added, renamed, re-described or gains a reference, and
it does **not** change when only the body text does; two checkouts of the same corpus agree on it
because no path or modification time enters it.

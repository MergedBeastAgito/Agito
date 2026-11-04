---
name: differential-parity
description: Use when scenarios/ is the contract a change must not move.
version: 1.0.0
license: MIT
platforms: [macos, linux]
metadata:
    hermes:
        tags: [parity, scenarios, differential-testing, effects, rust]
---

# Differential Parity

A scenario file is the contract. Two implementations used to answer these files and a referee
compared them; the TypeScript side was deleted, and what survives is the part that did the work: the
case is the file, the ordered effect list is the protocol the implementation spoke, and a change
that moves either is a change to the contract rather than a change to the code.

This skill is about keeping that discipline with one implementation. It is not about re-running a
comparison - there is nothing to compare against, and `Documentation/Parity.md` records what the
comparison established while it existed and what its retirement cost: an independent opinion.
Nothing replaces that. The gate now defends whatever it encodes, so the reading has to be done by a
person before the edit, not by a second tree after it.

---

## What the suite is now

Five files under `scenarios/` state 82 cases: 18 browse, 27 chat, 26 skill, and 11 timing cases in
the two generated files. Each file has exactly one reader - `Crate/BrowserCDP/Test/Parity.rs`,
`Crate/ChatDriver/Test/Parity.rs`, `Crate/SkillContract/Test/Parity.rs` - which builds the fixture
the file describes, drives the port through it, and asserts the `expect` block.

The claim is narrow on purpose. Every case a file states is executed, every executed case names the
unit test that proves it in its `rust` field or gives a `scenario-only` reason, and the expected
result, the ordered effect list and the `reports` count are asserted as written. A case that names a
test which does not exist fails the suite that reads it, so a rename cannot leave a table pointing
at nothing.

Nothing about a case lives in a test file. That is the property worth protecting: a behaviour
changed in Rust and not in the scenario is a behaviour the contract no longer describes.

---

## Read the case before changing behaviour

Grep for the case that encodes the behaviour, then read its `expect.effects` list. It is ordered and
exact, so it tells you what the change will cost before you make it.

```sh
grep -rn "answer-never-settles" scenarios/
```

A case is `fixture` (a page, a question, a driver) plus `steps` plus `expect`. The file-level
`fixture` is the world every case is set in; a case's own `fixture` overrides it.

---

## Effects are the protocol, and the label is the fake's choice

The chat fake notes an operation by the name of the expression's kind - `evaluate:answers`,
`evaluate:mode`, `evaluate:press`, `evaluate:submit`, `evaluate:model`, `evaluate:collapse`,
`evaluate:trace`, `evaluate:sources`, `evaluate:composer` - not by the bytes it was sent. The rest
of its vocabulary is `open:<url>`, `opened:<url>`, `written:<question>`, `sent`,
`refused:submit:disabled`, `release:<url>`, `failed:<what>`, `report:<code>`.

That asymmetry is the main technique of this class: **to make an operation report more, fold the new
fact into the payload it already returns rather than adding a probe.** The fixture keys behind a
read (`answersReads`, `mode`, `submit`, `trace`, `sources`) are payload, and a case can change any
of them without moving the effect stream. A completion signal, a busy flag, an identity field and a
turn boundary all ride along in the read that was already happening.

A new read, a retry loop or an extra poll changes the stream, and that breaks every case whose
`expect.effects` counts reads - including cases about unrelated behaviour. The count is the
contract, not the answer.

**Read what the label is before assuming a payload change is free.** The label is the fake's choice
and it differs by capability: the browse fake notes `evaluate:<the expression>`, so for
`agentic.browse` the text is part of the effect and a rewording is a scenario change; the corpus
fake notes `discovered:<skill>` and `finding:<rule>`, so a detail wording is free but a rule name is
not.

---

## A case's fixture merges one level deep

`merge` copies a patched section's fields into the base section, and a patched field that is itself
an object replaces the base field whole. So a case can override `page.model` or `page.answersReads`,
and it cannot append one step to `page.trace` - that replaces the trace.

Two consequences:

- Prefer a case-level override to editing the file's shared `fixture`. The shared block is the world
  every other case is set in, and a change there is a change to all of them.
- When a case needs a richer payload, give it the whole value it needs. A partial nested object is
  silently a replacement, and the missing fields then read as absent rather than as inherited.

---

## Three ways a change can be legitimate

1. **Payload change.** The effect stream is identical and only what an operation returns is richer.
   No scenario edit.
2. **One pass, one side.** The behaviour genuinely changes, so the case, its `rust` unit test and
   the port change together in the same commit. A scenario edited alone is a specification nobody
   implements; a port edited alone is a behaviour the contract denies.
3. **A knob with the old default.** The new behaviour is configurable and its default reproduces
   what the scenarios encode, so an untouched case still passes. This is the option when the work is
   scoped to one consumer: the shared spec cannot move, so the new behaviour must be opt-in.

Anything else is a divergence wearing a fix's clothes.

---

## Run the parity target by name

```sh
cargo test -p agentic-chat-driver --test Parity
cargo test -p agentic-browser-cdp --test Parity
cargo test -p agentic-skill-contract --test Parity
```

A full `cargo test --workspace` hides which case disagreed behind a wall of unrelated passes. The
parity target is the one that prints the divergence as a sentence:

```
the Rust port disagrees with the scenario files:
<case-id>: effects: wanted [...], got [...]
```

That diff is the specification change you just made, spelled out. Read it before deciding whether to
revert the code or edit the case - the diff does not say which is right, and choosing wrongly is how
a contract drifts. The other sentences it can print name a result kind, a code, a message substring,
an observation token, a text/model/mode field, a trace, a source list or the report count.

---

## A default must be the reference behaviour

When a capability gains a signal, the reference default is what the cases already encode, not what
the product in front of you needs. `--format` absent reads the rendered text; `--selectors` merges
over the declared defaults; `Poll` is `DefaultPoll` at 250 ms. A default tuned to one product makes
every other instance of the class fail, and it makes the suite lie about what the port promises.

Then **measure the default before trusting it, and ship it off when it cannot be measured.** A
default that looks plausible and is wrong is worse than no default: `Documentation/Parity.md`
records a turn header read as a completion marker that was a duration label present from the first
token, and would have settled every turn on its own first sentence. Enable a new signal's default
only after watching one live instance end to end; otherwise leave it empty and let the documented
rule carry the behaviour.

---

## Prove the assertions bite

A green suite proves the assertions ran, not that they bite. `Documentation/Parity.md` carries five
mutations with the file and the test each must break. Run one by hand: snapshot **before** the edit,
apply exactly one occurrence, require the named test to fail, restore, and prove the tree is
unchanged.

```sh
cp Crate/ChatDriver/Source/Struct/Driver.rs "$TMPDIR/mutation.bak"
cargo test -p agentic-chat-driver --test Chat refuses_with_wrong_model_and_sends_nothing
cp "$TMPDIR/mutation.bak" Crate/ChatDriver/Source/Struct/Driver.rs && git diff --quiet
```

Two of the five are caught by the effect list alone: one leaves the result identical and only the
order of `evaluate:model` against `evaluate:submit` gives it away, and another leaves the answer's
text unchanged and only the number of `evaluate:answers` reads moves. A mutation that fails to
compile is not a catch - it proves the build, not the assertions.

The snapshot discipline is the harness's own design, and it is why this is a procedure rather than a
gate: a test that rewrites its own sources and re-runs `cargo test` can leave a tree mutated.

---

## Reference

`Reference/Index.md` - the file list for this skill.

`Documentation/Parity.md` - what the cases establish, what the comparison established while it
existed, the four skipped cases with their reasons, and the five mutations.

---

## Claim → test

| Claim                                           | Test                                                                                                                                                                             |
| ----------------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| The suite still answers every case              | `cargo test -p agentic-chat-driver --test Parity` passes with no scenario edit                                                                                                   |
| A payload change moved nothing                  | `git diff --stat scenarios/` is empty and all three parity targets are green                                                                                                     |
| A case names a test that exists                 | `cargo test -p agentic-chat-driver --test Parity every_scenario_names_a_case_the_rust_suite_has`                                                                                 |
| Every case agrees with its expectation          | `cargo test -p agentic-chat-driver --test Parity every_scenario_agrees_with_its_expectation`                                                                                     |
| The browse and corpus suites answer their files | `cargo test -p agentic-browser-cdp --test Parity` and `cargo test -p agentic-skill-contract --test Parity`                                                                       |
| A knob defaults to the reference                | `cargo test -p agentic-chat-driver --test Chat the_default_reading_is_the_rendered_text_and_a_selector_file_can_ask_for_markdown`                                                |
| An effect-list-only defect is caught            | apply `model-after-send` per `Documentation/Parity.md`, require `cargo test -p agentic-chat-driver --test Chat refuses_with_wrong_model_and_sends_nothing` to fail, then restore |
| A mutation leaves no trace                      | after restoring, `git diff --quiet` succeeds                                                                                                                                     |

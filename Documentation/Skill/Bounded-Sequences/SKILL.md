---
name: bounded-sequences
description: Use when several page operations must run in order, gated and verified.
version: 1.0.0
license: MIT
platforms: [macos, linux]
metadata:
    hermes:
        tags: [cdp, rust, sequence, batch, verification, pagination]
---

# Bounded Sequences

A sequence of page operations is a sequence of budgets, not a script with sleeps in it. Each step is
one `Call`: a deadline, a cancellation, and a result that is classified before the next step runs.

---

## One budget per step, and stop on the first refusal

```sh
rust/Target/debug/agentic browse --attach <ws> --bind <target-id> \
  --expression '<step 1>' --timeout 20000
rust/Target/debug/agentic browse --attach <ws> --bind <target-id> \
  --expression '<step 2>' --timeout 20000
```

Two rules make a sequence trustworthy:

- **A failed step ends the sequence.** A step whose result is `null`, `false` or an error is a
  refusal, not a no-op, and the next step's premise has just become a guess.
- **The budget belongs to the step, not the run.** One long deadline shared by ten steps hides which
  step spent it; ten short ones name the step that hung.

Between steps, re-read the state the next step depends on. A page that re-rendered between two
evaluations can leave the second expression pointing at an element that no longer exists, and the
failure then looks like a bad selector rather than a stale handle.

---

## A batch mutation is verified by the server, not by the click

A click that returns `true` proves the element was there and was clicked. It does not prove the
action happened. Verify each mutation by a scalar the server owns - a count, a state field, a row
that is now absent - and record it beside the input:

```
item=<id> clicked=true after=<state> ok=true
```

A batch that reports "12 of 12 clicked" and no state has reported nothing. The audit line is the
deliverable; the clicks are how it was produced.

Prefer an idempotent form where the product offers one (set-to-value over increment), because a
batch interrupted halfway is then re-runnable.

---

## A paginated read ends on a repeated key, not on a page count

Collect until the page yields an identity you have already seen, and cap the walk independently of
that. A count is a guess about a page that may add rows while you read it; a repeated id is evidence
that you have come back around.

Deduplicate on a stable id, never on position: a virtualised list re-renders rows and a positional
key renames them mid-read.

---

## A wait that settles before anything changed has not seen the change

"The value stopped changing" is true the instant the wait begins on a page that has not started
changing yet. A watcher must observe at least one change since it began before "stable" is allowed
to mean "finished", and the deadline stays the only other exit.

The symptom is a step that finishes in one poll with the old value while the new one lands seconds
later. For a chat product the same rule is why `busy` is read before stability is counted.

---

## Reference

`Reference/Index.md` - the file list for this skill.

---

## Claim → test

| Claim                                 | Test                                                                                                    |
| ------------------------------------- | ------------------------------------------------------------------------------------------------------- |
| A sequence stops on the first refusal | make step 2 return `false`; step 3 is never sent                                                        |
| A budget names the step that hung     | give one step `--timeout 2000` against a page that never answers; the error names that step's operation |
| A batch mutation is verified          | after the run, the audit line's `after=` value matches a fresh read of the same state                   |
| A paginated read stops on a repeat    | run twice over the same list: the second run's item count equals the first                              |
| A stale handle is reported as such    | remove the target element between two steps; the second step fails instead of reporting success         |

---
name: batch-mutation-verify
description: Use when a destructive action runs across many rows. Verify each by a server scalar.
version: 1.0.0
license: MIT
platforms: [macos, linux]
metadata:
    hermes:
        tags: [cdp, rust, batch, mutation, audit, verification]
        related_skills: [ui-step-sequence, bounded-sequences, paginated-scrape]
---

# Batch Mutation Verify

A batch that reports "12 of 12 clicked" has reported nothing: the click proves an element existed
and was clicked, not that the server changed. This skill is the loop that runs one destructive
action across N rows, confirms each row by a scalar the server owns, and writes an audit log a later
run can reconcile against.

---

## The loop

`agito browse` evaluates one expression and returns its value, so a batch is a loop the caller owns:
one bounded call per step, per row, each with its own `--timeout`.

1. Load the target set, deduplicated by stable ID, and the audit log. Re-read the log at start, so
   an interrupted run continues where it stopped instead of repeating its first rows.
2. Match a rendered row to its target **by stable ID only**, never by display name. A name is a
   label the page may repeat, truncate or localise; the ID is the join key.
3. For each matched row, run the step sequence (`UI-Step-Sequence/`) - one `Call` per step, stop on
   the first refusal (`Bounded-Sequences/`). A step returning `null` or `false` is a refusal, not a
   no-op.
4. Verify the mutation, then log it. A row whose effect is unproven is a failure, not a success
   awaiting confirmation.
5. Append one JSON line per action to a separate audit log. Never mutate without a matching line.

---

## Verify by a scalar the server owns

Confirm that a live count or text changed by exactly the expected amount - 1,290 to 1,289 - read
back from the page after the action, in a call of its own.

Two cheaper signals are false, and both look like success:

- **DOM absence.** A virtualised list drops off-screen rows from the DOM, so "the element is gone"
  is true of every row that scrolled away. The absence is a property of the viewport, not of the
  data.
- **A success toast.** The toast is rendered by the same code that sent the request, so it reports
  the intent, and it lingers over the failure it was meant to announce.

The scalar is the only per-item signal that can fail. A count that did not move is a mutation that
did not happen, whatever the DOM says.

---

## Reconcile on discrepancy

When the log's removed-count disagrees with the live scalar, the **scalar wins**:

1. Re-scrape the live list (`Paginated-Scrape/`) into the same shape as the target set.
2. `remaining = source ∩ live`; `false_positive = logged - live`.
3. Flip the false-positive log entries to `removed:false`, then resume.

An entry is a claim, and reconcile corrects a claim that turned out wrong rather than deleting it:
the run keeps the evidence that it was wrong.

---

## Operational rules

- **Run it detached.** A batch over hundreds of rows outlives an interactive session, and a run that
  dies with the terminal leaves rows mutated and unlogged.
- **Track failures in their own list, and bound the retries.** A row that fails the same way twice
  is a finding; retrying it forever is how a batch spends an hour on one item.
- **Fall back to the app's own search box** when scroll or load-more cannot reach some rows: surface
  each remaining item by name, match it by stable ID, mutate.
- **Mutate only rows whose stable ID is in the target set.** A row that appeared between the export
  and the run is out of scope, however similar its name.
- **Get an explicit go from the user after they have reviewed the exported list.** The export is the
  review gate; the batch is the irreversible half.
- **Keep artifacts in the workspace the user designated**, never the system temp directory.

The audit line is the sole record of what exactly changed, so it carries the ID, the action, the
scalar after the action, and the verdict:

```json
{
	"id": "<stable-id>",
	"action": "remove",
	"clicked": true,
	"after": 1289,
	"ok": true,
	"at": "<iso-8601>"
}
```

---

## Claims and how to test them

| Claim                          | Probe                                                                               | Pass condition                                                 |
| ------------------------------ | ----------------------------------------------------------------------------------- | -------------------------------------------------------------- |
| A click is not a mutation      | run a step whose click lands on a row the server refuses                            | the audit line reads `ok=false`                                |
| Verification reads the server  | after the run, compare the last line's `after` with a fresh read of the same scalar | the two agree                                                  |
| DOM absence is not removal     | scroll a virtualised list away from a row, then re-count                            | the row is absent from the DOM and present in the server count |
| The log is the record          | `wc -l` on the log against the number of actions taken                              | equal, one line per action                                     |
| A run resumes                  | kill the loop mid-batch, restart it                                                 | no ID appears twice in the log                                 |
| The scalar wins a disagreement | hand-edit one line to `removed:true` for a row that is still live                   | reconcile flips it to `false` and the row stays in scope       |
| A stuck row is bounded         | point one ID at a row that refuses the action                                       | the run ends with that ID in the failure list, the rest done   |
| Only targeted rows change      | export the list, add a row, then run                                                | the new row is untouched and absent from the log               |

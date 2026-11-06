---
name: bench-harness
description: Use when a harness must grade a driver that operates a page.
version: 1.0.0
license: MIT
platforms: [macos, linux]
metadata:
    hermes:
        tags: [benchmarking, evaluation, cdp, grading, flake, cost]
---

# Bench Harness

A harness grades the thing that operates a page - the driver that submits a question, reads the turn
and classifies completion - and reports whether a change made it worse, repeatably, over weeks. The
thing under test is the driver, not the model behind the page. Get that wrong and every number is
unreadable: a rate-limited product looks like a worse driver, and a driver that confidently returns
the previous answer looks correct.

**This tree ships no harness.** There is no task record, no trial, no run manifest, no evidence
store, no grader for driver correctness, no failure taxonomy and no bench verb - `agito --help`
lists `manifests`, `verify`, `skills`, `browse`, `chat` and `design`, and nothing else. Every
ingredient a harness is built from exists here, and this skill names each one.

It is not a test suite. A test suite proves a stated case was answered correctly against a scripted
page and a clock the test owns, and it cannot say whether the driver got better, because it never
meets a product, never costs anything and never flakes.

---

## The ingredients

- **An evidence stream, per read.** `Driver::Observe` pushes one `Observation` (`Count`, `Text`,
  `Busy`, `Fresh`, `Settled`) to a sink for every read it takes, and the sink answers
  `Control::Continue` or `Control::Stop`. `Target/debug/agito chat --observe` prints one JSON line
  per read on stderr - `{"count", "chars", "busy", "fresh", "settled"}` - so stdout stays the answer
  a caller is parsing. That line is the first record of the stream a harness needs; it is not
  persisted anywhere.
- **A settlement rule that is already the completion contract.**
  `Crate/Chat/Driver/Source/Struct/Settlement.rs` settles on the third consecutive agreement with
  the page's own busy control clear, and resets on a busy read. A harness grades this rule; it must
  not re-implement it.
- **A grader that is already pure and total.** `Verification::Check` takes measurements and
  invariants and returns `Verdict { Held, Evidence, Absent }`, where each `Evidence` carries
  `Label`, `Got`, `Want` and `Held`. It cannot fail: a reading that throws, a derivation that throws
  and a comparator that panics are each a row that did not hold. This is the shape to copy for a
  driver grader.
- **Budgets that belong to the caller.** `Call` is a deadline plus a cancellation, `Pace` is the
  clock and the wait, and `--timeout` is the whole call's budget. A harness owns the budget and
  passes it in; an adapter that keeps its own deadline makes two arms incomparable.
- **Admission.** Every crate carries `module.json` with a `state`: `Stateless`, `HoldTransport`,
  `HoldIdentity`, `HoldWorkspace`. `Target/debug/agito verify --module @agito/chat-driver` prints
  `{"ready", "state", "errors", "kind", "port", "module"}`. A trial's preflight is that report plus
  the state's own check - a liveness probe for a transport, a freshness check for an identity, a
  reset proof for a workspace.
- **Cost data, as a claim.** `module.json` carries `cost { coldMs, warmMs, network }` per crate
  (`browser-cdp` also names `processes: ["chromium"]`). These are the author's claims, not
  measurements; a harness that reports them must label them as declared.
- **The rest of a harness.** Task, trial and run records, an append-only evidence store, a grader
  for submit/correlate/complete/extract, the failure taxonomy, flake and cost per success, and a
  verb that runs a plan.

---

## Task, trial, run - and retries are not trials

- **Task**: versioned intent, required starting state, allowed actions, budgets, and outcome
  predicates. Independent of any execution.
- **Trial**: one execution of one task against one driver and one environment, containing every
  retry and recovery it needed.
- **Run**: an immutable manifest plus the trials of one planned comparison.

A deliberate repetition is a new trial; a retry inside a trial is not. Counting retries as trials
lets a driver raise its success rate by failing repeatedly, which is the opposite of the
measurement. A task states what must happen; which controls and observations establish it is the
adapter's business.

---

## The grader must not trust the driver

The driver returns `{"url", "format", "text", "model", "mode", "trace", "sources"}`, and an error is
one of `not-ready`, `wrong-mode`, `wrong-model`, `truncated`. A returned answer proves the call
returned; it does not prove the turn was the one this question caused, that the composer held the
question, or that the page had finished. Grade from evidence the driver did not produce: the page's
own thread, the submission count, the correlated turn, the text as the page renders it.

Grade these separately, because they fail independently: preconditions met; action executed once;
correct turn correlated; terminal state classified; extraction faithful; task outcome satisfied;
budgets respected. `Check`'s discipline applies whole - a grader that throws on a page defect has
ended a run instead of recording a row.

---

## Three axes, and never average them

1. **Driver correctness** - was the intended action taken exactly once, was the response bound to
   that action, was completion classified correctly, was that action's content extracted faithfully?
2. **Product availability** - was the product reachable, unthrottled, and in a configuration that
   permits the task?
3. **Answer quality** - was the content any good?

Report (1) unconditionally, (2) as its own operational figure, and (3) only where human labels or
deterministic predicates exist. Collapsing them produces a single number that moves when the product
has a bad afternoon.

---

## Correlation and completion are assertions, not conveniences

- **Position is not identity.** The driver already refuses it: `Fresh` is membership - an answer
  whose identifier was not in the thread before the question - because a virtualised thread unmounts
  the oldest turn while the newest arrives, and the count stops growing while the answer is plainly
  there.
- **A quiet period is not completion.** `Busy` is the page's own statement that it is still
  producing, read in the same expression as the text. A stability-only rule false-settles on a pause
  between tokens, a reasoning step or a slow code block.
- **A stop is not a success and not a failure.** A cancelled or failed turn also stops generating;
  the driver's own turn identity and deadline are what distinguish them, and a harness that grades
  `settled` alone will call both complete.

---

## Keep UNKNOWN, and declare capabilities before the run

Assign one primary failure cause and keep contributing ones: `SETUP`, `PRODUCT_UNAVAILABLE`,
`RATE_LIMIT`, `ACTION`, `CORRELATION`, `PREMATURE_COMPLETION`, `EXTRACTION`, `ANSWER_QUALITY`,
`BUDGET`, `HARNESS`, `UNKNOWN`. The driver's four codes are one implementation's refusal vocabulary;
they map into this taxonomy and are not a substitute for it. Forced attribution produces dashboards
that are confidently wrong.

A capability the adapter does not support is not a failed task - but support must be declared before
evaluation. Relabelling failures as unsupported afterwards is how a score is manufactured.

---

## Two lanes: replay and live

- **Controlled replay** feeds identical recorded observations into the driver. No recorded
  observations exist in this tree, so the fixtures are recorded before the lane has anything to
  feed; a harness adds nothing to replay except the records around it.
- **Live differential** runs matched tasks in separate equivalent sessions, randomising
  implementation order and interleaving pairs closely in time. It measures deployment reality and
  cannot control a changing service.

Run both. Compare semantic outcomes, never byte-identical answers or identical click traces. A port
is a reference, not an oracle: both can share one bug.

---

## Flake and cost per success are the two numbers that decide

- **Flake** is measured across repeated matched trials: the proportion of tasks with mixed outcomes,
  plus per-task intervals. One green run is a sample of one. Record the knobs that differ between
  arms - `pollMs` (default 250 ms), `--timeout`, the selector file - rather than tuning them
  mid-run.
- **Cost per success** is the total attributed cost of every attempt, failures included, divided by
  the number of successful tasks. `--observe` gives reads and characters per trial; the call gives
  wall time; `module.json` gives a declared cold/warm cost that is a claim until the harness
  measures it.

A leaderboard of success rates without these two is a ranking of how much each arm was allowed to
spend.

---

## What this skill does not claim

- No numbers. Nothing here has been measured against a live product, and no figure from this tree is
  a measurement.
- Not a rubric for answer quality - that axis needs human labels or a deterministic predicate, and
  neither exists here - and not a replacement for a test suite, of which this tree has none.

---

## Reference

`Reference/Index.md` - the file list for this skill.

---

## Claim → test

| Claim                                                 | How it is checked                                                                               |
| ----------------------------------------------------- | ----------------------------------------------------------------------------------------------- |
| Every read is available as evidence                   | `--observe` emits one JSON line per read; unasserted in this tree                               |
| An observer can end the wait without failing the call | `Control::Stop` ends the wait and returns the answer so far; unasserted in this tree            |
| Settlement is the page's signal plus three reads      | `Settlement.rs` settles on the third consecutive agreement with `busy` clear; unasserted here   |
| The budget is the caller's, not the adapter's         | `--timeout` is the whole call's budget and `Call` carries it; unasserted in this tree           |
| The grader records a failure instead of throwing      | `Verification::Check` returns a `Verdict` with the failing row in `Absent`; unasserted here     |
| An absent element fails even when every reading holds | `Verdict` carries `Absent` beside `Held`; unasserted in this tree                               |
| Admission is a JSON a harness can read                | `Target/debug/agito verify --module @agito/chat-driver` prints `"ready": true` with its `state` |
| A replay lane needs recorded observations             | no recorded observation exists in this tree                                                     |
| This tree ships no harness                            | `Target/debug/agito --help` lists no verb that records a trial or a run                         |

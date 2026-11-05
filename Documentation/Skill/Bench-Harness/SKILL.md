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

**No harness exists in this tree.** There is no task record, no trial, no run manifest, no evidence
store, no grader for driver correctness, no failure taxonomy and no bench verb - `agentic --help`
lists `manifests`, `verify`, `skills`, `browse`, `chat` and `design`, and nothing else. What exists
is every ingredient the harness would be built from, and this skill names each one.

It is not a test suite, and this tree no longer has one: the scenario suite that proved a stated
case was answered correctly, against a scripted page and a clock the test owned, is deleted. Even
while it existed it could not say whether the driver got better, because it never met a product,
never cost anything and never flaked.

---

## What exists, and what is owed

- **An evidence stream, per read.** `AgenticChatDriver::Observe` pushes one `Observation` (`Count`,
  `Text`, `Busy`, `Fresh`, `Settled`) to a sink for every read it takes, and the sink answers
  `Control::Continue` or `Control::Stop`. `Target/debug/agentic chat --observe` prints one JSON line
  per read on stderr - `{"count", "chars", "busy", "fresh", "settled"}` - so stdout stays the answer
  a caller is parsing. That line is the first record of the stream a harness needs; it is not
  persisted anywhere.
- **A settlement rule that is already the completion contract.**
  `Crate/ChatDriver/Source/Struct/Settlement.rs` settles on the third consecutive agreement with the
  page's own busy control clear, and resets on a busy read. A harness grades this rule; it must not
  re-implement it.
- **A grader that is already pure and total.** `AgenticDesignVerification::Check` takes measurements
  and invariants and returns `Verdict { Held, Evidence, Absent }`, where each `Evidence` carries
  `Label`, `Got`, `Want` and `Held`. It cannot fail: a reading that throws, a derivation that throws
  and a comparator that panics are each a row that did not hold. This is the shape to copy for a
  driver grader.
- **Budgets that belong to the caller.** `Call` is a deadline plus a cancellation, `Pace` is the
  clock and the wait, and `--timeout` is the whole call's budget. A harness owns the budget and
  passes it in; an adapter that keeps its own deadline makes two arms incomparable.
- **Admission.** Every crate carries `module.json` with a `state`: `stateless`, `holds-transport`,
  `holds-identity`, `holds-workspace`. `Target/debug/agentic verify --module @agentic/chat-driver`
  prints `{"ready", "state", "errors", "kind", "port", "module"}`. A trial's preflight is that
  report plus the state's own check - a liveness probe for a transport, a freshness check for an
  identity, a reset proof for a workspace.
- **Cost data, as a claim.** `module.json` carries `cost { coldMs, warmMs, network }` per crate
  (`browser-cdp` also names `processes: ["chromium"]`). These are the author's claims, not
  measurements; a harness that reports them must label them as declared.
- **Owed:** task/trial/run records, an append-only evidence store, a grader for
  submit/correlate/complete/ extract, the failure taxonomy, flake and cost per success, and a verb
  that runs a plan.

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

- **Controlled replay** feeds identical recorded observations into the driver. The scenario corpus
  this lane would have replayed is deleted, so the fixtures have to be recorded again before a
  harness has anything to feed; a harness adds nothing to replay except the records around it.
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

## Build order

1. Versioned task/trial/run records and an append-only evidence stream - the `--observe` line is the
   first record's shape.
2. An independent grader for submit, correlate, complete, extract, modelled on `Check`.
3. A replay lane over recorded cases whose answer is known, so the grader is calibrated before it
   scores anything.
4. A live lane whose preflight is `agentic verify --module <name>` plus the state's own check.
5. A report: first-attempt success, failure causes, latency, flake, coverage, cost per success.

**The first run exists to correct the harness, not to score anything.** Expect the metric set to
change once real trials exist; a benchmark whose first report is quoted as a result has not been
calibrated.

---

## What this skill does not claim

- No numbers. Nothing here has been measured against a live product, and no result from this tree
  should be quoted as one.
- Not a rubric for answer quality - that axis needs human labels or a deterministic predicate, and
  neither exists here - and not a replacement for a test suite, of which this tree now has none.

---

## Reference

`Reference/Index.md` - the file list for this skill.

---

## Claim → test

| Claim                                                 | How it was checked, or what is left                                                                 |
| ----------------------------------------------------- | --------------------------------------------------------------------------------------------------- |
| Every read is available as evidence                   | the chat driver's suite (deleted); `--observe` still emits the line                                 |
| An observer can end the wait without failing the call | the chat driver's suite (deleted)                                                                   |
| Settlement is the page's signal plus three reads      | the chat driver's suite (deleted)                                                                   |
| The budget is the caller's, not the adapter's         | the chat driver's suite (deleted)                                                                   |
| The grader records a failure instead of throwing      | the design-verification suite (deleted)                                                             |
| An absent element fails even when every reading holds | the design-verification suite (deleted)                                                             |
| Admission is a JSON a harness can read                | `Target/debug/agentic verify --module @agentic/chat-driver` prints `"ready": true` with its `state` |
| The replay lane would have been the scenario suite    | deleted with the corpus; there is nothing to replay against                                         |
| This tree ships no harness                            | `Target/debug/agentic --help` lists no verb that records a trial or a run                           |

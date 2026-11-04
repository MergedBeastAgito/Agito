# Benchmark harness for UI research

A research call through `agentic chat --bind <target-id>` on a freshly reset session, asking how to
build a harness that grades a UI-driving driver - first on one product, then across any UI - using
ordinary human use as the source of tasks and labels. Reproduced in full; the question is
summarised, the answer is not.

| Field      | Value                                                                 |
| ---------- | --------------------------------------------------------------------- |
| Characters | 20,500                                                                |
| Format     | markdown                                                              |
| Mode       | search                                                                |
| Model      | GPT-6.1 Sol                                                           |
| Thread     | https://www.perplexity.ai/search/a449cbb5-9a64-49fa-8a17-539fbd9a92e9 |

> Captured with `agentic chat --format markdown`: the answer is the page's own DOM, serialised by
> the page, so the three tables are the product's tables, the code blocks are fenced with their
> language, and a formula is its TeX rather than its glyphs. The earlier reading of this thread was
> `innerText`, which kept the tables as tab-separated runs and needed them rebuilt by hand. This one
> needed no repair.

**Asked.** Design a benchmark harness for UI-driving agents: (1) harness shape; (2) Perplexity
first; (3) any UI; (4) human training and tracking; plus the five things to build first.

**Answered.**

Build the harness around **contracts**, not recorded click sequences or exact answer strings. The
central test is: given a declared starting state, can this driver submit the intended request
exactly once, identify the corresponding response, distinguish completion from interruption, and
return the right content with auditable evidence?

Keep three scores separate: driver correctness, product availability, and answer quality. Otherwise
a Rust port can look worse because the product rate-limited it—or look correct because it
confidently returned the previous answer.

The design below treats your existing preconditions, turn correlation, completion rule, observation
stream, and TypeScript differential suite as the starting point—not components to replace.

## 1. HARNESS SHAPE

## Task, trial, and run

Use these definitions consistently:

| Entity | Definition                                                                                                                              | Example                                                                                                              |
| ------ | --------------------------------------------------------------------------------------------------------------------------------------- | -------------------------------------------------------------------------------------------------------------------- |
| Task   | Versioned intent, starting-state requirements, allowed actions, budgets, and outcome predicates. Independent of a particular execution. | “In a fresh conversation, submit this prompt and return its completed answer, not the welcome text or another turn.” |
| Trial  | One execution of one task against one driver and one environment instance. Includes every retry and recovery action.                    | Rust executes task `fresh-answer@3` once and encounters a streaming interruption.                                    |
| Run    | An immutable manifest plus a collection of trials for a planned comparison.                                                             | Rust commit A versus TypeScript commit B, on 30 tasks with five repetitions each.                                    |

Retries do not become new independent trials. A deliberate repetition does. Otherwise an
implementation can inflate its success rate by hiding failed attempts.

A task should contain:

```
id: followup-answer
version: 3
intent: ask_and_read
fixture: existing_conversation_v2
preconditions:
  authenticated: true
  composer: enabled
  active_generation: false
input:
  prompt: "Now give only the second item's name."
expect:
  submission_count: 1
  response_parent: submitted_turn
  terminal_state: completed
  extraction_scope: correlated_assistant_turn
  answer_predicate: fixture_second_item_name
budget:
  wall_ms: 90000
  submissions: 1
  recoveries: 1
required_capabilities: [conversation, followup, completion_detection]
```

The task expresses what must happen. The adapter decides which controls and observations establish
it.

## Runner and observation stream

The runner owns isolation, deadlines, scheduling, artifacts, and grading. The driver owns UI
interaction. The grader should not have to trust the driver’s final success flag.

Use an append-only event stream with:

- Trial ID, sequence number, monotonic timestamp, and wall-clock timestamp.
- Action intent, dispatched action, acknowledgment, and causal parent.
- Browser target/frame/navigation identifiers.
- Normalized UI observations and their supporting DOM, accessibility, screenshot, or network
  references.
- Correlation candidates and confidence/evidence—not just the selected turn.
- Lifecycle transitions, retries, timeouts, and final extraction.

Record both raw evidence and normalized events. A normalization bug must be diagnosable without
rerunning a changed website.

A useful lifecycle is:

```
preflight → ready → submitting → submitted → response_observed
          → streaming → completed
                      ↘ stopped | failed | timed_out | ambiguous
```

An empty answer is not necessarily “still generating”; a disconnected browser is not “completed”; a
stopped answer is not a successful complete answer.

CDP provides domain-specific commands and events, rather than one universal semantic interaction
log. Build a semantic observation layer above it and retain the protocol evidence
underneath.[chromedevtools](https://chromedevtools.github.io/devtools-protocol/)

## Correlation and completion are first-class assertions

Correlation should combine:

1. The pre-submit conversation snapshot.
2. Evidence that the intended user turn appeared.
3. A newly created or newly advanced assistant turn associated with that submission.
4. Conversation identity and navigation continuity.
5. Product turn/request identifiers when legitimately observable.

Do not use “the last assistant element” as the contract. It fails on stale answers, concurrent tabs,
regeneration, optimistic rendering, and delayed hydration.

Likewise, do not define completion as “text unchanged for two seconds.” Prefer an adapter-specific
terminal signal, supported by UI lifecycle evidence and final extraction. A quiet period can be a
fallback, but must produce an explicitly weaker completion classification. Network idleness alone is
not a semantic terminal condition.

## Grader and failure taxonomy

Grade independently along these axes:

- Preconditions satisfied.
- Intended action executed, with no duplicate submission.
- Correct turn correlated.
- Terminal state classified correctly.
- Extracted answer faithful to that turn.
- Task-specific outcome satisfied.
- Budgets respected.

Assign a primary failure cause and preserve contributing causes:

`SETUP`, `PRODUCT_UNAVAILABLE`, `RATE_LIMIT`, `ACTION`, `CORRELATION`, `PREMATURE_COMPLETION`,
`EXTRACTION`, `ANSWER_QUALITY`, `BUDGET`, `HARNESS`, `UNKNOWN`.

When attribution is uncertain, retain `UNKNOWN`. Forced attribution creates misleading dashboards.

## Metrics and result store

Record more than pass/fail:

| Metric                     | Definition or caution                                                                                                              |
| -------------------------- | ---------------------------------------------------------------------------------------------------------------------------------- |
| End-to-end success         | Successful tasks / all planned eligible trials; report setup failures and exclusions separately.                                   |
| Driver-contract success    | Correct submission, correlation, terminal classification, and extraction, independently of answer usefulness.                      |
| First-attempt success      | No recovery or retry required.                                                                                                     |
| Turns                      | Separate user submissions, assistant generations, and agent planning turns.                                                        |
| Action count               | Semantic actions and low-level CDP commands separately.                                                                            |
| Latency                    | Preflight, submit acknowledgment, first observed answer content, completion, extraction; report p50/p95.                           |
| Retries and interventions  | By reason; include human rescue and browser restarts.                                                                              |
| Tokens                     | Agent-model usage when metered; product usage only when actually exposed. Otherwise mark unknown, or label estimates explicitly.   |
| Flake                      | Across repeated matched trials: proportion of tasks with mixed outcomes, plus per-task success probabilities and intervals.        |
| Cost per success           | Total attributed cost of all attempts, including failures, divided by successful tasks. State subscription allocation assumptions. |
| Safety/correctness defects | Duplicate submissions, wrong-turn returns, false completion, unapproved actions.                                                   |

Store manifests and summaries in SQLite initially; put large artifacts in content-addressed storage.
Every result references task, grader, adapter, driver, browser, configuration, and artifact hashes.
A regrade creates a new grading record; it does not overwrite history.

## Comparability and the TypeScript original

Use two evaluation lanes:

- Controlled replay: feed identical recorded observations into Rust and TypeScript logic, testing
  correlation, lifecycle transitions, extraction, and error handling.
- Live differential: execute matched tasks in separate equivalent sessions, randomize driver order,
  and interleave pairs closely in time.

Replay controls the evidence but cannot establish that live UI interaction still works. Live
evaluation measures deployment reality but cannot perfectly control a changing service.

Compare semantic results—not byte-for-byte answers or identical click traces. Maintain explicit
cases for stale answers, long streaming pauses, replacement of DOM nodes, missing terminal signals,
interrupted streams, navigation, stop, regeneration, rate limits, and duplicate-submit hazards.

The TypeScript implementation is a reference, not an oracle. Both implementations can share the same
bug.

## 2. PERPLEXITY FIRST

## The smallest honest single-product harness

Start with roughly 10–15 task templates covering:

- Fresh question.
- Follow-up in an existing thread.
- Long answer requiring scrolling.
- Delayed first content and pauses during streaming.
- Stop during generation.
- Ask again after stopping.
- Regenerate, if supported by the tested product configuration.
- Stale answer already on screen.
- Navigation or reload during generation.
- Explicit failure and rate-limit handling.

Use recorded or injected observations for failures you cannot safely and reliably trigger live.
Label those results as simulated, never live product results.

Capture account tier, selected mode/model where visible, locale, viewport, browser version,
conversation state, visible experiment indicators, and timestamp. If a backend model version or
experiment assignment is unavailable, record it as unknown.

## What to record from ordinary human use

A consented recorder should capture:

- Semantic typing/input events, paste, composition completion, and the final submitted value.
- Submit mechanism: keyboard shortcut or button.
- Scroll gestures, viewport position, and relevant content visibility.
- Stop, regenerate, navigation, and thread changes.
- DOM/accessibility snapshots at important transitions.
- Screenshots at submission, first response, stop, and completion.
- Relevant network lifecycle metadata, with sensitive values removed.
- The observer’s turn-correlation and completion decisions.

Do not assume attaching CDP automatically records all physical human typing and clicking. Instrument
input listeners and browser/UI state changes; combine them with CDP observations. Chrome’s Network
tooling exposes request-level information, but that is only one part of the interaction
record.chromedevtools+1

Capture full text only where necessary and consented. For many timing or lifecycle tests, lengths,
hashes, and structure are sufficient.

## From a recording to a task

A recording is evidence of one route through the UI—not a specification.

Convert it through a short review:

1. Extract the user’s intended outcome.
2. Declare the minimum initial state needed.
3. Replace accidental coordinates and delays with semantic requirements.
4. Define acceptable terminal states and outcome predicates.
5. Remove personal data and parameterize reusable inputs.
6. Preserve the original recording as provenance.
7. Validate the task with a fresh execution.

For example, “human scrolled three times” should usually become “the extractor returns the full
correlated answer despite viewport changes,” not “the agent must scroll three times.”

A human pressing Stop provides a valuable interruption scenario. It does not prove the preceding
answer was complete, correct, or satisfactory.

## Grading free text without making the judge the benchmark

Separate extraction correctness from answer quality.

For extraction, compare the driver’s returned text against an independently captured representation
of the correlated answer, using documented normalization. Verify meaningful preservation of
paragraphs, code, links, and citations where required.

For answer quality, prefer:

1. Deterministic transformations with known results.
2. Structured outputs with schema and field predicates.
3. Stable factual questions with versioned accepted facts.
4. Source-grounded claims with manually reviewed rubrics.
5. Open-ended questions graded only for driver behavior unless human quality labels exist.

Use a small human-reviewed calibration set. An LLM judge can triage ambiguous cases, but should be
versioned, blinded to driver identity, and audited against human decisions. Its uncertain judgments
should not silently become release blockers.

Do not reward exact wording, a particular citation count, or inclusion of a magic phrase unless that
is actually the task.

## Non-determinism and live-product failure modes

For every repeated live task:

- Keep prompt, initial state, configuration, budgets, and grading version fixed.
- Randomize implementation order.
- Separate fresh-conversation tests from history-dependent tests.
- Record rate limits and product errors rather than silently waiting until success.
- Use declared backoff rules; do not bypass controls or rotate accounts to evade limits.
- Mark configuration changes as new comparison strata.
- Preserve unresolved experiment/version information as a comparability limitation.

Report two views: all-attempt operational success and driver correctness conditional on
independently established product availability. Never discard rate-limited attempts merely because
they make the headline worse.

A changed answer is not automatically a regression. A wrong-turn extraction is.

## 3. ANY UI

A shared environment abstraction is useful: BrowserGym, for example, supports multiple browser
benchmarks through a unified framework. That is evidence for sharing execution infrastructure—not
for treating every application as equivalent.[arxiv](https://arxiv.org/html/2403.07718v5)

## Capability matrix

Declare capabilities with support level and verification method:

| Capability     | Relevant distinction                                       |
| -------------- | ---------------------------------------------------------- |
| Authentication | Anonymous, authenticated, role-specific                    |
| Input          | Plain text, rich text, file upload                         |
| Conversation   | Fresh thread, follow-up, branching                         |
| Generation     | Streaming, atomic response                                 |
| Lifecycle      | Observable completion, stop, regenerate                    |
| Retrieval      | Citations, linked sources, browsing                        |
| Persistence    | History, saved artifacts                                   |
| Side effects   | Draft-only, reversible write, irreversible action          |
| Observation    | DOM, accessibility, screenshots, permitted network signals |

Unsupported is not failed. But an adapter must declare support before evaluation; it cannot relabel
failed tasks as unsupported afterward.

## Adapter contract

Keep the core interface narrow:

```
capabilities()
prepare(fixture) -> precondition_evidence
observe() -> normalized_observation
act(semantic_action) -> dispatch_evidence
correlate(submission, observations) -> turn_binding
classify_lifecycle(binding) -> state + evidence
extract(binding) -> answer_artifact
cleanup() -> cleanup_evidence
```

The runner owns deadlines and retry policy. Otherwise each adapter quietly receives a different
evaluation budget.

Provide an observation-permission profile. A DOM-only agent and an agent allowed to inspect private
application state are not comparable just because both use CDP.

The DSL should express intent, fixtures, constraints, budgets, and predicates. Product-specific
selectors belong in adapters, not task definitions. Shared temporal assertions can express rules
such as “never emit completed before terminal evidence” or “eventually return the submitted turn or
an explicit error.”

## What stays product-specific

Keep these in adapters or product task packs:

- Composer behavior and controls.
- Thread, turn, and regeneration identity.
- Meaning of terminal UI signals.
- Source/citation representation.
- Authentication, subscriptions, and rate-limit behavior.
- Fixture setup and cleanup.
- Product-specific answer rubrics.

Do not force a fake universal “message ID” where the product exposes no reliable equivalent.

## Honest cross-product reporting

Publish:

1. Common-core scores on an identical supported task intersection.
2. Per-capability scores.
3. Product-specific extension scores.
4. Coverage: which tasks and capabilities were tested.
5. Operational results, including availability and cost.
6. Repetition counts, uncertainty, and observation permissions.

If an overall score is necessary, predeclare task weights and show its components. A product
evaluated only on easy fresh questions should not outrank one evaluated on follow-ups,
stop/recovery, citations, and long answers without that difference being explicit.

Keep agent UI competence separate from the underlying product’s answer quality.

## 4. HUMAN TRAINING AND TRACKING

## Mine candidates, not automatic ground truth

Ordinary use can yield candidate task episodes:

```
intent → starting state → actions → observed outcome → optional correction
```

Segment around submissions and thread transitions. Cluster redacted episodes by interaction pattern,
then sample across frequency, novelty, failures, and product features.

Do not infer success from “the user stopped interacting.” They may have been satisfied, distracted,
or frustrated.

Collect lightweight labels at useful moments:

- “Did this achieve your goal?”
- “Was this the right answer turn?”
- “Complete, intentionally stopped, or broken?”
- Optional failure category.

Use occasional targeted review for ambiguous cases, plus periodic audits of apparently successful
ones. Reviewing only failures misses false-positive grading.

## Consent, privacy, and retention

Make recording explicit, pausable, and scoped to a designated browser profile or tab. Redact locally
before upload where feasible. Exclude credentials, cookies, authorization headers, payment data, and
unrelated tabs.

Separate permissions for recording, internal evaluation, human review, and training. Access to
operational telemetry is not automatically permission to turn it into training examples.

Perplexity documents account controls governing AI data usage and separate Enterprise privacy
arrangements; these product settings do not replace consent and retention controls for your own
recorder.perplexity+2

Use short retention for raw sensitive artifacts, longer retention for de-identified task definitions
and aggregates, encryption, access auditing, and deletion propagation into derived datasets.

## Drift over weeks

Maintain two cohorts:

- Frozen sentinels: stable tasks and graders used for longitudinal comparisons.
- Rolling production samples: recent consented episodes, used to measure current relevance.

Track weekly changes in:

- Success and failure categories.
- Correlation ambiguity and completion evidence.
- Selector or adapter fallback use.
- Latency distributions and retries.
- Configuration and observed UI fingerprints.
- Human–grader disagreement.
- Task mix and operator mix.

When a sentinel regresses, rerun a small matched diagnostic set. Determine whether the cause is the
driver, adapter, grader, browser, or product before changing expectations.

Keep old and new grader scores side by side during migrations. Otherwise a “quality improvement” may
only be a relaxed grader.

## Avoid operator overfitting, gaming, and leakage

Sample across consenting operators, languages, prompt styles, input methods, viewport sizes, and
usage intensity. Split related episodes by operator, template family, and time—not randomly by
individual recording.

Maintain three pools:

- Development tasks visible to implementers.
- Held-out release tasks with restricted artifacts.
- Rotating recent-production tasks.

Use parameterized variants, varied valid wording, different starting states, and hidden grader
predicates. Keep hidden labels out of the browser and agent context.

Defend against shortcuts:

- Validate final state independently of the driver’s report.
- Count hidden retries and duplicate submissions.
- Detect hard-coded answers with task variants.
- Treat page content as untrusted, including instructions to alter logs or grading.
- Version the rubric before comparison.
- Log exclusions and manual overrides.
- Track dataset access and exposure.

Human-trained improvements should be evaluated on untouched operators, task families, or later time
windows. Testing on the same episodes that informed a fix measures memorization as much as
generalization.

## The five highest-value things to build first, ordered

1. Versioned task/trial/run records and an append-only evidence stream. Establish identity,
   provenance, budgets, and diagnostic artifacts before building dashboards.
2. An independent grader for submission, correlation, completion, and extraction. Prioritize
   wrong-turn returns and false completion over subjective answer quality.
3. A small Rust-versus-TypeScript suite with controlled replay and interleaved live trials. Include
   interruption, stale state, duplicate-submit risk, and explicit product failures.
4. A consented human-session recorder with a lightweight task-review workflow. Turn recordings into
   reviewed specifications, not brittle macros or presumed success labels.
5. A longitudinal report showing first-attempt success, failure causes, latency, flake, coverage,
   and cost per success. Add frozen sentinels and rolling samples before attempting a cross-product
   leaderboard.

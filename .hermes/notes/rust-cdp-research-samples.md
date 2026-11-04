# Rust CDP research samples

Four questions asked through `agentic chat --bind <target-id>` against the reference product's own
thread, on the day the `agentic-browser-cdp` substrate gained an existing-tab bind. Each answer is
reproduced in full, with the wall time the call took. Mode and model are recorded because an answer
from another mode is a different answer that looks exactly like a right one.

| Sample                                                 | Characters | Wall time | Mode   | Model       |
| ------------------------------------------------------ | ---------- | --------- | ------ | ----------- |
| Differential parity beyond effect lists                | 8,960      | 35.07s    | search | GPT-6.1 Sol |
| Where an adapter boundary actually is                  | 5,934      | 19.38s    | search | GPT-6.1 Sol |
| Ranking completion signals for SPA chat UIs            | 9,354      | 39.80s    | search | GPT-6.1 Sol |
| Testability with no browser: fake page, injected clock | 7,701      | 36.74s    | search | GPT-6.1 Sol |

---

## Differential parity beyond effect lists

**Asked.** A Rust port and a TypeScript original share one differential scenario suite (ordered
effect lists, error codes, payload shapes). Beyond re-running the suite, what assertions catch drift
that effect lists cannot: payload shape, error-code mapping, timing invariants, idempotence of
release? And how should a new configurable signal be tested so its DEFAULT still reproduces the
reference behaviour exactly?

**Answered.**

Add assertions on the public contract, not just the sequence of effect names: exact serialized
values, failure classification, when results become observable, and resource ownership after
cleanup.

For a new configurable signal, the essential compatibility assertion is:

text TypeScript reference == Rust with option omitted == Rust with explicit DEFAULT

Compare all observable behavior, including timing and cleanup—not just final text.

1. Payload shape: assert the actual serialized output

Use the reference’s JSON as the expected value:

rust let actual = serde_json::to_value(rust_result)?; let expected: serde_json::Value =
load_reference_payload(case);

assert_eq!(actual, expected);

Do not compare only effect discriminators or deserialize into a type that discards fields. Compare
the serialized boundary exposed to consumers.

Include distinct fixtures for:

json {} {"partial": null} {"partial": ""} {"partial": "unfinished"}

Add focused assertions where presence matters:

rust assert!(actual.get("partial").is_none()); assert_eq!(actual["code"], expected["code"]);
assert_eq!(actual["retryable"], expected["retryable"]); assert_eq!(actual["answer"],
expected["answer"]); Test shape Failure mode caught Exact JSON equality Correct effect name with
wrong payload Missing versus null versus empty Rust optional-field serialization differs from
TypeScript Nested objects and arrays Dropped metadata, wrong nesting, changed array ordering
Unicode, newlines, quotes, code blocks Escaping or text transformation changes content
Unexpected-field assertion Rust leaks additional internal fields

Ignore JSON object-key ordering unless serialized bytes are explicitly contractual. Preserve array
ordering.

2. Error mapping: test every injected failure and collision

Use an explicit table with expected codes and payloads taken from the reference:

rust for case in failure_cases { let actual = run_with_failure(case.injection).await;

    assert_eq!(actual.public_result, case.reference_result);
    assert_eq!(actual.terminal_results.len(), 1);
    assert!(!actual.emitted_success());
    assert_eq!(actual.release_calls, case.expected_release_calls);

}

Cover failures at acquisition, input, observation, generation, and cleanup, plus cancellation and
deadline expiry.

Also test overlapping conditions:

Injection Required assertion Failure mode caught Operation fails; cleanup also fails Preserve the
reference’s primary-error/cleanup-error policy Cleanup overwrites the meaningful failure
Cancellation, then late success Exactly one cancellation result Late callback resurrects a cancelled
turn Timeout, then late failure Exactly one timeout result Terminal result changes after return
Success and deadline at the same logical time Reference-defined precedence Scheduler-dependent
classification Unknown underlying error Exact reference fallback mapping Port invents a new generic
code or leaks raw errors

Compare the entire public error envelope, not only code. Message, cause, retryability, and
partial-output placement may also be contractual.

3. Timing invariants: assert “not yet,” not just “eventually”

Use a controlled clock and explicitly drive the event queue. The harness should expose both returned
results and terminal emissions.

Threshold boundaries

For an N-read stability rule:

text N−1 qualifying equal reads -> no completion Nth qualifying equal read -> completion becomes
eligible

Specify whether the first observation counts as read 1.

rust drive_reads(&mut driver, n - 1, "A"); assert!(driver.result().is_none());
assert_eq!(driver.terminal_emissions(), 0);

drive_reads(&mut driver, 1, "A"); assert_eq!(driver.result(), expected_reference_result);

Failure mode caught: off-by-one settling.

Counter reset text A, A, B, B, ... ^ restart the qualifying run

Failure mode caught: accumulating equal reads across different answer values.

Signal gating, only in the configuration that enables it text busy=true; unchanged text for >N reads
-> no completion busy=false; remaining conditions absent -> no completion required terminal
evidence; N stable reads -> completion busy reappears during settling -> reset/block completion

Failure mode caught: settling during a streaming pause, or reusing stability accumulated before the
signal permits completion.

Do not impose this behavior on the default path unless the reference already has it.

Deadline and post-terminal behavior text deadline−ε -> pending, absent earlier terminal evidence
deadline -> reference-defined timeout/tie outcome deadline+ε -> late events cannot emit another
result

Assert that ordinary polls do not restart the overall deadline.

Failure modes caught: sliding timeouts, early timeout, late success, duplicate terminal emissions.

4. Release idempotence: inspect backend calls and ownership

Test cleanup through every supported entry path. With a reusable release handle, the shape is:

rust lease.release().await?; lease.release().await?; drop(lease);

assert_eq!(backend.successful_releases(), 1); assert_eq!(backend.active_leases(), 0);

If release consumes the handle, exercise duplicate cleanup through the actual supported paths
instead—such as explicit cleanup plus cancellation or drop.

Test Assertion Failure mode caught Repeated successful release One successful backend release Double
release Explicit release followed by drop No additional release Destructor repeats cleanup Success,
error, cancellation, timeout Correct final ownership state in every case Exceptional-path leak
Acquisition fails No release of an unacquired resource Invalid cleanup Cleanup races with
cancellation One cleanup owner; one terminal result Concurrent double cleanup Backend release fails
Exact reference retry/state policy Marking an unreleased resource released, or unintended retries

Distinguish release attempts from successful releases. “Idempotent release” does not automatically
specify whether a failed attempt may be retried.

Also assert that release is neither premature nor missing:

text before ownership ends -> resource remains owned after terminal cleanup -> reference-defined
released/error state 5. New configurable signal: prove default equivalence and opt-in divergence
Three-way comparison for every shared scenario rust let reference = run_typescript(&scenario).await;

let omitted = run_rust( &scenario, config_without_new_option(), ).await;

let explicit_default = run_rust( &scenario, config_with_new_option(DEFAULT), ).await;

assert_contract_eq(&omitted, &reference); assert_contract_eq(&explicit_default, &reference);
assert_contract_eq(&omitted, &explicit_default);

Have assert_contract_eq compare:

text ordered effects, including full payloads public result/error terminal-result count logical
completion time or specified timing boundaries release attempts and final ownership state

Failure mode caught: a changed default that still produces the same coarse effect sequence.

Test every default-construction path

Run equivalent cases through:

text configuration field omitted Config::default() builder without the setter explicit documented
default explicit null, if the input format supports it

If the reference rejects null, assert rejection rather than silently treating it as omission.

Failure mode caught: deserialization, builder, and runtime defaults disagree.

Include a scenario where the policies must differ

Use a deliberately discriminating trace:

text text remains unchanged for N reads new signal still says busy later, more text arrives
eventually, the signal permits completion

Expected results:

text omitted/default: exactly the reference result and timing

explicit signal-aware configuration: stays pending during the pause completes according to its
configured signal rule

Failure modes caught:

The supposedly optional signal silently affects the default.

The opt-in setting is ignored.

The test suite exercises only cases where both policies happen to agree.

Prove the default has no hidden signal dependency

If default behavior is supposed to bypass the new signal, install a probe that fails if invoked:

rust let probe = FailIfCalledSignalProbe::new();

let actual = run_with_probe(default_config(), probe).await;

assert_contract_eq(&actual, &reference);

For the enabled path, separately test matched, unmatched, missing, ambiguous, and failed
observations according to its documented policy.

Failure mode caught: the default gains a new selector lookup, delay, error path, or refusal despite
nominally preserving the old mode.

Compatibility boundary

Keep default-compatibility tests separate from opt-in safety tests. If the TypeScript reference
completes on text stability alone, a Rust default that now waits for a busy/completion signal is not
behaviorally identical—even if it is safer.

Preserve the reference in DEFAULT; introduce stricter behavior explicitly, or make a deliberate
versioned contract change.

---

## Where an adapter boundary actually is

**Asked.** In a capability/adapter design where product markup may live only in configuration
(selectors), and never in capability code: which concepts are safe to hard-code as generic, which
are product-specific, whether 'the newest answer is the one whose count grew' is a generic identity
rule, how to express 'this turn is finished' when a product only offers a text label, and where a
selector-based adapter inevitably leaks.

**Answered.**

The boundary is semantics versus evidence: capability code may define turns, answers, activity, and
terminal outcomes; adapter configuration must define how a particular product’s UI establishes them.
Moving CSS strings into configuration is necessary, but does not by itself remove product
assumptions.

1. Generic concepts versus product-specific bindings Safe to define in capability code
   Product-specific; belongs in the adapter definition A turn is a submitted interaction with
   driver-side identity How rendered records correlate with that submission An answer is output
   associated with a turn Which elements contain it and how blocks are assembled Busy means work
   associated with the turn remains active Stop controls, status labels, attributes, scope,
   visibility A terminal outcome is success, cancellation, or failure Which observations establish
   each outcome Settling means output remains unchanged over a window Extraction and normalisation
   rules Unobservable is distinct from false How to handle missing, duplicate, hidden, or replaced
   nodes

Generic types might be:

rust enum Observation<T> { Known(T), Unobservable(ObservationError), }

enum TerminalOutcome { Success, Cancelled, Failed, }

Do not hard-code these interpretations:

text last answer = current turn missing Stop = successful completion one submission = one new answer
element tool finished = whole turn finished

They remain product assumptions even when their selectors are configurable.

2. “Its count grew” is a product assumption, not identity

Count growth establishes a change in the rendered collection—not the identity or origin of the new
logical answer.

It assumes an append-only, fully rendered list, one matching element per answer, and no concurrent
history loading or replacement. Virtualisation breaks this correspondence because only a window of
records is mounted and DOM nodes can be recycled. web

UI behavior What breaks A new answer mounts while an old one unmounts Count stays constant; the new
answer is missed Scrolling mounts historical answers Count grows; history is mistaken for the
current answer Regeneration replaces an existing answer New generation occurs without count growth A
DOM node is recycled The same element now represents a different logical answer A placeholder
becomes the answer Count changes before actual answer output appears

The capability should request:

rust observe_answer(turn_handle) -> Result<AnswerObservation, ObservationError>

The adapter supplies correlation through configured IDs, attributes, relationships, or an explicitly
weaker strategy.

If count-based detection is all the product supports, declare it as a restricted adapter
strategy—for example, single-flight submission into an append-only, nonvirtualised list. Do not
promote it into a generic identity invariant.

3. Text-only completion: configure a text predicate

The clean solution is a declarative locator plus predicate—not a product-label comparison inside
capability code.

Illustrative configuration:

text success: scope: current_turn selector: ".product-status" cardinality: exactly_one
require_visible: true predicate: kind: text_equals value: "Finished" normalize_whitespace: true

The shared evaluator implements generic primitives:

rust enum Predicate { Exists, Visible, AttributeEquals { name: String, value: String }, TextEquals {
value: String, normalize_whitespace: bool }, }

The product-specific selector and "Finished" literal stay in configuration. Text matching is an
established locator operation distinct from CSS selection; Playwright supports exact text matching
and scoped text filters. playwright

For native CDP evaluation, implement that predicate explicitly. Do not assume Playwright’s custom
:has-text() syntax works in document.querySelector. playwright

Two conditions are essential:

Match the current turn, not an earlier turn’s completion label.

Establish that the label means the whole turn successfully finished—not merely that a search or tool
phase ended.

If configuration permits only native CSS selector strings, it cannot generally express text-label
matching. Extend the schema with typed predicates rather than violating the boundary with
product-specific code.

4. The unavoidable leak—and its containment

Selectors locate elements. They do not establish identity, lifecycle, causality, or the meaning of
absence.

For example:

text completion element missing

could mean:

text not finished temporarily unmounted wrong turn changed markup observation failed

Likewise, “Finished” might describe one substep, while an absent Stop control might mean success,
cancellation, failure, or rerendering.

The least-bad containment is a versioned adapter definition containing:

text locators and predicates current-turn scoping identity/correlation strategy cardinality
requirements output extraction rules lifecycle interpretation explicit assumptions and unsupported
cases

Have the adapter return normalised observations, not guessed Booleans:

rust struct TurnObservation { turn: TurnIdentity, busy: Observation<bool>, terminal:
Observation<Option<TerminalOutcome>>, answer: Observation<Option<AnswerSnapshot>>, }

Capability code then owns only the state machine:

rust let observation = adapter.observe(&turn_handle).await?; state_machine.apply(observation)?;

Contain the remaining assumptions with adapter contract tests: stale labels, duplicate matches,
missing controls, replaced nodes, virtualised rows, and delayed output. An ambiguous observation
must remain unresolved or become an error—not silently become idle or success.

When the configuration cannot describe a product safely, add a genuinely reusable observation
primitive or declare that case unsupported. The architectural promise is “product interpretation is
isolated and testable,” not “selectors eliminate product interpretation.”

---

## Ranking completion signals for SPA chat UIs

**Asked.** Enumerate the completion signals a CDP driver can observe - stop/cancel control,
aria-busy, network idle, streamed response end, DOM mutation quiet, text stability - rank them by
reliability with the specific false positive and false negative of each, give the fail-closed
decision procedure, and say how to verify a completion detector offline.

**Answered.**

Rank signals by whether they establish termination, rather than merely a period of silence. For a
driver that must not return partial answers, only validated, turn-correlated success evidence can
authorise success; idle and quiet signals are supporting checks.

1. Signals, reliability, and failure modes

There is no universal ordering independent of the application’s contract. The following is a
practical ranking for successful-answer detection.

Rank Signal What it actually establishes Specific false positive: reports success too early Specific
false negative: misses completed work 1 Application-level terminal-success event in a stream, or an
equivalent validated UI state The application declares the current turn successfully complete Event
describes a tool substep or another turn; DOM has not rendered the final payload yet Listener starts
late; terminal event is dropped or parser no longer recognises it 2, conditional Turn-scoped
Stop/Cancel control transitions active → inactive The UI no longer offers cancellation of that
operation Cancellation, failure, temporary unmount, or a phase transition also removes the control
Stale/duplicate control remains, or selector matches another operation 2, conditional Turn-scoped
aria-busy="true" → "false" The application says that region is no longer being updated Attribute
covers one phase only, or clears on error/cancellation Attribute remains true because of a bug or
unrelated updates in the same region 3 Raw response/stream end A particular transport response ended
Truncated output, an application error inside a successful HTTP response, or only one stage ending
Persistent connection carries multiple turns and does not close after this answer 4 Relevant DOM
mutation quiet No observed mutations occurred during the window Streaming pause, wrong subtree, or
observer attached to a replaced node Citations, timestamps, or other rendering keep mutating after
the answer is complete 4–5 Answer text unchanged for N reads Extracted text matched at sampled
instants Long pause, initial empty output, stale node, or extractor omitting code blocks
Formatting/citations keep changing extracted text; unstable extraction 6 Global network idle Tracked
network activity satisfied an inactivity rule Computation or a delayed follow-up request continues
after the quiet window Polling or unrelated long-lived requests prevent idle indefinitely

The two busy indicators are comparable; their reliability depends on scope and implementation.
Neither inherently distinguishes success from cancellation or failure.

aria-busy describes element updates, not model-generation success; false is the default. An
initially absent/false attribute therefore does not establish a completed turn. github

Likewise, CDP defines Network.loadingFinished as a request finishing loading and loadingFailed as a
load failure—not as application-level answer outcomes. chromium.googlesource

Global network idle is particularly weak: Playwright maintainers explicitly discourage relying on it
because background traffic can prevent it, while delayed activity can make it occur prematurely.
github

2. Fail-closed procedure when several signals exist

Do not majority-vote signals. Network quiet plus DOM quiet plus stable text does not override a
valid “still generating” observation.

Configure the evidence contract

For each adapter, specify:

text current-turn correlation authoritative success evidence cancellation and error evidence
required busy signals complete-answer extraction render-complete evidence, or a weaker
rendering-settle policy overall deadline

Represent missing or failed observations as Unknown, not false.

Execute this state machine text Submitted → Generating → SuccessObserved → RenderingSettled →
Complete

Any nonterminal state → Cancelled | Failed | TimedOut

Apply these rules:

Register listeners before submission and capture the prior-turn baseline.

Correlate every relevant observation to the submitted turn.

Accept explicit cancellation or failure according to documented precedence.

On validated terminal-success evidence, enter SuccessObserved; do not immediately return DOM text.

Block completion while any required busy signal remains active or unobservable.

Start the rendering-settle counter only after success evidence and required idle conditions.

Reset it if output changes, busy reappears, or turn identity becomes uncertain.

Revalidate the final snapshot and return exactly one terminal result.

On deadline expiry, return timeout with optional diagnostic partial output—not success.

A concrete acceptance predicate is:

text Complete = current_turn_identified AND successful_terminal_evidence_for_that_turn AND
all_required_busy_observations_are_known_idle AND final_output_available AND
rendering_requirement_satisfied AND no_winning_cancellation_or_error AND deadline_not_expired

For N-read settling:

rust if !success_seen || !required_signals_known_idle { previous = None; equal_reads = 0; } else if
previous.as_ref() == Some(&text) { equal_reads += 1; } else { previous = Some(text.clone());
equal_reads = 1; }

An explicit success event can establish that a fast turn occurred even if polling missed its
transient busy control. Whether observing the busy transition is mandatory belongs in the adapter
contract.

Important limitation: generation versus rendering

A generation-success event does not necessarily mean the DOM contains the full answer. Under a
strict no-partial-output contract, prefer:

The authoritative final response payload; or

A validated application state that establishes final rendering.

Post-terminal text stability is a practical rendering heuristic, not a proof against arbitrarily
delayed DOM updates.

If only busy indicators and quietness are available, without a validated success interpretation,
successful completion remains unverified.

3. When only text stability is available

Do not return verified success.

Either fail immediately:

rust Err(CompletionUnverifiable { partial })

or continue until the deadline:

rust Err(TimedOut { partial })

If useful, expose an explicitly weaker result through a separate contract:

rust ObservedAnswer { text, completion: UnverifiedStable, }

The reason is fundamental. These executions have the same observable prefix:

text A: "Hello" → silence for 30 seconds → finished B: "Hello" → silence for 30 seconds → " world"

Any finite silence-based detector that accepts A during that prefix also accepts B prematurely.
Increasing N reduces some practical risks; it does not establish termination.

4. Offline verification without a live model

Use two complementary test layers.

A. Deterministic event replay

Implement the detector as a state machine driven by events and a controlled clock:

rust fn step( state: &mut Detector, now: Timestamp, event: ObservationEvent, ) -> Vec<Effect>;

Keep the expected lifecycle in the fixture, independent of what the detector observes.

Example:

text 0 submit T 10 busy(T)=true 20 text(T)="A" 10000 text(T)="A" // Longer than every quiet
threshold. ASSERT: no success 10010 text(T)="AB" 10020 success(T) 10030 busy(T)=false 10040
text(T)="ABC" // Delayed final rendering. ... rendering condition satisfied ASSERT: one success,
payload="ABC"

Essential adversarial cases:

Fixture Required assertion Pause longer than the stability window No early success Repeated empty
reads before output No empty “completed” answer Previous turn’s terminal marker Ignored for current
turn Stop disappears due to cancellation Cancellation, not success Response ends with application
error Failure, not success Raw EOF without successful terminal evidence No promotion to verified
success Final DOM update follows terminal event Final content included Busy reappears during
settling Settling resets or blocks Required selector becomes unavailable Unknown/error, not idle
Success, cancellation, deadline tie Deterministic documented precedence Late events after terminal
result No second result or payload mutation

Assert both:

text Safety: no success before the fixture’s completion oracle correct turn and complete payload at
most one terminal result

Liveness: valid completed traces eventually succeed unresolved traces reach their deadline
irrelevant background activity does not prevent completion B. Local synthetic SPA over real CDP

Serve a fixture page and deterministic local streaming endpoint. Exercise the production Rust
adapter against it.

Script:

Stop-control insertion/removal and aria-busy changes.

Chunked responses with long pauses.

Explicit success, cancellation, error, and truncated transport.

Delayed code blocks and citations.

Replaced answer containers and recycled list nodes.

Unrelated network traffic and DOM mutations.

This catches failures the pure state-machine test cannot: selector scope, observer lifetime,
extraction omissions, node replacement, and actual CDP event handling.

Finally, mutation-test the detector: remove busy gating, accept raw EOF, count pre-terminal stable
reads, or treat unknown as idle. Each mutation should break a targeted test. A detector that passes
only happy paths has not demonstrated protection against partial answers.

---

## Testability with no browser: fake page, injected clock

**Asked.** How should a browser-driving library be structured so its RULES are testable with no
browser: a fake page and an injected clock. What must the fake express, what must the clock control,
and what is the trap of a fake that recognises the expression string it is handed rather than the
operation?

**Answered.**

Structure the library around a semantic port: the rules depend on typed page operations, an injected
clock, and an independent cancellation source—not Chromium handles or JavaScript expression strings.
The real CDP adapter and a deterministic fake implement the same port.

1. Separate policy from browser mechanics

A useful dependency structure is:

text Capability rules ├── PagePort │ ├── CDP adapter + configured selectors/predicates │ └──
Scripted fake page ├── Clock │ ├── Runtime clock │ └── Manual test clock └── Cancellation source

A schematic Rust interface:

rust trait PagePort { async fn observe(&self, turn: &TurnHandle) -> Result<TurnObservation,
PageError>;

    async fn replace_input(&self, text: &str)
        -> Result<(), PageError>;

    async fn submit(&self)
        -> Result<TurnHandle, PageError>;

    async fn release(&self)
        -> Result<(), PageError>;

}

trait Clock { fn now(&self) -> Timestamp; async fn sleep_until(&self, deadline: Timestamp); }

TurnObservation should expose evidence, not decide the policy under test:

rust struct TurnObservation { answer: Option<AnswerSnapshot>, busy: Observation<bool>, terminal:
Observation<Option<TerminalOutcome>>, }

For example, the fake reports “busy, text=A”; the capability decides whether to keep waiting. Do not
make the fake return “the detector should now finish.”

This ports-and-adapters separation also permits shared contract tests for the fake and production
adapter, rather than trusting that they mean the same thing. github

2. What the fake must express

The fake needs controllable asynchronous behavior, not merely a queue of successful values.

Scenario Fake behavior Rule assertion Operation hangs Returned future stays pending until explicitly
released or dropped Deadline/cancellation can interrupt a blocked operation Page dies mid-read Read
starts, then completes with PageClosed when triggered No partial success; correct error and cleanup
Partial answer Text appears, pauses, then grows while busy remains active Stable partial text cannot
settle the answer Control never enables Input/submit readiness remains false Bounded waiting; no
invalid submission Answer never settles Output keeps changing beyond the deadline Overall deadline
is not restarted by progress Completion evidence unavailable Required signal becomes Unobservable
Unknown is not interpreted as idle Late result Operation completes after cancellation or timeout No
second terminal result Cleanup fails Release returns a configured error Reference error precedence
and ownership policy Node/answer identity changes Observation reports a different identity
Wrong-turn output is not accepted

For hanging operations, use a gate or pending future—not a blocking loop. A fake that blocks the
executor prevents the cancellation and timer mechanisms from running.

Provide test-side synchronization:

rust let read_started = fake.on_operation_started(Operation::Observe); let read_gate =
fake.hold(Operation::Observe);

let task = spawn(run_capability(...)); read_started.await;

// The read is now genuinely in flight. fake.kill_page(); read_gate.fail(PageError::PageClosed);

That handshake avoids guessing whether the driver reached the intended point.

Keep scripts semantic:

text Observe(T) -> busy=true, text="A" Observe(T) -> pending until gate opens Observe(T) ->
PageClosed

Use recorded operations for diagnostics and contractual ordering, but do not require every
incidental poll to match a rigid script.

3. What the clock must control

All rule-level time must use the injected time source:

Overall deadline.

Poll/retry delays.

Readiness timeout.

Stability windows.

Cleanup grace period, if defined.

Prefer one absolute overall deadline:

rust let deadline = clock.now() + config.timeout;

Progress must not silently replace it with now + timeout.

Cancellation remains an independent event. The clock allows the test to schedule when cancellation
occurs; advancing time should not itself imply cancellation.

Test these separately:

text hung read + advance to deadline -> TimedOut

hung read + cancel before deadline -> Cancelled

cancel + deadline become eligible together -> documented tie-breaking outcome

The rules must race deadline and cancellation against the operation itself—not check them only
between completed reads. Otherwise one hung observe() defeats both guarantees.

Also test:

text deadline−ε -> still pending deadline reached -> terminal outcome late read completes -> no new
outcome

A clock jump is not a scheduler barrier. After advancing time, drive or await the relevant task
progress before asserting. Tokio documents that advance() does not guarantee all timers have been
processed before it returns. docs

If using Tokio’s paused clock rather than a custom clock, remember that it can auto-advance when no
other work is runnable; start_paused also requires the test-util feature. Use explicit
operation-start handshakes instead of assuming a paused clock stays stationary. tokio +1

4. The expression-string trap

Avoid this fake:

rust async fn evaluate(&self, expression: &str) -> Value { if expression.contains("innerText") {
return json!("answer"); } if expression.contains("disabled") { return json!(false); } // ... }

It recognises implementation spelling, not behavior.

That creates two failures:

False failures: renaming a variable, changing whitespace, using textContent, or combining two
evaluations breaks tests despite unchanged behavior.

False confidence: malformed or semantically wrong JavaScript still receives the canned result
because it contains the expected substring.

It can also silently answer the wrong operation: checking a composer’s text and extracting an answer
may both contain innerText.

Instead:

rust fake.set_observation(turn, observation); fake.set_submit_readiness(false);
fake.fail_next(Operation::Observe, PageError::PageClosed);

The production adapter translates those semantic operations into CDP commands and configured DOM
queries. Test that translation separately against a local fixture page.

Do not attempt to build a miniature JavaScript interpreter into the fake.

5. Keep the fake from ossifying the implementation

Use these constraints:

Assert outcomes and invariants, not the full internal call graph. Check “no submit before
readiness,” rather than “exactly three readiness reads.”

Make observations state-based where possible. Repeated reads return current state; test events
change that state.

Reserve exact call-count assertions for contracts that require them, such as at-most-once successful
release.

Keep scheduling explicit. Provide “operation started,” “deliver result,” and “run until blocked”
hooks.

Generate variations. Insert extra polls, split output into different chunks, lengthen pauses, and
deliver irrelevant events. Equivalent semantic traces should produce equivalent outcomes.

Share port-contract tests. Verify error categories, pending-operation behavior, identity, and
release semantics against the fake and a local-browser adapter fixture. Shared contracts help keep
fake and real implementations aligned. github +1

Keep a thin browser test layer. Browser-free rule tests cannot prove selectors, focus, DOM
extraction, or CDP event translation are correct.

A useful final check is a harmless refactor: replace two JavaScript evaluations with one, or change
the polling implementation without changing the timing contract. The rule tests should still pass.
If they fail because the fake no longer recognises the implementation, the test boundary is too
low-level.

---

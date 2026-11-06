# Research

Three research threads precede this workspace, one per capture. The index gives one row per thread -
what it settled, what it left open, and which `Documentation/` file consumes it - and the sections
below carry each thread's findings in the order they were produced.

**The rule:** a claim that came from a thread names its capture in a `Source:` line; a claim this
repository makes says `Decision: this repository`. One hop from statement to origin. The `:NN`
anchors name positions in the capture, so a reader can tell which part of a thread a claim was taken
from.

## Index

| Capture    | Thread                                                              | Answered                                                                                                                                                                                                                | Left open                                                                                                                                                           | Consumed by                                                                                                                                                                                                                                                          |
| ---------- | ------------------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `575cb198` | hidden properties, disclosure, and how a subscription leaks them    | the execution model that makes atomic disclosure possible; the formal frame (action-conditioned POMDP, epistemic logic as a separate question); a relational design and a reducer rule for revealing without publishing | five SpacetimeDB questions - rollback, cross-module concurrency, scheduled ordering, subscription guarantees, replication determinism - and three of its own checks | nothing yet - `Documentation/Open-Questions.md` Q1-Q8                                                                                                                                                                                                                |
| `e9b61659` | an audit of `Maintain/Source/Transport` in the consuming repository | nine assumptions the CDP transport makes, each with file and line, its break, and whether the break is silent                                                                                                           | whether any inferred break occurs on a live page - nine experiments, none run                                                                                       | nothing yet - `Documentation/Open-Questions.md` Q9-Q17                                                                                                                                                                                                               |
| `4ffde6d5` | the port of five TypeScript packages into a Cargo workspace         | crate boundaries, consumer-owned ports in Cargo terms, the CDP backend recommendation, manifest representation, the cancellation and budget traits                                                                      | the manifest field mapping and the CI check for the arrows - answered in the capture, not in the tree - plus parity and house-style verification                    | `Documentation/Rust.md` throughout; `Documentation/Design.md` §The four layers, §The manifest, §The two failures the boundary guards against; `Documentation/Reflection.md` §The trust boundary comes first, and the bisimulation paragraph closing §What the runtime reports |

## What the threads are not

Two of the three carry descriptions that do not match their content: the design thread (adapter
schema, ports-belong-to-consumers, five packages, homotopy/bisimulation, reflection) and the
repository thread (verifier defects, workflow defects, RFC board, event collision, issue triage).

- `575cb198.md` is a SpacetimeDB and hidden-information thread behind a first exchange on a chat-UI
  workflow. That exchange is the only place `adapter` appears; the hidden-properties half contains
  no occurrence of `adapter`, `bisimulation`, `homotopy` or `reflection`.
- `e9b61659.md` is a CDP transport audit of another repository's `Maintain/Source/Transport`. It
  contains no occurrence of `RFC`, `verifier`, `event collision` or `issue triage`.
- `4ffde6d5.md` matches its description.

The occurrence counts were `26` and `0`. All 26 are `adapter` in the chat-UI exchange, where it
means a framework adapter rather than a capability adapter; the hidden-properties half contains none
of the four words. Provenance that invents a consumer is worse than provenance that reports a gap,
so the rows above say "nothing yet" where that is the truth, and the consequence for the design
documents is in the next section.

## The consequence for `Documentation/`

`Documentation/Rust.md` is downstream of a thread and reads as one. `Documentation/Design.md` and
`Documentation/Reflection.md` are this repository's own position: their claims are argued from the
consuming repository's surface and from this workspace's constraints.

`e9b61659.md` is the only thread that looks at the consuming repository, and it looks at one
subsystem - the CDP transport. It is therefore evidence for the premise that the boundaries were
argued from a real surface, not evidence for any particular boundary. The inventory numbers in
`Documentation/Design.md` are measurements that no thread records.

`575cb198.md` is the odd one out: nothing in `Documentation/` consumes it. It answers a question
this workspace does not ask - how an actor's knowledge of a capability's hidden properties is
disclosed, bounded and recorded - and its answer is a data model (private truth, audience-scoped
immutable evidence, derived belief) rather than an architecture. It matters only where an
observation boundary replaces a state invariant; until then it is indexed here and left alone.

## Thread 1 - hidden properties and disclosure

**Source:** `575cb198`, 608 lines.

**Asked:** (1) is SpacetimeDB's execution model reactor-style, and what is actually documented about
concurrency and ordering; (2) what formal frame fits capabilities whose properties are hidden at
first and revealed by traces or by other actors - POMDP, epistemic logic, hidden Markov model - and
what Netrunner teaches about disclosure that is fair, legible and not degenerate; (3) how to
represent those properties in a relational database whose subscriptions push rows to clients without
leaking through the subscription itself, with the table design, the reducer rule, and where the leak
is unavoidable.

**Settled** (`:417-568`):

- Each database is a single-threaded actor: transactions in one database execute serially, different
  databases concurrently. "Reactor" is not the documented word; the architecture article says
  "actor" (`:417-425`, `:436-443`).
- Transaction guarantees are not scheduling guarantees. There is no global FIFO across clients and
  no guaranteed dispatch order for overdue scheduled reducers, so ordering has to be made explicit
  with sequence or version preconditions (`:426-435`).
- An action-conditioned POMDP is the frame for discovery, with an epistemic MDP as the close special
  case when properties are fixed; the belief update is Bayesian filtering, and a stored confidence
  number is not a calibrated model (`:443-451`, `:452-459`).
- Epistemic logic answers a different question - what each actor knows, and what they know about
  others' knowledge - and probabilities are degrees of belief rather than knowledge (`:460-466`).
- Latent truth, authorised evidence and actor belief are three records with three authorities;
  another actor's statement is testimony, not a mutation of truth (`:467-473`).
- Netrunner's lesson is rule-governed access, not face-down storage: every reveal must name the rule
  authorising its target, audience and extent, repeated probing must be bounded, a persistent
  property must not be rerolled on inspection, and "hidden again" is not "unknown again"
  (`:474-481`, `:482-489`, `:490-497`).
- Never publish a secret-bearing row and conceal it client-side: subscriptions synchronise
  authorised rows into the client cache, private tables are inaccessible, and a caller-aware view
  can project by `ctx.sender` (`:498-505`, `:514-521`).
- A schema (`CapabilityPublic` / `CapabilitySecret` / `Observation`), an `observe(target, action)`
  reducer that charges and reveals in one transaction, recorded disclosure preferred over a live
  join to current truth, traces treated as a subscription surface, and a nonleakage target stated as
  observable- transcript equality rather than equal visible values (`:505-551`).

**Left open:** the five follow-ups the thread lists (`:569-573`) - rollback, cross-module
concurrency, scheduled-reducer sequencing, subscription guarantees, replication determinism - plus
the checks embedded in its findings (dominance, racing probes, canary, paired worlds). All eight are
`Documentation/Open-Questions.md` Q1-Q8.

## Thread 2 - the transport audit

**Source:** `e9b61659`, 324 lines.

**Asked:** read `Maintain/Source/Transport` in the consuming repository, name the invariants it
assumes about browser and page, where each breaks, which breaks are silent, with file and line for
every claim, saying where it infers and what it would run.

**Settled** (`:41-301`) - nine findings, in the thread's order:

1. Post identity is stable, article _position_ is not; `toggle()` sends an index, so a reorder
   between read and click is a time-of-check/time-of-use failure that still returns the original
   identity (`:53-81`).
2. A URL is a locator, not an identity constraint: `wanted` is populated only for the id variant, so
   a URL target snapshots index 0 (`:82-106`).
3. "An article exists" is treated as "the post is ready"; a partial render is silent, and an
   unrelated article can satisfy readiness (`:107-132`).
4. Extraction assumes the DOM preserves meaning - relative permalinks, `data-testid`, English metric
   keywords - and turns "not observed" into `0`, `false` or `""` (`:133-162`).
5. Background visibility is handled asymmetrically: `reveal()` runs only for an id target in a
   temporary tab, and swallows its own errors (`:163-184`).
6. "Network idle" is a quiet timestamp, not an empty in-flight set (`:185-204`).
7. A completed click is not a verified write: the settle result is discarded, `now` defaults to the
   desired state when the control is unreadable, reload verification can return null, and existing
   tabs are never reloaded (`:205-238`).
8. CDP failures are loud, but `waitFor()` catches every evaluation error and keeps polling, so a
   permanent error surfaces as a generic readiness timeout (`:239-262`).
9. Cleanup assumes attachment succeeded: `withPage()` attaches before its cleanup try, so an
   attachment failure leaks the tab, and close errors are suppressed (`:263-281`).

The sentence worth keeping is the thread's own conclusion: the strongest signals concern connection
and element availability, the weakest concern the _meaning_ of a successful result (`:290-294`).

**Left open:** every finding is a source reading plus an inference - the thread says so at
`:47-50` - and none was run. It names the experiment for each and five follow-ups (`:296-300`). Nine
experiments, `Documentation/Open-Questions.md` Q9-Q17.

## Thread 3 - the Rust port

**Source:** `4ffde6d5`, 885 lines.

**Asked:** convert the five TypeScript packages into a Rust workspace in the house style of
`github.com/PlayForm/Run`, answering crate boundaries under "ports belong to consumers", whether the
CDP adapter can be Rust or needs a TypeScript sidecar, how the manifest and its four state classes
survive, where the runtime and cancellation live, how the inward arrows become a CI check, and how
the manifest fields map.

**Settled:**

- Keep all five as separate crates, add a composition root, add a workflow crate only when a real
  composition exists; the five must not collapse into modules of one crate (`:183-190`, `:300-303`).
- Ports belong to consumers concretely: two distinct `Browse` traits inside the two capabilities,
  concrete substrate types exported by the adapter, and one root-owned wrapper implementing both -
  with the warning that a shared "universal browser request" type quietly becomes the forbidden
  ports crate (`:333-375`).
- CDP in Rust with `chromiumoxide` behind a small session API; a TypeScript sidecar only as a
  migration bridge, with four options' failure modes tabulated and an explicit acceptance suite
  before the sidecar is removed (`:376-450`).
- Session semantics: `Ownership::{Attached, Launched}`, attach and launch as separate operations,
  disposal rules per ownership, a disconnected session means cold readiness, cancellation distinct
  from navigation failure, and "bounded evaluation" as two bounds with an escalation ladder
  (`:399-429`).
- Manifest: a checked-in `module.json` embedded with `include_str!`, validated by serde in CI, read
  by the harness without starting the application; descriptor identity separated from artifact
  identity in two stages; never `cargo run`, never `cargo test` as an admission command
  (`:451-493`).
- RunTime: Tokio in the runtime, the adapter and the root entry point only; capabilities stay
  executor-independent, with the `Call` / `Cancellation` / `Browse` trait shapes given (`:494-589`).

**Left open:** nothing in the answer itself - it reaches all six questions, ending with the
exceptions at `:807-828`; the manifest field mapping is `:697-764` and the arrows check is a
concrete script at `:598-696`. What the thread cannot supply is verification: the house style is
taken from the request because the repository could not be retrieved (`:192-194`). Parity, the
two-trait derivation, the `panic = "abort"` conflict and the library behaviours are
`Documentation/Open-Questions.md` Q18-Q24.

## Capture integrity

The captures were tracked, not ignored, so `git status` and `git log` reported an edit to them.
These digests name the exact bytes every `:NN` anchor in this file was re-anchored against:

| Capture       | Lines | SHA-256                                                            |
| ------------- | ----- | ------------------------------------------------------------------ |
| `575cb198.md` | 608   | `349853f38b32fb02e0016e5ee17b52e3caf17cafdd482a7ba0f880540c44fb0e` |
| `e9b61659.md` | 324   | `d62005e23f322affc24a7b04144670fbbf80bba80b94216b6be2ea29cd0f723d` |
| `4ffde6d5.md` | 885   | `183a81fcf7532d005e5f03fb5dd1d15e399e8332d15b9e81eb36fdf4da3b406a` |

A capture that has been tidied is no longer evidence, which is why the sections above carry the
findings rather than a pointer to them.

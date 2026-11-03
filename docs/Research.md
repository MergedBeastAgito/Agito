# Research

The three captures in `.hermes/research/` are the raw record of the threads this workspace came out
of. They are evidence, not documentation: they hold the reasoning, the confidence labels and the
experiments in the order they were produced, and they are never edited. This file is the index that
makes them usable - one row per thread, what it settled, what it left open, and which `docs/` file
consumed it.

**The rule:** every claim in `docs/` carries a `Source:` line naming a capture, or a
`Decision: this repository` line saying no capture makes it. One hop from statement to origin. A
claim with neither is unfinished, not merely undocumented.

## Index

| Capture                                                            | Thread                                                              | Answered                                                                                                                                                                                                                | Left open                                                                                                                                                           | Consumed by                                                                                                                                                                                                                         |
| ------------------------------------------------------------------ | ------------------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| [`research-575cb198.md`](../.hermes/research/research-575cb198.md) | hidden properties, disclosure, and how a subscription leaks them    | the execution model that makes atomic disclosure possible; the formal frame (action-conditioned POMDP, epistemic logic as a separate question); a relational design and a reducer rule for revealing without publishing | five SpacetimeDB questions - rollback, cross-module concurrency, scheduled ordering, subscription guarantees, replication determinism - and three of its own checks | nothing yet - `docs/Open-Questions.md` Q1-Q8                                                                                                                                                                                        |
| [`delo-e9b61659.md`](../.hermes/research/delo-e9b61659.md)         | an audit of `Maintain/Source/Transport` in the consuming repository | nine assumptions the CDP transport makes, each with file and line, its break, and whether the break is silent                                                                                                           | whether any inferred break occurs on a live page - nine experiments, none run                                                                                       | nothing yet - `docs/Open-Questions.md` Q9-Q17                                                                                                                                                                                       |
| [`rust-4ffde6d5.md`](../.hermes/research/rust-4ffde6d5.md)         | the port of five TypeScript packages into a Cargo workspace         | crate boundaries, consumer-owned ports in Cargo terms, the CDP backend recommendation, manifest representation, the cancellation and budget traits                                                                      | manifest field mapping and the CI check for the arrows - both cut off mid-answer - plus parity and house-style verification                                         | `docs/Rust.md` throughout; `docs/Design.md` §The four layers, §The manifest, §Why the packages are these five, §The two failures; `docs/Reflection.md` §The trust boundary, and the bisimulation paragraph closing §What gets built |

## What the captures are not

The brief for this pass described the first two captures as the design thread (adapter schema,
ports-belong-to-consumers, five packages, homotopy/bisimulation, reflection) and the repository
thread (verifier defects, workflow defects, RFC board, event collision, issue triage). Neither
description matches its file.

- `research-575cb198.md` is a SpacetimeDB and hidden-information thread. It contains no occurrence
  of `adapter`, `bisimulation`, `homotopy` or `reflection`.
- `delo-e9b61659.md` is a CDP transport audit of another repository's `Maintain/Source/Transport`.
  It contains no occurrence of `RFC`, `verifier`, `event collision` or `issue triage`.
- `rust-4ffde6d5.md` matches its description.

Check it rather than believing it:

```sh
grep -ciE "adapter|bisimulation|homotopy|reflection" .hermes/research/research-575cb198.md
grep -ciE "RFC|verifier|event collision|issue triage" .hermes/research/delo-e9b61659.md
```

Both answer `0`. Provenance that invents a consumer is worse than provenance that reports a gap, so
the rows above say "nothing yet" where that is the truth, and the consequence for the design
documents is in the next section.

## The consequence for `docs/`

`docs/Rust.md` is downstream of a capture and reads as one. `docs/Design.md` and
`docs/Reflection.md` are this repository's own position: their claims are argued from the consuming
repository's surface and from this workspace's constraints, and most of them are marked
`Decision: this repository` for exactly that reason.

`delo-e9b61659.md` is the only capture that looks at the consuming repository, and it looks at one
subsystem - the CDP transport. It is therefore evidence for the premise that the boundaries were
argued from a real surface, not evidence for any particular boundary. The inventory numbers in
`docs/Design.md` are measurements that no capture records.

`research-575cb198.md` is the odd one out: nothing in `docs/` consumes it. It answers a question
this workspace has not asked yet - how an actor's knowledge of a capability's hidden properties is
disclosed, bounded and recorded - and its answer is a data model (private truth, audience-scoped
immutable evidence, derived belief) rather than an architecture. It becomes load-bearing the moment
`docs/Reflection.md` needs an observation boundary instead of a state invariant; until then it is
indexed here and left alone.

## Thread 1 - hidden properties and disclosure

**Source:** [`research-575cb198.md`](../.hermes/research/research-575cb198.md), 154 lines.

**Asked:** (1) is SpacetimeDB's execution model reactor-style, and what is actually documented about
concurrency and ordering; (2) what formal frame fits capabilities whose properties are hidden at
first and revealed by traces or by other actors - POMDP, epistemic logic, hidden Markov model - and
what Netrunner teaches about disclosure that is fair, legible and not degenerate; (3) how to
represent those properties in a relational database whose subscriptions push rows to clients without
leaking through the subscription itself, with the table design, the reducer rule, and where the leak
is unavoidable.

**Settled** (`:11-142`):

- Each database is a single-threaded actor: transactions in one database execute serially, different
  databases concurrently. "Reactor" is not the documented word; the architecture article says
  "actor" (`:11`, `:17`).
- Transaction guarantees are not scheduling guarantees. There is no global FIFO across clients and
  no guaranteed dispatch order for overdue scheduled reducers, so ordering has to be made explicit
  with sequence or version preconditions (`:14`).
- An action-conditioned POMDP is the frame for discovery, with an epistemic MDP as the close special
  case when properties are fixed; the belief update is Bayesian filtering, and a stored confidence
  number is not a calibrated model (`:21`, `:24`, `:112`).
- Epistemic logic answers a different question - what each actor knows, and what they know about
  others' knowledge - and probabilities are degrees of belief rather than knowledge (`:115`).
- Latent truth, authorised evidence and actor belief are three records with three authorities;
  another actor's statement is testimony, not a mutation of truth (`:118`).
- Netrunner's lesson is rule-governed access, not face-down storage: every reveal must name the rule
  authorising its target, audience and extent, repeated probing must be bounded, a persistent
  property must not be rerolled on inspection, and "hidden again" is not "unknown again" (`:119`,
  `:122`, `:123`).
- Never publish a secret-bearing row and conceal it client-side: subscriptions synchronise
  authorised rows into the client cache, private tables are inaccessible, and a caller-aware view
  can project by `ctx.sender` (`:127`, `:132`).
- A schema (`CapabilityPublic` / `CapabilitySecret` / `Observation`), an `observe(target, action)`
  reducer that charges and reveals in one transaction, recorded disclosure preferred over a live
  join to current truth, traces treated as a subscription surface, and a nonleakage target stated as
  observable- transcript equality rather than equal visible values (`:131-141`).

**Left open:** the five follow-ups the thread lists (`:145-149`) - rollback, cross-module
concurrency, scheduled-reducer sequencing, subscription guarantees, replication determinism - plus
the checks embedded in its findings (dominance, racing probes, canary, paired worlds). All eight are
`docs/Open-Questions.md` Q1-Q8.

## Thread 2 - the transport audit

**Source:** [`delo-e9b61659.md`](../.hermes/research/delo-e9b61659.md), 155 lines.

**Asked:** read `Maintain/Source/Transport` in the consuming repository, name the invariants it
assumes about browser and page, where each breaks, which breaks are silent, with file and line for
every claim, saying where it infers and what it would run.

**Settled** (`:12-144`) - nine findings, in the thread's order:

1. Post identity is stable, article _position_ is not; `toggle()` sends an index, so a reorder
   between read and click is a time-of-check/time-of-use failure that still returns the original
   identity (`:16-26`).
2. A URL is a locator, not an identity constraint: `wanted` is populated only for the id variant, so
   a URL target snapshots index 0 (`:28-38`).
3. "An article exists" is treated as "the post is ready"; a partial render is silent, and an
   unrelated article can satisfy readiness (`:40-54`).
4. Extraction assumes the DOM preserves meaning - relative permalinks, `data-testid`, English metric
   keywords - and turns "not observed" into `0`, `false` or `""` (`:56-74`).
5. Background visibility is handled asymmetrically: `reveal()` runs only for an id target in a
   temporary tab, and swallows its own errors (`:76-84`).
6. "Network idle" is a quiet timestamp, not an empty in-flight set (`:86-94`).
7. A completed click is not a verified write: the settle result is discarded, `now` defaults to the
   desired state when the control is unreadable, reload verification can return null, and existing
   tabs are never reloaded (`:96-116`).
8. CDP failures are loud, but `waitFor()` catches every evaluation error and keeps polling, so a
   permanent error surfaces as a generic readiness timeout (`:118-128`).
9. Cleanup assumes attachment succeeded: `withPage()` attaches before its cleanup try, so an
   attachment failure leaks the tab, and close errors are suppressed (`:130-138`).

The sentence worth keeping is the thread's own conclusion: the strongest signals concern connection
and element availability, the weakest concern the _meaning_ of a successful result (`:144`).

**Left open:** every finding is a source reading plus an inference - the thread says so at `:14` -
and none was run. It names the experiment for each and five follow-ups (`:146-150`). Nine
experiments, `docs/Open-Questions.md` Q9-Q17.

## Thread 3 - the Rust port

**Source:** [`rust-4ffde6d5.md`](../.hermes/research/rust-4ffde6d5.md), 383 lines.

**Asked:** convert the five TypeScript packages into a Rust workspace in the house style of
`github.com/PlayForm/Run`, answering crate boundaries under "ports belong to consumers", whether the
CDP adapter can be Rust or needs a TypeScript sidecar, how the manifest and its four state classes
survive, where the runtime and cancellation live, how the inward arrows become a CI check, and how
the manifest fields map.

**Settled:**

- Keep all five as separate crates, add a composition root, add a workflow crate only when a real
  composition exists; the five must not collapse into modules of one crate (`:56-58`, `:161`).
- Ports belong to consumers concretely: two distinct `Browse` traits inside the two capabilities,
  concrete substrate types exported by the adapter, and one root-owned wrapper implementing both -
  with the warning that a shared "universal browser request" type quietly becomes the forbidden
  ports crate (`:189-218`).
- CDP in Rust with `chromiumoxide` behind a small session API; a TypeScript sidecar only as a
  migration bridge, with four options' failure modes tabulated and an explicit acceptance suite
  before the sidecar is removed (`:220-281`).
- Session semantics: `Ownership::{Attached, Launched}`, attach and launch as separate operations,
  disposal rules per ownership, a disconnected session means cold readiness, cancellation distinct
  from navigation failure, and "bounded evaluation" as two bounds with an escalation ladder
  (`:238-259`).
- Manifest: a checked-in `module.json` embedded with `include_str!`, validated by serde in CI, read
  by the harness without starting the application; descriptor identity separated from artifact
  identity in two stages; never `cargo run`, never `cargo test` as an admission command
  (`:283-312`).
- Runtime: Tokio in the runtime, the adapter and the root entry point only; capabilities stay
  executor-independent, with the `Call` / `Cancellation` / `Browse` trait shapes given (`:314-383`).

**Left open:** the answer is cut off mid-sentence at `:378-383`, before the manifest field mapping;
the CI check for the arrows is asserted (`:58`) but never specified; and the house style is taken
from the request because the repository could not be retrieved (`:60`). Those, plus parity, the
two-trait derivation, the `panic = "abort"` conflict and the library behaviours, are
`docs/Open-Questions.md` Q18-Q24.

## Capture integrity

`.hermes` is excluded by a global gitignore, so `git status` cannot tell you whether a capture has
been edited - it reports nothing either way. These digests can:

| Capture                | Lines | SHA-256                                                            |
| ---------------------- | ----- | ------------------------------------------------------------------ |
| `research-575cb198.md` | 154   | `e6755c3aed5852dedfdb01db2642c161d6b87832e833992955ce34ed59d6248a` |
| `delo-e9b61659.md`     | 155   | `ed2fcc9f0d9b9b4dcc7bb05725b398916519bb4f30a1081785cc089ae4ea781e` |
| `rust-4ffde6d5.md`     | 383   | `ada96a1e3a121c99fcb3a143758792dd018579e674368749a69ba0232fa6baa7` |

```sh
shasum -a 256 .hermes/research/*.md
```

A capture that has been tidied is no longer evidence.

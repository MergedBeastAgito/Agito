# Reflection and Coherence

How a system that can rewrite its own capability definitions stays honest about what it is running.
The formal apparatus is worth using only up to a point; this note says where that point is.

## The property worth naming

**Manifest-enforcement coherence:** the capability manifest a runtime reports is the manifest it
actually enforces.

Not "the manifest is correct" - correctness is a separate, unprovable-in-general claim. Coherence is
checkable: read what the system says it is, then exercise the system and compare. Every safety claim
in this workspace names one of three things: a state invariant, an observation boundary, or an
equivalence theorem. A claim that names none of them is prose. An observation boundary is the half
of a claim that says what an observation must name and record.

## The trust boundary comes first

If the mutation path can rewrite the reporter, the enforcer and the verifier, an internal self-check
has no independent basis for detecting coordinated deception - it is checking the story against
itself.

So one component sits **outside** the mutation authority, and something external attests to its
identity. In this system that is the composition root plus the process that launches it: the root
holds the manifest it bound, and the runtime can only report what it was given. The mechanism is a
source descriptor embedded in the crate and an external release descriptor binding it to the
delivered bytes, so a manifest read from the mutable store is not the only copy that exists. A
manifest read from the mutable store alone is not evidence.

## The obligation, reduced to induction

Coherence is an invariant, so the proof shape is an induction over transitions:

- establish `Inv(s0)` - the manifest bound at start-up is the manifest enforced at start-up;
- show `Inv(s) ∧ s →ℓ s' ⇒ Inv(s')` for every state-changing transition: admission, invocation,
  mutation, recovery, lease reuse, revocation.

For a mutation specifically, four things must jointly establish the invariant for the new
generation: authorisation, validation, migration, publication. If any one of them is missing, the
new generation is coherent only by accident.

Tests check instances of this; a general claim needs a proof or exhaustive checking of a finite
model. The cheap version that is still honest is a transition table listing every state-changing
operation beside the invariant it must preserve, and a conformance run per generation.

## Where the apparatus collapses

If reflection only reads and replaces **declarative records under a fixed interpreter** - which is
what this workspace is - then none of the higher-order machinery is needed. The requirement is
ordinary versioning:

- a versioned contract,
- a validator,
- a compiler or installer,
- a migration `m: Vn → Vn+1`,
- and a test that the migration composed with enforcement preserves the contract.

No reflective tower is required merely because the records describe executable capabilities. The
check is: demonstrate that every admitted record is interpreted by the same trusted semantics.

## Where it stops being ordinary migration

If an authorised update can change the **interpreter**, the **authorisation semantics**, or the
**meaning of the manifest language**, record-shape validation is not enough. Then you need either a
semantics-preservation or refinement obligation between interpreter versions, or an explicit
declaration that the policy changed.

The distinction to enforce in every upgrade: _new policy approved_ versus _old policy preserved_.
Conflating them is how a rewrite gets described as a refactor.

## What the runtime reports

The `RunTime` port carries `Reflect`, which returns a `Binding`: the generation the runner was
constructed with, and the registrations it will admit, sorted by name. The runner holds one
generation and one registration list and derives every answer from that binding, so the manifest it
reports is the manifest it enforces. `Reflect` reads the runner's own fields and does not read the
mutable store.

Three further items are recorded and not built: a conformance run scoped to a generation, whose
result is recorded with that ID; a transition table in the runtime package listing each state-changing
operation beside the invariant it preserves; and a refusal of a mutation that would change
interpreter semantics unless the caller states which policy changed. No source file in the workspace
mentions conformance, and no transition table exists.

Bisimulation is used where it belongs - comparing executions across two implementations of the same
port, or across two interpreter versions. Congruence is the obligation when claiming those
comparisons survive inspection and mutation contexts. Neither is needed to say "this generation
enforces what it reports". The browser acceptance cases - attach and launch, navigation races,
evaluation timeout, disconnect, disposal, shared-browser ownership - are one such comparison, named
concretely rather than gestured at.

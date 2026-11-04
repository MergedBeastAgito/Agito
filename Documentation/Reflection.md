# Reflection and Coherence

How a system that can rewrite its own capability definitions stays honest about what it is running.
The formal apparatus is worth using only up to a point; this note says where that point is.

## The property worth naming

**Decision: this repository** - the term, the definition and the three things a safety claim may
name. No capture uses "manifest-enforcement coherence". The observation-boundary half of the third
item is the subject of `.hermes/research/research-575cb198.md`, which argues what an observation
must name and record; no section here draws on it yet.

**Manifest-enforcement coherence:** the capability manifest a runtime reports is the manifest it
actually enforces.

Not "the manifest is correct" - correctness is a separate, unprovable-in-general claim. Coherence is
checkable: read what the system says it is, then exercise the system and compare. Every safety claim
in this workspace should name one of three things: a state invariant, an observation boundary, or an
equivalence theorem. A claim that names none of them is prose.

## The trust boundary comes first

**Decision: this repository** - the argument that a self-check has no independent basis, and the
choice of the composition root plus the process that launches it as the party outside the mutation
authority. **Source:** `.hermes/research/rust-4ffde6d5.md` §3 for the mechanism the section relies
on: a source descriptor embedded in the crate and an external release descriptor binding it to the
delivered bytes, so a manifest read from the mutable store is not the only copy that exists.

If the mutation path can rewrite the reporter, the enforcer and the verifier, an internal self-check
has no independent basis for detecting coordinated deception - it is checking the story against
itself.

So one component must sit **outside** the mutation authority, and something external must attest to
its identity. In this system that is the composition root plus the process that launches it: the
root holds the manifest it bound, and the runtime can only report what it was given. A manifest read
from the mutable store alone is not evidence.

## The obligation, reduced to induction

**Decision: this repository** - the induction shape, the four-part mutation obligation
(authorisation, validation, migration, publication) and the transition table as the cheap version
that is still honest. No capture states any of them.

Coherence is an invariant, so the proof shape is an induction over transitions:

- establish `Inv(s0)` - the manifest bound at start-up is the manifest enforced at start-up;
- show `Inv(s) ∧ s →ℓ s' ⇒ Inv(s')` for every state-changing transition: admission, invocation,
  mutation, recovery, lease reuse, revocation.

For a mutation specifically, four things must jointly establish the invariant for the new
generation: authorisation, validation, migration, publication. If any one of them is missing, the
new generation is coherent only by accident.

Tests check instances of this; a general claim needs a proof or exhaustive checking of a finite
model. The cheap version that is still honest: a transition table listing every state-changing
operation beside the invariant it must preserve, and a conformance run per generation.

## Where the apparatus collapses

**Decision: this repository** - the boundary at declarative records under a fixed interpreter, and
the reduction to ordinary versioning. No capture discusses reflection; this is why
`research-575cb198.md` cannot be read as the source of this section despite the name the brief gave
it.

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

**Decision: this repository** - the three changes that defeat record validation (interpreter,
authorisation semantics, the manifest language itself), and the "new policy approved versus old
policy preserved" distinction.

If an authorised update can change the **interpreter**, the **authorisation semantics**, or the
**meaning of the manifest language**, record-shape validation is not enough. Then you need either a
semantics-preservation or refinement obligation between interpreter versions, or an explicit
declaration that the policy changed.

The distinction to enforce in every upgrade: _new policy approved_ versus _old policy preserved_.
Conflating them is how a rewrite gets described as a refactor.

## What gets built

**Decision: this repository** - all four items. Nothing in the captures proposes a `reflect`
operation, a generation-scoped conformance run, a transition table or the semantics-change refusal.

1. A `reflect` operation that returns the enforced manifest **and its generation id**, answered from
   the composition root rather than from the mutable store.
2. A generation-scoped conformance run: the contract test suite executes against the generation, and
   its result is recorded with that id.
3. A transition table in the runtime package listing each state-changing operation beside the
   invariant it preserves.
4. A refusal: a mutation that would change interpreter semantics is rejected unless the caller
   states which policy changed, so the audit trail records the difference between preservation and
   replacement.

Bisimulation is used where it belongs - comparing executions across two implementations of the same
port, or across two interpreter versions. Congruence is the obligation when claiming those
comparisons survive inspection and mutation contexts. Neither is needed to say "this generation
enforces what it reports".

**Source:** `.hermes/research/rust-4ffde6d5.md` §2 - the acceptance suite the capture requires
before the TypeScript sidecar is removed (attach/launch, navigation races, evaluation timeout,
disconnect, disposal, shared-browser ownership) is one such comparison, named concretely rather than
gestured at. **Decision: this repository** - the bisimulation and congruence framing, and the claim
that neither is needed for enforcement coherence.

# Design

What this workspace is for, and the decisions that shape it. Written after a research pass over the
real inventory of the consuming repository, so the boundaries here are argued from that surface
rather than from a taxonomy.

Provenance convention: each section below opens with **Source:** naming the capture in
`.hermes/research/` that states the claim, or **Decision: this repository** where no capture does.
`Documentation/Research.md` indexes the captures.

## The problem

**Decision: this repository** - the 125 module / 13 skill / 17 script inventory, and the four
capabilities it implies, are measurements of the consuming repository. No capture records them.
`e9b61659.md` is a research pass over one subsystem of that repository, which supports the premise
in the opening paragraph rather than the numbers.

A repository of 125 TypeScript modules, 13 documentation skills and 17 reference scripts has
capabilities that are useful outside it - browser control, chat research, design measurement, skill
validation - but each is entangled with the product it was written for. The goal is to lift those
capabilities out without lifting the product with them.

## The four layers

**Source:** `.hermes/research/4ffde6d5.md` - the layer table, "the arrows point inward", "a
capability importing an adapter is a defect" and "ports are owned by their consumers, not by a
central crate" are stated there, and §1 answers the Cargo consequence: no shared ports crate, two
consumer-owned `Browse` traits. **Decision: this repository** - the reason given for the defect (a
capability untestable without its provider) and the rule that a shared contract is extracted only
when two consumers genuinely share semantics and evolution policy.

| Layer            | Owns                                       | May import                   |
| ---------------- | ------------------------------------------ | ---------------------------- |
| Capability       | a use case and the port it needs           | its own port and data shapes |
| Adapter          | one port against one mechanism or provider | that port, that provider     |
| Workflow         | a composition of capabilities              | capabilities only            |
| Composition root | the binding of ports to adapters           | everything                   |

The arrows point inward. Nothing imports a root, and a capability that imports an adapter is a
defect: it makes the capability untestable without the provider, which is the property this
workspace exists to keep.

**Ports belong to their consumers, not to a centre.** A shared ports package becomes the dependency
magnet at forty modules - one workflow's requirement then forces edits through unrelated modules.
Extract a shared contract only when two consumers genuinely share semantics and evolution policy.

**A port states more than data shapes.** Domain errors, ordering, pagination, cancellation,
idempotency, consistency and resource ownership belong in the port, or two adapters type-check and
still behave differently.

## The manifest

**Source:** `.hermes/research/4ffde6d5.md` - the four state classes and their admission role, the
hashed `artefact` entry point and the cold/warm `cost` are stated there, and §3 answers how they
survive into Rust: a checked-in `module.json`, serde validation in CI, read by a harness that never
starts the application. **Decision: this repository** - the observation each class implies: a
liveness probe for `holds-transport`, a freshness check for `holds-identity`, a reset proof for
`holds-workspace`. The capture says what a harness must revalidate at admission, not which
observation proves it.

Every package carries `module.json`. The field that does the most work is `state`:

- `stateless` - nothing to check at admission.
- `holds-transport` - a liveness probe, because an open socket can be dead.
- `holds-identity` - a freshness check, because a token expires while pooled.
- `holds-workspace` - a reset proof, because a dirty workspace answers wrongly.

`artefact` carries a hashed entry point, so a harness loads a built file and never builds at launch.
`cost` carries cold and warm figures, so a pre-warm planner has something to budget against rather
than guessing.

## The lifecycle

**Decision: this repository** - the seven-step order, and the claim that preparation and admission
are different acts. The captures supply the vocabulary (admission, leases, budgets, cancellation,
cleanup) but not this sequence. `e9b61659.md` supplies the failure the paragraph below is about: a
completed click accepted as a write (`:205-238`), and an article's existence accepted as readiness
(`:107-132`).

prepare reusable infrastructure → lease an isolated context → bind identity and task → validate
credentials, connections and versions → execute within a fresh budget → persist only authorised
artefacts → destroy or prove a reset.

Preparation improves latency. Admission establishes correctness. Merging them is how a warm pool
starts returning wrong answers quickly.

## Why the packages are these five

**Source:** `.hermes/research/4ffde6d5.md` - the five packages with their kinds, state classes and
errors, and §1's decision that all five stay separate crates rather than becoming modules of one.
**Decision: this repository** - the build order, and each reason for building that package when it
is built. The capture says which crates exist, not which one to write first.

Chosen in build order, each justified by a consumer that already exists:

1. **browser-cdp** - the only execution substrate shared by chat research, design verification,
   scraping and browser maintenance. Build it first because everything else needs it.
2. **chat-driver** - a real reusable behaviour with identifiable failure semantics: turn
   correlation, mode and model verification, completion deadlines, trace extraction.
3. **design-verification** - separates a measurement engine from the particular design being
   measured.
4. **skill-contract** - gives a large skill corpus one validation path without pretending every
   skill is executable infrastructure.
5. **runtime** - admission, leases, budgets, cancellation, cleanup. Build it last, after the four
   above expose real lifecycle needs; otherwise it is a harness designed around imagined
   commonality.

## What is deliberately not here

**Decision: this repository** - the exclusion list. No capture discusses store, model, command,
vote, ingest, board synchronisation or gateway maintenance as a capability domain; the words appear
only in other senses (a probabilistic model, a cost model, a CLI command).

Domain policy and write authority stay with their consumers until a second workflow demonstrates a
stable contract: store, model, command, vote, ingest, board synchronisation, gateway maintenance. A
generic name is not a port, and a package called `model` or `context` becomes foundational by
default unless somebody refuses it.

## The two failures this design expects

**Source:** `.hermes/research/4ffde6d5.md` §2 for the ownership half -
`Ownership::{Attached, Launched}`, attach and launch as separate operations, disposal of an attached
session that must not terminate the browser, and a disconnected session that invalidates warm
readiness. **Decision: this repository** - the five-surface inventory, the global warm browser
scenario, and the extraction failure. No capture examines either.

**Competing browser owners.** Five surfaces already touch browser execution with different ownership
needs - a driver, a verifier, a scraper, a relaunch tool, an audit. A global warm browser can close
a session someone else owns, reuse the wrong account, or couple an isolated audit to interactive
state. The lease model has to make ownership explicit before any of it is shared.

**Source-relative assumptions surviving extraction.** Bundling into `Target/` does not make
`Reference/`, routes, tokens, configuration or sibling paths portable. The test is installing packed
artefacts into a fixture with the original repository unavailable.

## The warning metric

**Decision: this repository** - the metric itself. Unrelated coordination per change is this
document's proposal; nothing in the captures measures it or names it.

After two years the bad version of this design has hundreds of package files, duplicated manifests,
permissive graph exceptions, warm contexts nobody can safely evict, and consumers pinned to matching
release trains. The useful signal is not package count: it is **unrelated coordination per
change** - how many packages must be edited together for one workflow's requirement.

# Design

The decisions that shape the workspace: what each layer may know, what a manifest states, and where
the boundary between a capability and a product sits.

## The four layers

| Layer            | Owns                                       | May import                   |
| ---------------- | ------------------------------------------ | ---------------------------- |
| Capability       | a use case and the port it needs           | its own port and data shapes |
| Adapter          | one port against one mechanism or provider | that port, that provider     |
| Workflow         | a composition of capabilities              | capabilities only            |
| Composition root | the binding of ports to adapters           | everything                   |

The arrows point inward. Nothing imports a root, and a capability that imports an adapter is a
defect: it makes the capability untestable without the provider, which is the property this
workspace exists to keep.

**Ports belong to their consumers, not to a centre.** A shared ports crate becomes the dependency
magnet at forty modules: one workflow's requirement then forces edits through unrelated modules. A
shared contract is extracted only when two consumers genuinely share semantics and evolution policy.

**A port states more than data shapes.** Domain errors, ordering, pagination, cancellation,
idempotency, consistency and resource ownership belong in the port, or two adapters type-check and
still behave differently.

## The split

Eleven crates, one role each:

- **Capabilities** - `agito-chat-driver`, `agito-design-verification`, `agito-skill-contract`,
  `agito-lane`, `agito-spacetime-catalog`. Each declares the port it needs and imports no adapter.
- **The substrate** - `agito-browser-cdp`, the one crate that talks to Chromium.
- **The adapter port** - `agito-adapter`, the shape a substrate this workspace did not write
  implements. It ships no behaviour and no substrate of its own.
- **The workflow** - `agito-workflow`, the bindings that translate a capability's own port onto the
  substrate, and one composition over four capabilities.
- **The runtime** - `agito-runtime`, which every layer above may use and which names no substrate.
- **The facade** - `agito-sdk`, the six crates a consumer would otherwise name.
- **The root** - `agito`, the wiring, the manifest table and the binary.

The order the crates were written in is not the order they matter in. The substrate came first
because every capability needs it; the runtime came after the capabilities exposed the lifecycle they
actually need, because a runtime designed before that is a harness built around imagined commonality.

## The manifest

Every crate carries `module.json`. The field that does the most work is `state`, because it decides
what may be reused and what has to be revalidated at admission:

- `Stateless` - nothing to check at admission.
- `HoldTransport` - a liveness probe, because an open socket can be dead.
- `HoldIdentity` - a freshness check, because a token expires while pooled.
- `HoldWorkspace` - a reset proof, because a dirty workspace answers wrongly.

`artefact` carries the entry point a harness loads, so nothing builds at launch. `cost` carries cold
and warm figures, so a pre-warm planner budgets against a number rather than a guess. `agito verify`
compares the error codes a manifest names against the codes its crate defines, which is what keeps
the two from drifting.

## The lifecycle

prepare reusable infrastructure → lease an isolated context → bind identity and task → validate
credentials, connections and versions → execute within a fresh budget → persist only authorised
artefacts → destroy or prove a reset.

Preparation improves latency. Admission establishes correctness. Merging them is how a warm pool
starts returning wrong answers quickly.

## The boundary

Domain policy and write authority stay with the consuming product: the workspace ships no store, no
model, no command, no vote, no ingest and no board synchronisation. A generic name is not a port, and
a crate called `model` or `context` becomes foundational by default unless somebody refuses it.

## The two failures the boundary guards against

**Competing browser owners.** A consuming product can have several surfaces touching browser
execution with different ownership needs - a driver, a verifier, a scraper, a relaunch tool, an
audit. A global warm browser can close a session somebody else owns, reuse the wrong account, or
couple an isolated audit to interactive state, which is why a session carries its ownership and why
disposal closes only a browser it launched.

**Source-relative assumptions surviving extraction.** Bundling into `Target/` does not make
`Reference/`, routes, tokens, configuration or sibling paths portable. A path is resolved relative to
the crate, then from an environment variable, then from one installed location, in that order - so a
tree that moves still answers, and a tracked file names no machine.

## The signal to watch

Package count is not the useful number. The useful signal is **unrelated coordination per change**:
how many crates must be edited together for one workflow's requirement. A design that keeps that
number at one is a design whose boundaries hold, whatever its file count.

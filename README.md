# Agentic

Capabilities, adapters and the runtime that leases them. Nothing here knows about
a particular product; the product consumes this workspace and supplies its own
composition root.

## The four layers

| Layer | Owns | May import |
| --- | --- | --- |
| Capability | a use case and the port it needs | its own port and data shapes |
| Adapter | one port against one mechanism or provider | that port, that provider |
| Workflow | a composition of capabilities | capabilities only |
| Composition root | the binding of ports to adapters | everything |

The arrows point inward and nothing imports a root. A capability that imports an
adapter is a defect rather than a shortcut: it makes the capability untestable
without the provider, which is the one property this workspace exists to keep.

## The manifest

Every package carries `module.json`. The field that does the most work is `state`,
because it decides what a harness may warm and what it must revalidate at admission:

- `stateless` - nothing to check.
- `holds-transport` - a liveness probe, because an open socket can be dead.
- `holds-identity` - a freshness check, because a token expires while pooled.
- `holds-workspace` - a reset proof, because a dirty workspace answers wrongly.

## The lifecycle

prepare reusable infrastructure → lease an isolated context → bind identity and
task → validate credentials, connections and versions → execute within a fresh
budget → persist only authorised artefacts → destroy or prove a reset.

Preparation improves latency. Admission establishes correctness. They are different
acts, and merging them is how a warm pool starts returning wrong answers quickly.

## The packages

| Package | Kind | Why it exists |
| --- | --- | --- |
| `@agentic/browser-cdp` | adapter | the shared execution substrate: attach versus launch, bounded evaluation, disposal |
| `@agentic/chat-driver` | capability | ask a chat product, correlate turns, verify mode and model, read answer and trace |
| `@agentic/design-verification` | capability | measure a built page and produce evidence; expectations stay with the site |
| `@agentic/skill-contract` | capability | discover, validate and package a skill corpus |
| `@agentic/runtime` | runtime | admission, leases, budgets, cancellation, cleanup, telemetry |

## Deliberately not here

Domain policy and write authority stay with their consumers until a second workflow
demonstrates a stable contract: store, model, command, vote, ingest, board
synchronisation, gateway maintenance. A generic name is not a port.

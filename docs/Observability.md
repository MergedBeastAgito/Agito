# Observability: the database is the surface

An earlier pass argued that SpacetimeDB is a poor fit for control flow and that
orchestration belongs in Rust. That was too broad, and this document records the
correction because it changes the architecture.

## The corrected boundary

The line is not control flow versus data. It is **effects versus state**.

| Lives in SpacetimeDB | Lives in Rust |
| --- | --- |
| workflow transitions between phases | the browser connection and its socket |
| admission decisions | DOM evaluation and input dispatch |
| leases, fences, expiry | process and filesystem operations |
| budgets and their consumption | the work an effect actually performs |
| cancellation requests | honouring a cancellation mid-operation |
| operator commands | the binary's own supervision |

Rust does not need a second authoritative workflow engine whose decisions are copied
into the database afterwards. That copy is a second state machine, and the
reconciliation between the two is the bug you spend the next year fixing.

## Why a reducer is the right primitive

A reducer is one transaction. That means a single call can:

1. move an invocation into a new phase,
2. reserve its budget,
3. grant its resource lease,
4. record the transition and its cause,
5. and make the resulting state visible to every subscriber,

and if any step fails, none of it happened. A log line cannot do the fourth step, and
an out-of-band write cannot do the fifth. This is the difference between a dashboard
that describes the system and a dashboard that *is* the system's state.

## What this buys over OTEL

- **Subscriptions are a live view, not a query loop.** A panel sees a change because
  it was pushed, not because it polled.
- **The record and the state are the same bytes.** There is no window in which the
  trace says one thing and the state another.
- **Replay is re-reading.** A generation's rows are what happened; a subscription
  replayed from the start is the history.
- **Atomicity is the transaction, not a convention.** A partial transition is not a
  thing that can be recorded.

## What OTEL still does better

- **Cross-process and cross-vendor traces.** A database only observes what writes to
  it; a process that dies before its first write leaves no row. OTEL's collector sees
  the process, the crash and the resource pressure around it.
- **Cardinality at volume.** Metrics with unbounded label sets are cheap to emit and
  expensive to store as rows.
- **Ecosystem.** Existing dashboards, alerting and retention policies.

The honest position: SpacetimeDB for the state machine and the operational surface;
OTEL for what happens *around* the process, if it is wanted at all.

## The dashboard

One surface, exported from inside the binary, able to attach to any deployment:

- **Transport**: a subscription over the same database the workers write to. Not a
  second API, because a second API is a second truth.
- **Identity**: the panel connects as a client with its own identity and a role that
  grants read on the operational tables and no write. The role is data, so it can be
  revoked without a redeploy.
- **Federation**, in ascending order of effort: one shared control database for every
  enrolled binary; several panel connections each labelled with its environment; or a
  designed replication layer. SpacetimeDB's inter-database communication is
  outbox-based - a message row delivered as a reducer call - which is evidence for
  that pattern, not proof of global transactions across databases.
- **Staleness is visible.** A disconnected panel or a worker whose heartbeat has
  expired must render as unknown, never as healthy-because-of-its-last-row. The
  cheapest way to guarantee that is a heartbeat table plus a view that computes
  freshness from the database's own clock.

## What this rules out

- A second scheduler in Rust that decides what the database then records.
- A dashboard that polls a REST projection of the tables.
- Any state that exists only in a process and is "synced later".

# Database

The rules the database enforces, as states. Everything here is an internality: a reducer moves it,
and a subscriber sees the move.

## Page 1 - a generation

```mermaid
stateDiagram-v2
	[*] --> Unbound
	Unbound --> Bound: bind(manifest digest)
	Bound --> Superseded: bind(a newer digest)
	Unbound --> Refused: a digest naming an undeclared capability
	Superseded --> [*]
	Refused --> [*]
	note right of Unbound
		legend: effect = touches the world (Rust) · internality = a record (database) · ticket = lease + fence + generation · fence = grant number, stale is refused · generation = one bound manifest · refused = a result, never a panic · missing = a state the code does not have
	end note
```

A generation is appended, never updated. The rule that makes it useful: a write under a generation
that is not the latest bound one is refused, and the refusal names the generation it saw.

---

## Page 2 - a lease, and the fence

```mermaid
stateDiagram-v2
	[*] --> Free
	Free --> Held: claim(resource, ttl)
	Held --> Held: renew(current fence)
	Held --> Free: release
	Held --> Expired: the module's clock passes expiry
	Expired --> Held: claim again, fence + 1
	Free --> Refused: claim while another holds it
	note right of Free
		legend: effect = touches the world (Rust) · internality = a record (database) · ticket = lease + fence + generation · fence = grant number, stale is refused · generation = one bound manifest · refused = a result, never a panic · missing = a state the code does not have
	end note
```

The fence increments on every grant. A write carrying an old fence is refused - not queued, not
applied late. `Refused` is a result, never a panic.

---

## Page 3 - a probe, and why a receipt is not a disclosure

```mermaid
sequenceDiagram
	participant A as Actor
	participant R as Reducer
	participant T as private truth
	participant O as observation
	A->>R: probe(target, action)
	R->>R: check the actor's entitlement
	R->>T: read the value
	R->>O: write one audience-scoped row
	R->>R: charge the cost, in the same transaction
	R-->>A: only what was written
	Note over T,O: the private value is never a column a reader can select
	Note over [*]: legend: effect = touches the world (Rust) · internality = a record (database) · ticket = lease + fence + generation · fence = grant number, stale is refused · generation = one bound manifest · refused = a result, never a panic · missing = a state the code does not have
```

The review's point stands: a receipt is evidence that a probe happened, not a mechanism that
prevented a leak. The leak is prevented by the private value never being selectable, and by the
observation being tied to a property version so an old grant cannot silently authorise a future
change.

---

## Page 4 - missing states

```mermaid
stateDiagram-v2
	Held --> Abandoned: the holder dies without releasing
	Abandoned --> Expired: the clock, eventually
	Expired --> Quarantined: the resource may be dirty
	Quarantined --> Free: a reset is proven
	note right of Held
		legend: effect = touches the world (Rust) · internality = a record (database) · ticket = lease + fence + generation · fence = grant number, stale is refused · generation = one bound manifest · refused = a result, never a panic · missing = a state the code does not have
	end note
```

`Abandoned` and `Quarantined` do not exist. Expiry covers the clock; nothing covers the workspace a
dead holder may have left dirty, which is the `holds-workspace` class the manifest names and the
schema does not yet model.

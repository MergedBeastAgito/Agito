# Runtime

The lease and effect lifecycle, as it is and as it should be. Pages are separated by a rule; the
last page is the one with states nobody has built.

## Page 1 - the lease as it exists today

```mermaid
stateDiagram-v2
	[*] --> Free
	Free --> Claimed: claim(resource, ttl)
	Claimed --> Claimed: renew(fence)
	Claimed --> Admitted: with_lease(lease, effect)
	Admitted --> Landed: effect completes
	Admitted --> Refused: ticket stale at the destination
	Claimed --> Expired: clock passes expires_at
	Expired --> Free: release
	Landed --> Free: release
	Refused --> Free: release
	Free --> [*]
```

`Refused` is the state the fence card added. It exists in the port and is exercised by tests;
whether it exists at a _destination_ is the open question.

---

## Page 2 - the late arrival, which is the counterexample

```mermaid
sequenceDiagram
	participant W as Worker
	participant R as Runtime
	participant D as Destination
	W->>R: claim(resource, 30s)
	R-->>W: lease fenced at 7
	W->>R: with_lease(lease, effect)
	R->>R: check passes, effect admitted
	Note over W,R: the lease expires here
	W->>D: present ticket (fence 7)
	alt destination checks its own record
		D-->>W: refused, stale fence
	else destination trusts the ticket
		D-->>W: accepted, and the effect landed
	end
```

The second branch is what "fencing the caller" means. A write that landed is not retracted because
the record of it was refused.

---

## Page 3 - missing states

```mermaid
stateDiagram-v2
	[*] --> Free
	Free --> Claimed
	Claimed --> Dispatched: effect started
	Dispatched --> Ambiguous: lease expired mid-flight
	Ambiguous --> Reconciled: the destination records what landed
	Ambiguous --> Quarantined: the resource is held until a human rules
	Reconciled --> Free
	Quarantined --> Free
```

None of `Dispatched`, `Ambiguous`, `Reconciled` or `Quarantined` exists. They are the handover rule
the review demanded: what happens to work already in flight when its authority ends. `Refused` alone
cannot cover it, because refusing a result says nothing about the effect that already happened.

---

## Page 4 - who owns which state

```mermaid
flowchart LR
	subgraph Process["In the process"]
		P1["the live transport"]
		P2["the effect itself"]
	end
	subgraph Database["In the database"]
		Q1["lease, fence, generation"]
		Q2["phase and handover record"]
		Q3["the trace of both"]
	end
	P2 --> Q3
	Q1 --> P1
	Q2 --> P1
```

The left column is effects. The right column is internalities. `Dispatched` and `Ambiguous` belong
on the right, which is why the crate cannot be the authority for them.

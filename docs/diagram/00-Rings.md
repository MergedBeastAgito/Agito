# Rings

The crates as concentric authority: what may know what, drawn as rings rather than as a dependency
list. Each page below is separated by a rule; later pages are the intended shape, not the current
one.

## Page 1 - the rings as they are intended

```mermaid
flowchart TD
	subgraph Ring3["Ring 3 - composition"]
		A["agentic<br/>names concrete adapters"]
	end
	subgraph Ring2["Ring 2 - workflows"]
		W["agentic-workflow<br/>owns the Browse to Page bridge"]
	end
	subgraph Ring1["Ring 1 - capabilities"]
		C["chat-driver"]
		D["design-verification"]
		S["skill-contract"]
	end
	subgraph Ring0["Ring 0 - substrate and policy"]
		B["browser-cdp<br/>effects"]
		R["runtime<br/>admission and leases"]
		P["spacetime-catalog<br/>the rules the database enforces"]
	end

	A --> W
	A --> C
	A --> D
	A --> S
	A --> B
	A --> R
	W --> C
	W --> D
	W --> S
	C -. "declares its own Page port" .-> C
	D -. "declares its own port" .-> D
```

The dotted self-edges are the point: a capability declares what it needs and imports nobody.
`browser-cdp` is the only crate in Ring 0 that touches the outside world.

---

## Page 2 - what the rings are for

```mermaid
flowchart LR
	subgraph Knowledge["Who may know"]
		K1["capability: its own port and shapes"]
		K2["adapter: one port and one provider"]
		K3["workflow: capabilities only"]
		K4["root: everything, once"]
	end
	K1 --> K2 --> K3 --> K4
```

A crate that knows more than its ring is a defect even when it compiles, which is why
`Scripts/Boundaries.mjs` exists and why `cargo tree -p agentic-chat-driver | grep -c browser` must
answer `0`.

---

## Page 3 - the ring that is missing

```mermaid
flowchart TD
	DB[("SpacetimeDB<br/>authority: phases, leases, generations, records")]
	E["Rust effect runner<br/>no lease state of its own"]
	X["missing: the destination<br/>refuses a stale ticket where the effect lands"]
	DB -- "issues a ticket" --> E
	E -- "presents the ticket" --> X
	X -. "refuses, or records the result" .-> DB
```

`X` does not exist yet. Until it does, the runtime crate is a second authority wearing the same
vocabulary - see `docs/Review.md` and the card that names this gap.

---

## Page 4 - the ring that should disappear

```mermaid
flowchart TD
	subgraph Today["Today"]
		RT["runtime: holds lease state in the process"]
	end
	subgraph Intended["Intended"]
		DB2[("the database holds the lease")]
	end
	RT -. "should not exist as an authority" .-> DB2
```

The lease, the fence and the generation are internalities. A process that owns them is a parallel
system, however correct its fencing.

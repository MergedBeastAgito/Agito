# Browser

The CDP adapter's lifecycle: attach versus launch, and the two things that can be true of a session
that is not alive.

## Page 1 - attach or launch

```mermaid
stateDiagram-v2
	[*] --> Unbound
	Unbound --> Launched: open(url) with no endpoint
	Unbound --> Attached: open(url) with attachTo
	Launched --> Live: debug socket found
	Attached --> Live: /json/version answers
	Unbound --> Unavailable: no executable, or no endpoint
	Live --> Evaluated: evaluate(expression)
	Evaluated --> Live
	Live --> Disposed: dispose()
	Attached --> Disposed: dispose() only disconnects
	Launched --> Disposed: dispose() closes the process
	Disposed --> [*]
	Unavailable --> [*]
	note right of Unbound
		legend: effect = touches the world (Rust) · internality = a record (database) · ticket = lease + fence + generation · fence = grant number, stale is refused · generation = one bound manifest · refused = a result, never a panic · missing = a state the code does not have
	end note
```

The distinction that must survive every refactor: `dispose` on a launched session may close the
browser; on an attached session it must not, because somebody else owns it.

---

## Page 2 - the three failures, and the one that hides

```mermaid
flowchart TD
	Op["an operation is issued"] --> Q{"what failed?"}
	Q -- "no socket, no executable" --> U["browser-unavailable"]
	Q -- "navigation rejected" --> N["navigation-failed"]
	Q -- "deadline or caller abort" --> T["timeout"]
	Q -- "socket open, nothing answers" --> D["looks alive<br/>no state reports it"]
	D -. "the gap" .-> U
	subgraph Legend["legend: effect = touches the world (Rust) · internality = a record (database) · ticket = lease + fence + generation · fence = grant number, stale is refused · generation = one bound manifest · refused = a result, never a panic · missing = a state the code does not have"]
	end
```

`D` is the state the adapter cannot currently name. An open socket is not a live browser, and a
liveness probe is the only thing that distinguishes them - which is why the manifest gives this
crate the `holds-transport` state class and an admission check.

---

## Page 3 - identity, which the review says is conflated

```mermaid
flowchart LR
	URL["a URL"] --> T1["target: page handle"]
	EP["an endpoint"] --> T2["browser handle"]
	T1 --> S["session"]
	T2 --> S
	S --> O{"ownership?"}
	O -- "launched" --> L["may close"]
	O -- "attached" --> A["may only disconnect"]
	U2["the user's browser<br/>same endpoint, different identity"] -. "collides with" .-> T2
	subgraph Legend["legend: effect = touches the world (Rust) · internality = a record (database) · ticket = lease + fence + generation · fence = grant number, stale is refused · generation = one bound manifest · refused = a result, never a panic · missing = a state the code does not have"]
	end
```

Three things are called "the browser" here: an endpoint, a page, and a user. The review flagged the
collision; the fix is that a `Target` carries its ownership, and that an endpoint is never treated
as an identity.

---

## Page 4 - missing states

```mermaid
stateDiagram-v2
	Live --> Stalled: socket open, no reply within probe window
	Stalled --> Live: probe answers
	Stalled --> Unavailable: probe times out
	Live --> Reclaimed: another client attached to the same page
	Reclaimed --> [*]
	note right of Live
		legend: effect = touches the world (Rust) · internality = a record (database) · ticket = lease + fence + generation · fence = grant number, stale is refused · generation = one bound manifest · refused = a result, never a panic · missing = a state the code does not have
	end note
```

`Stalled` and `Reclaimed` do not exist. Both are real: the first is the dead-but-open socket, the
second is two debugger clients on one page - which is exactly what happens when a second tool
attaches to a tab the extension already holds.

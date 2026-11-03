# Feed

The tools that carry a corpus into a chat product: bundle, attach, observe. They are not part of the
agent; they are how the agent reads a distant model.

## Page 1 - the feed

```mermaid
stateDiagram-v2
	[*] --> Collected: bundle.mjs walks the tree
	Collected --> Batched: deterministic split, 30 files or 200 KB
	Collected --> LeftOut: too large, or excluded
	Batched --> Attached: attach.mjs sets the hidden input
	Attached --> Confirmed: the page shows the name back
	Attached --> Refused: the input does not accept the extension
	Confirmed --> Prompted: cdp-send, mode asserted
	Prompted --> Watching: observe.mjs
	Watching --> Settled: quiet for N reads after a change
	Watching --> TimedOut: the deadline
	Settled --> Batched: next batch
	Settled --> [*]
	note right of Collected
		legend: effect = touches the world (Rust) · internality = a record (database) · ticket = lease + fence + generation · fence = grant number, stale is refused · generation = one bound manifest · refused = a result, never a panic · missing = a state the code does not have
	end note
```

The gate that matters: `Confirmed` is reached only when the page itself shows the file name. A
successful `setFileInputFiles` call is not evidence that the product accepted the file.

---

## Page 2 - why the batches must be deterministic

```mermaid
flowchart LR
	T["the same tree"] --> B1["batch 1, run A"]
	T --> B2["batch 1, run B"]
	B1 --> C{"byte-identical?"}
	B2 --> C
	C -- "yes" --> OK["a batch can be compared between runs"]
	C -- "no" --> X["a batch is a one-off nobody can diff"]
	subgraph Legend["legend: effect = touches the world (Rust) · internality = a record (database) · ticket = lease + fence + generation · fence = grant number, stale is refused · generation = one bound manifest · refused = a result, never a panic · missing = a state the code does not have"]
	end
```

`Scripts/Feed.mjs` asserts exactly this, because a feed whose batches differ between runs cannot be
reviewed - only re-read.

---

## Page 3 - the exclusion bug, as a state

```mermaid
stateDiagram-v2
	[*] --> Walking
	Walking --> Excluded: a directory matches, case-insensitively
	Walking --> Included: a file matches the extension
	Included --> Refused: larger than the cap
	Excluded --> [*]
	Refused --> [*]
	note right of Walking
		legend: effect = touches the world (Rust) · internality = a record (database) · ticket = lease + fence + generation · fence = grant number, stale is refused · generation = one bound manifest · refused = a result, never a panic · missing = a state the code does not have
	end note
```

The first version compared case-sensitively, so `Target/` walked past an exclude of `target` and
three files of build scratch were attached as a batch. On a case-sensitive volume those are
different directories, and the fix belongs in the walker, not in the prompt.

---

## Page 4 - missing states

```mermaid
stateDiagram-v2
	Prompted --> RateLimited: the product refuses more uploads
	RateLimited --> ContextOnly: prompt without files
	ContextOnly --> Settled
	note right of Prompted
		legend: effect = touches the world (Rust) · internality = a record (database) · ticket = lease + fence + generation · fence = grant number, stale is refused · generation = one bound manifest · refused = a result, never a panic · missing = a state the code does not have
	end note
```

`RateLimited` is real and unmodelled: a weekly upload quota turns a feed into a text-only
conversation, and the tool has no way to detect it other than by reading the product's own error.
Until it is modelled, the fallback is a person saying so.

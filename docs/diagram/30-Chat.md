# Chat

Asking a chat product a question, and the three preconditions that must be read before anything is
sent.

## Page 1 - the ask

```mermaid
stateDiagram-v2
	[*] --> Idle
	Idle --> ModeChecked: read the composer's toggle
	ModeChecked --> ModeFixed: press Search, re-read
	ModeChecked --> Refused: wrong-mode, nothing sent
	ModeFixed --> ModelChecked: read the model label
	ModelChecked --> Refused: wrong-model, nothing sent
	ModelChecked --> Sent: type, submit, composer empties
	Sent --> Streaming: answer count exceeds the baseline
	Streaming --> Settled: text unchanged for three reads
	Settled --> Truncated: a Read more control sits over the answer
	Settled --> Complete
	Streaming --> Refused: deadline hit, partial text not returned
	Complete --> [*]
	Truncated --> [*]
	Refused --> [*]
```

`Refused` is reached from four different places and means four different things. The manifest gives
it four codes for that reason: `not-ready`, `wrong-mode`, `wrong-model`, `truncated`.

---

## Page 2 - completion, which is a rule and not a sleep

```mermaid
flowchart TD
	B["baseline: how many answers exist now"] --> S["submit"]
	S --> W{"answer count > baseline?"}
	W -- "no" --> W2["wait, check deadline"] --> W
	W -- "yes" --> T{"text unchanged 3 reads?"}
	T -- "no" --> T2["keep reading"] --> T
	T -- "yes" --> C{"Read more over it?"}
	C -- "yes" --> X["truncated"]
	C -- "no" --> OK["complete"]
	D["deadline passed"] --> R["not-ready, no partial text"]
```

The three-read rule proves text stability, not that generation finished. That is the review's
qualification, and it is why `truncated` is a separate outcome rather than a retry.

---

## Page 3 - the trace, which is not the answer

```mermaid
flowchart LR
	A["answer text"] --> R1["the record"]
	T["step tree"] --> R1
	S["cited sources"] --> R1
	M["mode and model"] --> R1
	N["nothing else"] -.-> R1
```

The trace is how the answer was reached and it is not in the answer text. A capture that keeps only
prose has lost the evidence, which is why the extractor reads the tree separately from the
transcript.

---

## Page 4 - missing states

```mermaid
stateDiagram-v2
	Sent --> Queued: the product queues the question
	Queued --> Streaming: after a wait nobody reports
	Streaming --> Interrupted: the product stops mid-answer
	Interrupted --> Settled: resumed
	Interrupted --> Refused: never resumed
```

`Queued` and `Interrupted` are real and unmodelled. The first is why a fixed sleep looks necessary;
the second is why "text stable for three reads" can settle on a truncated answer that nobody marked
as truncated.

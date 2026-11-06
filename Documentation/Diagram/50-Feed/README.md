# Feed

The tools that carry a corpus into a chat product: bundle, attach, observe. They are not part of the
agent; they are how the agent reads a distant model.

## Page 1 - the feed

![Collected, batched, attached, confirmed, prompted, watching and settled, with the refusals](lifecycle.svg)

[`lifecycle.mmd`](lifecycle.mmd) · [`lifecycle.svg`](lifecycle.svg)

The gate that matters: `Confirmed` is reached only when the page itself shows the file name. A
successful `setFileInputFiles` call is not evidence that the product accepted the file.

---

## Page 2 - why the batches must be deterministic

![One tree, two runs, and the byte comparison that decides whether a batch can be reviewed](determinism.svg)

[`determinism.mmd`](determinism.mmd) · [`determinism.svg`](determinism.svg)

`Script/Feed.mjs` used to assert exactly this, and it was deleted with the Node toolchain: the
bundler is local tooling under `.hermes/`, so nothing in the tree holds its determinism now. A feed
whose batches differ between runs cannot be reviewed - only re-read - and
`Documentation/Open-Questions.md` records the gate as owed.

---

## Page 3 - the exclusion bug, as a state

![Walking, excluded, included, and a file refused for its size](exclusion.svg)

[`exclusion.mmd`](exclusion.mmd) · [`exclusion.svg`](exclusion.svg)

The first version compared case-sensitively, so `Target/` walked past an exclude of `target` and
three files of build scratch were attached as a batch. On a case-sensitive volume those are
different directories, and the fix belongs in the walker, not in the prompt.

---

## Page 4 - missing states

![Rate limited falling back to a context-only prompt](missing.svg)

[`missing.mmd`](missing.mmd) · [`missing.svg`](missing.svg)

`RateLimited` is real and unmodelled - it is drawn as an installable step. A weekly upload quota
turns a feed into a text-only conversation, and the tool has no way to detect it other than by
reading the product's own error. Until it is modelled, the fallback is a person saying so.

---
name: chat-research
description: Use when a web chat UI must answer a research question from Rust. Ask, settle, read.
version: 1.0.0
license: MIT
platforms: [macos, linux]
metadata:
    hermes:
        tags: [cdp, rust, research, chat-ui, perplexity, chat-driver]
---

# Chat Research

Ask a web chat product a question and read the answer once the page says it has finished. The
capability is `agentic-chat-driver`; the verb is `agentic chat`.

There is no API to call. The product is a composer, a send control and a block of text, so the
driver's rules are about what a page is allowed to claim.

---

## The call

```sh
Target/debug/agentic chat \
  --url https://example.test/ \
  --model '<the label the page shows>' \
  --attach ws://127.0.0.1:<debug-port>/devtools/browser/<uuid> \
  [--bind <target-id>] \
  [--selectors <file.json>] \
  [--format markdown|text] \
  --question '<text>' \
  --timeout 300000
```

The result is `{"url", "format", "text", "model", "mode", "trace", "sources"}`. `--bind` drives the
tab a person is already looking at; without it the call opens a page of its own. Use a bind for
research that belongs in an existing thread, and a new page for a probe you intend to throw away.

---

## Two readings of the same answer

`--format` decides what `text` is, and it is the only thing it decides: the same element, the same
scroll, the same busy control, the same turn correlation.

- **`text`** is `innerText`, trimmed. It is the default because every recorded baseline was read
  with it.
- **`markdown`** is the answer's DOM serialised by the page. Tables keep their columns, `pre > code`
  becomes a fenced block with its language, a formula is its TeX from
  `annotation[encoding="application/x-tex"]` instead of hundreds of positioned glyphs, lists keep
  their nesting and a link is `[text](href)`.

Ask for `markdown` when the answer is going into a note, a report or a diff - anywhere its
_structure_ is part of what was said. `innerText` is not wrong, it is unformatted: on a thread whose
answer carries three tables it returns every cell as a tab-separated run with no pipes, and a table
whose rows do not share a tab count cannot be recovered from it at all.

The completion rule is unaffected, because it compares text to text and never length to length: a
Markdown capture is longer, settles on the same third agreement, and its turn marks are computed
from the same reading the answer came from.

---

## Four preconditions, refused rather than assumed

**The mode is a precondition.** The composer's toggle is read before anything is typed. Not `search`
means it is pressed once, read again, and then refused as `wrong-mode` - nothing was sent. One
press, not a loop: a toggle that does not move on the first press will not move on the fifth, and a
retry loop spends the caller's deadline proving it.

**The model is a precondition.** The label is compared to `--model` before anything is typed, and a
mismatch is `wrong-model`. A driver that cannot say which model answered cannot defend its output.

**Completion is a rule, not a sleep.** See below.

**A collapsed answer is `truncated`.** The control that hides the rest of an answer is looked for
inside the newest answer once it has settled, because at that point the text in the DOM is
demonstrably not all of it.

Everything else the page does that is not one of those four is `not-ready`, with the cause in the
detail. The manifest has four codes and the driver does not invent a fifth.

---

## Completion: the page's own signal, then stable text

Text stability alone false-settles. A pause between tokens, a reasoning step and a slow code block
all leave the text unchanged while the answer is demonstrably unfinished, and a driver that only
counts stable reads returns a partial answer that looks whole.

So `ReadAnswers` returns `{count, text, busy}` in one read, where `busy` is the page's own statement
that it is still producing - a visible stop or cancel control. The rule is:

- a **busy** read resets the confirmations and confirms nothing;
- otherwise three consecutive reads with no change settle the answer.

`busy` is read in the same expression as the text, not in a probe of its own: a separate read
describes a different instant, and the two disagree exactly when it matters.

**The new turn is identified by membership, not by position or count.** A long thread is
virtualised, so the oldest turn unmounts as the newest arrives and the count stops growing while the
answer is plainly there; and the last element in document order is not reliably the newest turn.
Each read carries one identifier per answer - its length in code points and its opening - so "is
this answer new" is answered by membership: an answer whose identifier was not in the thread before
the question is the only reading that survives unmounting and reordering.

A page that is **already** producing when the call begins is refused rather than typed into: the
question would land in a turn that is still being written, and the answer read back would belong to
neither.

The honest limit: `busy` false is not proof of success. A cancelled or failed turn also stops
generating. The driver's own turn identity and deadline are what distinguish those, and a caller
that needs certainty reads the trace and the text rather than trusting the settle alone.

---

## Typing is an edit, not a property write

A contenteditable editor keeps its own document model. Writing `innerText` changes the DOM and
leaves the send control disabled, because no edit happened - the symptom is
`not-ready: the send control is disabled after the question was typed`.

`TypeQuestion` therefore performs a real edit: focus, select the contents, and
`document.execCommand('insertText', …)`, falling back to a written `innerText` plus an `input` event
only when the browser refuses. A `textarea` or `input` is set through its prototype's own value
setter, so a framework tracking the value sees the change. An existing draft is replaced through the
editor as well - select, `execCommand('delete')`, then insert - because overwriting the DOM under an
editor that already holds a question leaves its model holding the old one.

**The composer is read back before the send, and a mismatch sends nothing.** `PressSubmit` compares
the composer's text with the question, normalised, and returns `mismatch` instead of clicking. An
editor can report a successful insert while keeping the previous draft; sending that asks a question
nobody wrote, and nothing in the result would say so. That is the one failure worse than sending
nothing, so it is refused as `not-ready` with the cause in the detail.

The CDP-native alternative is `Input.insertText` after focusing the editable host. It is the better
primitive for an editor that resists `execCommand`, and it is the next step this substrate does not
yet expose.

---

## Observing instead of only asking

`Ask` is `Observe` with the inside hidden. `Observe` runs the same preconditions, turn correlation
and completion rule, and pushes one `Observation` to a sink for every read it takes:

```rust
let mut Sink = |Seen:&Observation| {
    // count, text, busy, fresh, settled
    Control::Continue
};
Driver.Observe(Question, Call, &mut Sink).await
```

`Control::Stop` is a mutation observer's `disconnect()`: it ends the wait and the answer so far
comes back, because stopping is not failing. The `Call`'s cancellation is the abort from the
outside, and it is also not an error. The `--observe` flag prints one JSON line per read on stderr,
which is how a stalled or flickering completion signal becomes visible instead of being averaged
into a single result.

**A read returns to the end of the thread first.** A virtualised thread does not mount what is
off-screen, so a read taken with the page scrolled to the top measures whichever turns happen to be
mounted - and an old answer is read as a new one. The page's own scroll control is pressed first,
and scrolling the last answer into view is the fallback for a product that has none.

## Ask, then weigh

- **Over-supply, under-read.** Send the real inventory - names, paths, counts - and read only the
  part where the verdict lands. The remote model's context is cheap; yours is not.
- **A long answer is not a truncated one.** The tell is the text, not a `Read more` control, which
  is a display clamp over text already in the DOM.
- **Ask for the answer, not a format.** Impose a shape only for status you will act on mechanically.
- **An answer is a hypothesis until the line it names is read.** A chat product sees the web and
  other repositories, not your working tree or your commits.
- **Make it argue against itself.** Paste the artefact rather than a description, ask for the
  strongest case against its own advice, and ask where the design is worse. A concession with no
  downside is a sales pitch.

---

## One question per thread, and say which tab answered

A follow-up in an existing thread carries that thread's history, so it does not have to restate
facts a fresh thread would need. A second question dropped into a saturated thread comes back
truncated, because the answer budget is shared with everything already in it.

Report the mode, the model and the tab. A reader who cannot tell which of the three produced an
answer cannot weigh it.

---

## Reference

- `Reference/Index.md` - the file list for this skill.
- `Reference/Perplexity.md` - the verified selectors and DOM facts for the reference product, and
  the two places its markup disagrees with a naive reading.

---

## Claim → test

| Claim                                            | Test                                                                                                                                                                                                              |
| ------------------------------------------------ | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| A question is answered and read back             | `agentic chat --attach <ws> --model '<label>' --question 'Reply with exactly: ok'` returns `text` containing `ok`                                                                                                 |
| The mode is enforced, not assumed                | a page whose toggle reads `computer` and will not switch exits non-zero as `wrong-mode` and sends nothing                                                                                                         |
| The model is compared before typing              | `--model 'not-the-page-label'` exits non-zero as `wrong-model` and sends nothing                                                                                                                                  |
| A busy page does not settle                      | ask a long question and read at ~10 s: the process is still running while the stop control is visible                                                                                                             |
| Typing produces a real edit                      | after a failed send, the composer holds the question text and the send control is enabled                                                                                                                         |
| A settled answer is the answer, not the question | the returned `text` is not equal to `--question`                                                                                                                                                                  |
| A bind writes into the tab, not a new one        | `/json/list` target count is unchanged across a `--bind` call                                                                                                                                                     |
| A research loop keeps context                    | a follow-up in the same tab answers using the earlier turn without restating it                                                                                                                                   |
| A virtualised thread still settles               | ask a sixth question in a thread long enough that the count has stopped growing: the call returns, and its text equals the newest answer in the DOM                                                               |
| A stale draft is replaced, not appended          | type question A, abandon it, then ask B: the answer is to B, and the composer never sends A                                                                                                                       |
| A composer that kept the old draft is refused    | with a draft present, ask a different question: the call exits non-zero as `not-ready` naming `mismatch`, and no turn is added                                                                                    |
| A settled answer equals the page's               | with the default `--format text`, compare the returned `text` with `.prose[data-renderer='lm']`'s last `innerText` after the call: they are equal, not a prefix                                                   |
| A Markdown capture keeps the answer's structure  | `--format markdown` on a page holding a table, a fence, a formula and a nested list returns a pipe table with one line per row, a fence carrying the code's language, and `$…$` equal to the formula's annotation |
| A citation is not a link                         | a numeric citation anchor inside the answer comes back as the number the page showed, not as `[1](https://…)`                                                                                                     |

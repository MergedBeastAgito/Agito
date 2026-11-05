---
name: chat-research
description: Use when a chat page must be asked, settled and read.
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
  [--format text|markdown|note] \
  [--thread <id>] [--title <text>] [--project <name>] \
  [--account <label>] [--captured <stamp>] \
  --question '<text>' \
  --timeout 300000
```

The result is `{"url", "format", "text", "model", "mode", "trace", "sources"}`, where `url` is the
page's own and not the one the call was given - with a bind the two differ. `--format note` is the
exception: it prints the note itself on stdout, so a redirect gets a file rather than a JSON string
holding a document. `--bind` drives the tab a person is already looking at; without it the call
opens a page of its own. Use a bind for research that belongs in an existing thread, and a new page
for a probe you intend to throw away.

---

## Two readings of the same answer

`--format` decides how the answer is read, and - for `note` alone - what is printed. `text` and
`markdown` are one JSON read two ways: the same element, the same scroll, the same busy control, the
same turn correlation.

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

### One read, two renderings

Two calls are two page reads, and the second read can see a page that has changed under it: a token
still arriving, a source chip that appeared, an answer collapsed behind a control. The two
renderings then describe different moments, and nothing in either of them says which one is older.

`--format both` takes them from one evaluate. The expression returns `text` and `markdown` of the
same element in the same synchronous call, so the pair describes one moment by construction rather
than by agreement between two. The text half is the reading the completion rule runs on, and the
turn marks are computed from it, so a `both` call settles on exactly the reads a `text` call takes:
the second rendering costs no page read. `--observe` shows that directly - the same reads, the same
character counts, for two outputs.

The JSON carries both renderings and, beside them, the note built from the Markdown half. Nothing is
written: stdout is the sink, and a caller that wants files redirects or splits the document itself.

---

## The note shape

`--format note` prints the research note instead of the JSON: YAML frontmatter, a table of the same
facts under the labels a person reads, then `## Transcript`. It is the only shape whose output a
reader opens rather than a program parses.

The reading is a consequence of the shape and not a second choice. A note reads Markdown whatever
the selector file says, because a note built from `innerText` carries a formula as one glyph per
line - the same answer, unreadable. So `markdown` stays the reading and `note` is the output: one
flag asks for both, and the pair that produces glyphs cannot be asked for.

`--format both` prints the same note inside its JSON, beside the two renderings it was built from,
so one call gives a program the glyph reading and a reader the TeX document. It is the same note:
the renderer takes the richest rendering the answer carries, and the pair read is the one call where
that is the Markdown half.

Five facts arrive as flags, because no answer's text can produce them:

- `--thread` - absent, it is the identifier in the page's own URL (`…/search/<id>`, query string
  ignored). A URL that names none is refused before anything is typed, rather than filed under a
  guess.
- `--title` - absent, the note is titled by its thread.
- `--project` - absent, the note states an empty project rather than naming one.
- `--captured` - absent, the moment the answer settled is used, in local time, because a note is
  filed by the day the person captured it.
- `--account` - not a fact the note records: it is the label the page shows for the signed-in
  person, used to recognise that line of furniture and then dropped. A note that recorded who was
  signed in is a note that cannot be published.

The trace count, the source count and the transcript come from the page and the answer. Nothing is
inferred from a filename, a directory or a previous note.

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

**An identical re-ask never settles, and that is the membership rule working.** A question asked
again in the same thread is answered with the same words, so the new turn's identifier is already in
the set read before the send, `fresh` stays false, and the call waits out its deadline as
`not-ready` with `the last read was nothing`. Rephrase the question, or ask it in a thread that does
not hold that answer. Reading one answer twice means reading the page twice, not asking twice.

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

---

## Weighing what comes back

The conduct of the research - what to send, what to read of the answer, and how much of it to
trust - is `Research-Conduct/`, because those rules hold for whatever product answered and this
skill holds for one product.

---

## The result names the model and the mode

The result carries `mode` and `model`. The page that answered is whichever tab `--bind` held or
whichever page the call opened, so a caller recording provenance takes it from the call rather than
from the answer text.

A follow-up in a bound tab carries that tab's thread, which is what makes a second question cheaper
and what makes it compete with everything already there for the answer budget.

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
| The note is a file a reader opens                | `agentic chat --format note` writes stdout whose first line is `---`, and whose frontmatter states all eleven facts - `title` through `characters` - in one order                                                 |
| The note carries TeX where text carries glyphs   | on one settled answer, `--format note` transcribes `$x = \frac{-b \pm \sqrt{b^2 - 4ac}}{2a}$` while the same element's `innerText` is one glyph per line                                                          |
| A note with no thread is refused, not filed      | `--format note --url https://example.test/` exits non-zero naming `--thread`, and no question is typed                                                                                                            |
| One read yields both renderings                  | `--format both` returns `text` and `markdown` from one call, and `--observe` prints the same reads - the same count and the same character counts - as `--format text` on the same page                           |
| The two renderings agree on the answer           | the returned `text` and the note's transcript state the same prose word for word, and the one difference is the formula: glyphs in `text`, its TeX in `markdown`                                                  |
| An identical re-ask never settles                | ask the same question twice in one thread: the second call exits `not-ready` with `the last read was nothing`                                                                                                     |
| A shared tab survives losing its session         | a tab the browser extension also holds can fail an evaluation as `navigation-failed` with `Session with given id not found`; the same call run again succeeds                                                     |

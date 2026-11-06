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
capability is `agito-chat-driver`; the verb is `agito chat`.

There is no API to call. The product is a composer, a send control and a block of text, so the
driver's rules are about what a page is allowed to claim.

---

## The call

```sh
Target/debug/agito chat \
  --url https://example.test/ \
  --model '<the label the page shows>' \
  --attach ws://127.0.0.1:<debug-port>/devtools/browser/<uuid> \
  [--bind <target-id>] \
  [--selectors <file.json>] \
  [--format text|markdown|both|note] \
  [--thread <id>] [--title <text>] [--project <name>] \
  [--account <label>] [--captured <stamp>] \
  --question '<text>' \
  --timeout 300000
```

`--model` and `--question` are both optional in the sense that a call which asks nothing does not
need them:

```sh
agito chat --url <product> --attach <ws> --bind <target> --probe   # what the composer row holds
agito chat --url <product> --attach <ws> --bind <target> --clear   # empty the composer
```

The result is `{"url", "format", "text", "model", "mode", "trace", "sources"}`, where `url` is the
page's own and not the one the call was given - with a bind the two differ. `--format note` is the
exception: it prints the note itself on stdout, so a redirect gets a file rather than a JSON string
holding a document. `--bind` drives the tab a person is already looking at; without it the call
opens a page of its own. Use a bind for research that belongs in an existing thread, and a new page
for a probe you intend to throw away.

`--probe` prints every `aria-haspopup` control the model selector matched, what each one says, why
it was or was not taken as the model, and what the editor holds. It types nothing. It exists because
a build that moves the model control otherwise shows up as a `wrong-model` naming something absurd,
and one probe run is what makes that diagnosable.

`--clear` empties the addressed composer with real key events - the platform's select-all chord
carrying the protocol's own `selectAll` command, then a delete - and reads the editor back. A
composer that still holds text exits non-zero with one `blocker:` line. `chat` runs the same path
before it types, so a caller never has to know the editor was dirty.

---

## Two readings of the same answer

`--format` decides how the answer is read, and - for `note` alone - what is printed. `text` and
`markdown` are one JSON read two ways: the same element, the same scroll, the same busy control, the
same turn correlation. `both` is those two from one read, and `note` is a document built from the
Markdown one.

- **`text`** is `innerText`, trimmed. It is the default.
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

## The mode is a precondition, refused rather than assumed

The composer's toggle is read before anything is typed. Not `search` means it is pressed once, read
again, and then refused as `wrong-mode` - nothing was sent. One press, not a loop: a toggle that
does not move on the first press will not move on the fifth, and a retry loop spends the caller's
deadline proving it.

---

## The model is a report field, and a gate only when one is demanded

The label is read before anything is typed. What it reads is the model control's own name, or
`unknown`.

`unknown` is the honest answer more often than it looks: the reference product's composer row
renders its popup controls in a state-dependent order, and on a build whose first popup is a drafts
control the label is a count - `1 draft` - which is a fact about the drafts, not about the model. So
every control the selector matched is classified, and a label that is a count or that names a
control rather than a model is never taken as one. A row with nothing left answers `unknown`, the
call proceeds, and the result says `"model": "unknown"` rather than naming something it did not
read.

A model that _is_ identified and differs from `--model` is still refused as `wrong-model`, and
nothing was typed. `--model` absent means no model is demanded at all; the page's own label is
reported.

`--model` cannot be satisfied by a value that changes when the number of drafts changes, because a
count is never read as a model.

---

## Typing is an edit, not a property write

A contenteditable editor keeps its own document model. Writing `innerText` changes the DOM and
leaves the send control disabled, because no edit happened - the symptom is
`not-ready: the send control is disabled after the question was typed`.

Three rules follow.

**A draft is emptied through the input domain, not through an editing command.**
`execCommand("delete")` over a selected range answers `true` on an editor that keeps its own model
and deletes nothing, so a "select all, delete, insert" sequence appends. Measured on the reference
product: a composer holding 4,167 characters grew to 8,496 over two calls while every delete
reported success. The clear is a real select-all chord carrying the protocol's `selectAll` editing
command, then a delete, then a read-back. A composer that will not empty is a named blocker _before_
the insert, never a `mismatch` after it.

The command matters and the chord alone is not enough: a chord is something the page has to
interpret, and a plain contenteditable or a `textarea` does not interpret it. Measured on a plain
contenteditable, `Meta+a` left the selection empty while the same press carrying `selectAll`
selected the whole editor.

**The composer is read back, and the comparison is whitespace-blind with a length range.** An editor
may lose a paragraph break - this product's editor renders one with no separator at all, so the text
reads `them.The` where the caller wrote `them. The` - so whitespace is removed from both sides
before they are compared. That alone would accept an insert that dropped text, so it is paired with
a range: the held text may not be shorter than the question's non-whitespace length (a prefix or a
truncation falls below it) and may not exceed the question's length plus that floor (a composer
still holding the previous draft and this question is above it). A refusal states both lengths and
the first differing offset:

```
not-ready: the composer does not hold the question
held 50 characters, wanted 132; first difference at 40: "" against "leventwelvethirteenfourt"
```

**A failed insert is retried once through the protocol's own input.** If the editing command
refuses, or the read-back disagrees, the composer is emptied again and the text is inserted with
`Input.insertText`, which is what a paste or an IME commit does and what an editor that refuses an
editing command still obeys. The retry is once: a second failure is the refusal above, and nothing
was sent.

---

## A long question has a stated limit

`MaxQuestion` is 100,000 characters, and it is measured rather than chosen. Against the reference
product's composer, through both insertion paths:

- **4,219** characters inserted and read back - the question this limit was written for;
- **200,000**, **240,000** and **400,000** inserted and read back in full;
- **800,000** and **1,200,000**: the editing command answered `true` and the editor kept its
  previous content. Nothing but a read-back can tell the caller the question never landed;
- `getconf ARG_MAX` on the machine that measured this answers 1,048,576, so a question above that
  never reaches `--question` at all.

The bound is a quarter of the largest size measured to work. A question above it is refused before
typing, with its own length in the detail, instead of arriving as a read-back mismatch nobody can
act on.

A second measurement belongs beside that one and is not a length at all: an insert sent to a
composer whose editor has not finished initialising reports success and holds nothing, at any size.
That is what the read-back and the one retry exist for, and it is why a call that fails to insert is
not automatically a call about a long question.

---

## What is left over is `not-ready`

**A collapsed answer is `truncated`.** The control that hides the rest of an answer is looked for
inside the newest answer once it has settled, because at that point the text in the DOM is
demonstrably not all of it.

Everything else the page does is `not-ready`, with the cause in the detail. The manifest has four
codes and the driver does not invent a fifth.

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
| A question is answered and read back             | `agito chat --attach <ws> --model '<label>' --question 'Reply with exactly: ok'` returns `text` containing `ok`                                                                                                   |
| The mode is enforced, not assumed                | a page whose toggle reads `computer` and will not switch exits non-zero as `wrong-mode` and sends nothing                                                                                                         |
| The model is compared before typing              | `--model 'not-the-page-label'` exits non-zero as `wrong-model` and sends nothing                                                                                                                                  |
| A busy page does not settle                      | ask a long question and read at ~10 s: the process is still running while the stop control is visible                                                                                                             |
| Typing produces a real edit                      | after a failed send, the composer holds the question text and the send control is enabled                                                                                                                         |
| A settled answer is the answer, not the question | the returned `text` is not equal to `--question`                                                                                                                                                                  |
| A bind writes into the tab, not a new one        | `/json/list` target count is unchanged across a `--bind` call                                                                                                                                                     |
| A research loop keeps context                    | a follow-up in the same tab answers using the earlier turn without restating it                                                                                                                                   |
| A virtualised thread still settles               | ask a sixth question in a thread long enough that the count has stopped growing: the call returns, and its text equals the newest answer in the DOM                                                               |
| A stale draft is replaced, not appended          | type question A by hand, then ask B: the answer is to B, and the editor's length after the send is B's, not A's plus B's                                                                                          |
| A composer that cannot be emptied is a blocker   | point `composer` at a selector matching nothing: `--clear` exits non-zero naming the selector, and no question is typed                                                                                           |
| A truncated insert is refused                    | a composer that keeps only a prefix of the question exits `not-ready` with both lengths and the first differing offset, and no turn is added                                                                      |
| A multi-paragraph question sends first time      | a question with blank lines between its paragraphs is sent on the first attempt against an editor that renders a break with no separator                                                                          |
| The model is never a count                       | `--probe` on a row whose first popup is a drafts control reports that candidate as `count` and the model as `unknown`                                                                                             |
| An unidentified model proceeds                   | `--model 'anything'` on a row with no identifiable model answers, and the result says `"model": "unknown"`                                                                                                        |
| An identified model still gates                  | `--model 'not-the-page-label'` against a row that names its model exits `wrong-model` and sends nothing                                                                                                           |
| A long question is refused by name               | a question above `MaxQuestion` exits `not-ready` naming its length and the limit, before anything is typed                                                                                                        |
| A settled answer equals the page's               | with the default `--format text`, compare the returned `text` with `.prose[data-renderer='lm']`'s last `innerText` after the call: they are equal, not a prefix                                                   |
| A Markdown capture keeps the answer's structure  | `--format markdown` on a page holding a table, a fence, a formula and a nested list returns a pipe table with one line per row, a fence carrying the code's language, and `$…$` equal to the formula's annotation |
| A citation is not a link                         | a numeric citation anchor inside the answer comes back as the number the page showed, not as `[1](https://…)`                                                                                                     |
| The note is a file a reader opens                | `agito chat --format note` writes stdout whose first line is `---`, and whose frontmatter states all eleven facts - `title` through `characters` - in one order                                                   |
| The note carries TeX where text carries glyphs   | on one settled answer, `--format note` transcribes `$x = \frac{-b \pm \sqrt{b^2 - 4ac}}{2a}$` while the same element's `innerText` is one glyph per line                                                          |
| A note with no thread is refused, not filed      | `--format note --url https://example.test/` exits non-zero naming `--thread`, and no question is typed                                                                                                            |
| One read yields both renderings                  | `--format both` returns `text` and `markdown` from one call, and `--observe` prints the same reads - the same count and the same character counts - as `--format text` on the same page                           |
| The two renderings agree on the answer           | the returned `text` and the note's transcript state the same prose word for word, and the one difference is the formula: glyphs in `text`, its TeX in `markdown`                                                  |
| An identical re-ask never settles                | ask the same question twice in one thread: the second call exits `not-ready` with `the last read was nothing`                                                                                                     |
| A shared tab survives losing its session         | a tab the browser extension also holds can fail an evaluation as `navigation-failed` with `Session with given id not found`; the same call run again succeeds                                                     |

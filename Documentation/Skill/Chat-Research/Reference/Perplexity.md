# Perplexity: what the page actually is

Every selector here was read from a live tab with
`agito browse --bind <target-id> --expression '<probe>'`, not guessed from a framework. Markup is
not a contract: re-probe before trusting any line below.

---

## The selectors the defaults use

| Role     | Selector                                                                                                              | What it matched                                                                    |
| -------- | --------------------------------------------------------------------------------------------------------------------- | ---------------------------------------------------------------------------------- |
| mode     | `[data-command-menu-control] button`                                                                                  | two buttons, `aria-label` `Search` / `Computer`, `aria-pressed` on each            |
| model    | `[class*='composer-row'] button[aria-haspopup], [data-appearance='framed'] button[aria-haspopup]`                     | one or two buttons: a drafts count and the model's name, or the model's name alone |
| composer | `textarea, [contenteditable='true']`                                                                                  | one `div[contenteditable=true]`                                                    |
| submit   | `[data-testid='submit-button'], button[aria-label='Submit']`                                                          | one button, `disabled` while the composer is empty                                 |
| answer   | `.prose[data-renderer='lm']`                                                                                          | the answer body, one per turn                                                      |
| busy     | `button[data-testid='stop-generating-response-button'], button[aria-label*='Stop' i], button[aria-label*='Cancel' i]` | `Stop response (Esc)`, visible only while generating                               |
| collapse | `button` (scoped inside the answer)                                                                                   | no control matched: nothing is clamped behind a "read more"                        |
| trace    | `[data-testid='trace-step'], [role='listitem']` (scoped inside the answer)                                            | nothing: the trace lives outside the answer body                                   |
| sources  | `a[href^='http']` (scoped inside the answer)                                                                          | nothing: citations are not anchors inside the body                                 |

---

## The two places a naive reading is wrong

**The question is rendered by the same renderer as the answer.** Both carry `data-renderer="lm"`, so
a selector of `[data-renderer='lm']` counts the question as an answer, and a stability loop settles
on the question the moment it appears - returning the prompt as the answer. The discriminator is the
`prose` class, which only the answer body carries; the question's renderer sits inside
`[data-message-bubble='right']` and has no `prose` class.

**The answer body is not the turn.** The turn is a `[data-workflow-items]` block holding the body,
the step list, the duration and the source chips. The body alone is the right container for text and
the wrong one for provenance: `trace` and `sources` come back empty because they are siblings of the
body, not descendants. Widening `answer` to the workflow block fills them in and makes the text
noisy with the trace and the chips. Pick one, and say which in the report.

---

## Facts worth carrying

- The mode toggle is two buttons, not one cycling control. `aria-pressed="true"` is on the selected
  one, so "the label of the single pressed toggle" is a well-defined read.
- The model button carries no `data-testid`; its `aria-label` _is_ the model name. That is why the
  driver compares the label rather than asserting a model id.
- The send control is `disabled` until the composer has a real edit in it, which is what makes a
  property-write typing path fail loudly instead of silently.
- A session started from a project page's composer is filed in the project while its URL is the
  ordinary `/search/<id>` form. The project owns membership and instructions; the thread owns the
  transcript. Read the thread URL for text, the project URL for where the next question lands.
- The project page's own composer is a valid research surface: binding it and submitting starts a
  thread in that project without opening a tab.

---

## What a live turn actually does

Sampled at 250 ms through a bound tab, with the stop selector and the turn text recorded on every
tick:

| When                  | New answer exists | Stop control | Turn header         |
| --------------------- | ----------------- | ------------ | ------------------- |
| 0 - ~3 s after submit | no                | no           | the previous turn's |
| ~3 s                  | no                | **yes**      | previous turn's     |
| ~5 s                  | yes, 64 chars     | yes          | `Finished 4s …`     |
| +250 ms each          | growing           | yes          | `Finished 4s …`     |
| last ~250 - 500 ms    | still growing     | **no**       | `Finished 4s …`     |
| after                 | final             | no           | `Finished 4s …`     |

Three consequences, each of which cost a wrong implementation to find:

**The stop control appears late and disappears early.** It is absent for the first ~3 s after
submit - the search phase - and absent again for the last few hundred milliseconds while text is
still landing. So `busy` is a gate, not a proof: it blocks the pre-generation window (where the
thread still holds the previous answer) and the streaming window, and the three-stable-reads rule
covers the tail it misses.

**`Finished` is a duration label, not a completion marker.** The turn header reads `Finished 4s`
from the first streamed token, so a `done` pattern of `^Finished` matches a turn that has just
begun - the exact false settle the check exists to prevent. That is why `done` and `turn` default to
empty and why a pattern must be enabled only after watching one live turn end to end.

**A long thread is virtualised.** The oldest turn unmounts as the newest arrives, so the answer
count stops growing - and a driver that identifies the new turn by `count > before.count` waits out
its whole deadline on a page whose answer is plainly there. The turn is identified by text identity
as well, which survives the unmounting.

---

## The model control is not stable, and the row's first popup is not always it

The model button has been observed with `aria-haspopup="dialog"` and, on a later build, with
`aria-haspopup="menu"`. A selector pinned to `dialog` reads `null` on the build that uses `menu`,
and the driver then refuses the call as `wrong-model` with `the page reads nothing` - a correct
refusal about a selector that aged out.

Select on the attribute's presence, not its value, and scope it to the composer:
`[class*='composer-row'] button[aria-haspopup]`. But presence is not enough either, because the row
renders more than one popup control and their order is state-dependent:

| State                       | The row's popup controls, in document order |
| --------------------------- | ------------------------------------------- |
| a thread with a saved draft | `1 draft`, `Model`                          |
| a page with no draft        | `GPT-6.1 Sol`                               |

So on one build the first popup's label is a count and on another it is the model's own name. A
driver that read the first match returned `1 draft` as the model, which no `--model` value could
satisfy and which changed whenever the number of drafts changed.

What the driver does now: it reads every match, classifies each label - a count is `count`, a word
naming a control (`model`, `drafts`, `session`, `share`, …) is `names-a-control` - and takes the
first one that is neither. A row with nothing left answers `unknown` and the call proceeds.
`--probe` prints the whole inventory, which is what makes the next build's change diagnosable in one
run.

Two live examples, both from `agito chat --probe`:

```json
{"model": "GPT-6.1 Sol", "chosen": 0, "candidates": [{"label": "GPT-6.1 Sol", "reason": "model"}]}
{"model": "unknown", "chosen": null, "candidates": [{"label": "1 draft", "reason": "count"}]}
```

## The scroll control and the in-flight flag

Two things on the page carry the state a driver needs, and neither is the one a naive read would
pick.

**`button[aria-label="Scroll to end"]`** is how the thread returns to its end, and it is visible
whenever the newest turn is off-screen. Measured on a long thread: the last answer sat at y = 25,910
in a 779 px viewport, so a read taken there measured older turns. Pressing it (or
`scrollIntoView(false)` on the last answer) puts the newest answer's bottom exactly at the viewport
edge. Note that `window.scrollY` stays 0 - the app scrolls an inner container, so a driver that
checks `window.scrollY` to decide "am I at the end" always thinks it is not.

**`[data-in-flight]`** on the scroll-to-end icon is the page's own "still producing" flag: `"true"`
while generating, `"false"` when idle, always present and always visible. It is a better busy signal
than the stop control because it exists for the whole turn rather than flickering, and it costs one
attribute read in the same expression that reads the text.

## A composer that keeps its draft

`document.execCommand('insertText')` reports `true` over a draft that the editor still holds.
Measured here: a question was typed over a 642-character leftover, the call reported success, and
the answer came back to the _leftover_ question - 7,133 characters of confident answer to something
nobody asked.

The same editor answers `execCommand('delete')` over a selected range with `true` and deletes
nothing, so the obvious repair - select all, delete, insert - appends instead: a composer holding
4,167 characters grew to 8,496 over two calls while every delete reported success, and every later
call refused with `not-ready: the composer does not hold the question`. The tab was wedged; nothing
in the tool could recover it.

Three rules follow, and all three are in the driver now:

- empty the composer through the input domain - a select-all chord carrying the protocol's own
  `selectAll` command, then a delete - and read the editor back; a composer that will not empty is a
  named blocker before the insert;
- read the composer back after typing and compare it to the question with whitespace removed, plus a
  length range, because this editor renders a paragraph break with no separator at all;
- retry a failed insert once through `Input.insertText`, which is what an editor that refuses an
  editing command still obeys.

An empty composer is the easy case and it hides all of this: every early probe passed because the
composer started empty. The failure only appears on the second consecutive call in the same tab.

Two further measurements from the same editor, both of which the crate's `MaxQuestion` records:

- an insert sent before the editor has finished initialising reports success and holds nothing, at
  any size - which is why a failed insert is retried rather than reported as a length problem;
- 400,000 characters insert and read back; 800,000 report success and leave the previous content in
  place. `getconf ARG_MAX` is 1,048,576, so a question larger than that never reaches `--question`.

## The cold-start race on a newly opened page

`chat` without `--bind` opens a page and probes it immediately. On a fresh page the composer can be
present before the model button has rendered, and the read then returns `nothing` - which the driver
reports as `wrong-model: the page is set to a different model … the page reads nothing`. That is a
page still arriving, not a different model; the selector is fine (the same selector reads
`GPT-6.1 Sol` on that page a moment later).

Bind an already-open tab for research, and treat that particular `wrong-model` whose detail says the
page reads nothing as "retry", not as "wrong model".

## Finding a thread whose id was never recorded

A project page (`/projects/<uuid>`) lists its threads in a sidebar whose anchors carry the question
as `aria-label` and the thread as `href`. Both attributes are readable, and the row's own text is
not: `innerText` of the row answers only "Recents", because the title is drawn elsewhere.

```js
[...document.querySelectorAll("a[href*='/search/']")].map((Anchor) => ({
	href: Anchor.href,
	question: Anchor.getAttribute("aria-label"),
}));
```

That is how a capture whose note recorded the bare origin (`https://www.perplexity.ai/`) is re-read:
find the thread by the opening of its question, then open `/search/<id>` in a background tab
(`Target.createTarget` with `background: true` on the browser-level socket) so the person's focus
stays where it was.

---
name: chat-selectors
description: Use when chat markup must be described as selectors.
version: 1.0.0
license: MIT
platforms: [macos, linux]
metadata:
    hermes:
        tags: [cdp, rust, selectors, adapter, configuration]
---

# Chat Selectors

The chat driver knows what to read, never where it lives. The where is a `Selectors` value: eight
strings, merged over the defaults from a JSON file.

---

## The file

`--selectors <path>` takes a JSON object. Keys that are absent keep their default; a key that is
present replaces it.

```json
{
	"answer": ".prose[data-renderer='lm']",
	"busy": "button[aria-label*='Stop' i]"
}
```

The merge is deliberate. A caller changing one selector should not have to restate the seven it is
happy with, and an absent key must keep the default rather than become an empty selector that
matches everything.

---

## The eight keys

| Key        | What it must find                                                                                  | What breaks without it                                                               |
| ---------- | -------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------ |
| `mode`     | the composer's mode toggles, each carrying its mode as a label and its selection as `aria-pressed` | the search-mode precondition cannot be read, and the call is refused as `wrong-mode` |
| `model`    | every popup control where the model is named, and nothing else                                     | the model cannot be identified, and the report says `unknown`                        |
| `composer` | the editable that accepts text                                                                     | nothing can be typed                                                                 |
| `submit`   | the control that sends                                                                             | nothing can be sent                                                                  |
| `answer`   | every answer in the thread, in document order                                                      | the thread cannot be counted, so a new turn cannot be told from an old one           |
| `collapse` | the control that hides the rest of an answer, searched **inside** the newest answer                | a truncated answer is returned as if it were whole                                   |
| `trace`    | one step of the answer's trace, searched **inside** the newest answer                              | the trace comes back empty                                                           |
| `sources`  | one cited source, searched **inside** the newest answer                                            | the sources come back empty                                                          |
| `busy`     | the control that means the page is still producing, searched **across the page**                   | text stability alone settles partial answers                                         |
| `turn`     | the answer's turn container, searched for as an **ancestor** of the answer                         | the turn-scoped checks below have nothing to read                                    |
| `done`     | a pattern the turn's text matches **once the turn is over**                                        | completion stays on text stability alone                                             |
| `scroll`   | the control that returns the thread to its end                                                     | the newest turn is never mounted, so reads measure old turns                         |

---

## Two keys must never be inferred, and one must not be trusted

`answer` and `busy` are the two the driver refuses to guess at:

- **`busy`** because without it completion is a guess about silence;
- **`answer`** because without a turn boundary a driver returns an old answer and calls it new.

`model` is the opposite case and the more dangerous one: a selector that matches too much is worse
than one that matches nothing. The composer row of a real product renders several `aria-haspopup`
controls and their order is state-dependent, so a selector that matches the row's popups can read a
drafts count as the model's name. The driver classifies every match - a count is `count`, a word
naming a control is `names-a-control` - and answers `unknown` when nothing is left. `unknown` is a
report field, not a failure: the call proceeds and the result says so.

A missing `busy` is not fatal the way a missing `answer` is: the driver reads "nothing says it is
still working" and leaves the text rule in charge. That is a degraded mode, not a correct one, and
the report should say so.

`agito chat --probe` is the check: it prints every control the `model` selector matched, what each
one says, and which one was chosen.

---

## Scoping is part of the selector

`collapse`, `trace` and `sources` are queried **inside the newest answer**, so an earlier turn's
control is never this answer's. `busy` is queried across the page, because a product keeps its stop
control in the composer: it is a fact about the turn, not about the text the turn has produced.

A selector that is right about the element and wrong about the scope produces a confident wrong
answer, which is worse than an empty one.

---

## Write selectors that survive a re-render

Prefer, in order: a stable `data-` attribute, an accessible label, a structural relationship that is
part of the component's meaning. A generated class name is the last resort, and a positional
selector (`div:nth-child(3)`) is not a selector at all - it is a description of today's markup.

Two specific traps:

- **`[class*='…']` matches any class containing that substring**, including one that means something
  else. Scope it to a parent that has the meaning you want.
- **The first match in document order is not the visible one.** A panel or dialog can leave a hidden
  editor earlier in the tree, and a send into it looks like a dead page.

---

## Reference

`Reference/Index.md` - the file list for this skill.

---

## Claim → test

| Claim                                                 | Test                                                                                      |
| ----------------------------------------------------- | ----------------------------------------------------------------------------------------- |
| An override replaces one key and keeps the rest       | a file with only `answer` set still reads mode and model from the defaults                |
| An absent key is not an empty selector                | a file with `{}` behaves identically to no `--selectors`                                  |
| A malformed file is refused                           | `--selectors` pointing at invalid JSON exits non-zero naming the file                     |
| A wrong `answer` selector is visible, not silent      | point `answer` at `body` and the returned text contains the product's chrome              |
| `busy` is read across the page, not inside the answer | a stop control in the composer still gates settlement when `answer` is scoped to the body |

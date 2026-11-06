---
name: browser-eval
description: Use when one expression must answer a question about a live page. One bounded read.
version: 1.0.0
license: MIT
platforms: [macos, linux]
metadata:
    hermes:
        tags: [cdp, rust, evaluate, expression, probe]
---

# Browser Eval

Ask a live page one question and read the answer as JSON. The verb is `agito browse`; the
operation is `Browse::Evaluate`.

---

## The shape of a call

```sh
Target/debug/agito browse \
  --url https://example.test/ \
  --attach ws://127.0.0.1:<debug-port>/devtools/browser/<uuid> \
  [--bind <target-id>] \
  --expression '<expression>' \
  --timeout 40000
```

The result is `{"url", "ownership", "value"}`, where `url` is the URL the page was actually on when
it was bound - not the `--url` argument. A bound tab keeps whatever it was showing, so the two
disagree by design and the reported one is the truthful one.

---

## Write an expression that returns data, not prose

An expression is one JavaScript expression. Make it an immediately-invoked function that returns an
object, so a probe answers several questions in one round trip instead of several:

```js
(() => ({
	title: document.title,
	editables: document.querySelectorAll("[contenteditable='true']").length,
	visible: [...document.querySelectorAll("button")].filter((b) => b.getClientRects().length > 0)
		.length,
}))();
```

Rules that keep a probe honest:

- **Return a value.** `undefined` arrives as `null`, and `null` is indistinguishable from "the page
  does not have it" unless the expression says which.
- **Trim what you read.** A page's whitespace is not part of what it said, and two reads that differ
  only in trailing spaces have not changed.
- **Count, do not assume.** `querySelectorAll(...).length` tells you whether a selector is unique;
  `querySelector(...) !== null` does not.
- **Visibility is a computed style, not a rect.** `getClientRects().length` is the cheap test and it
  is wrong for a `display: contents` wrapper (no box, renders children) and for a
  `content-visibility: auto` subtree (no rects while offscreen). Use it only where you have checked
  the element's own layout.

---

## A read is not a write

`browse` evaluates whatever it is given, so it can click and type. Keep the distinction explicit in
what you run: a probe reads, and a probe that mutates is a mutation wearing a probe's clothes. When
the intent is to act, say so in the command you keep, because the next reader will assume a `browse`
call was read-only.

An expression that throws is `navigation-failed` with the exception in the detail, not a silent
`null`. That is the useful direction: a broken selector in a probe should look broken.

---

## Budgets

`--timeout` is the whole operation's budget in milliseconds and covers open plus evaluate. Silence
is a `timeout`, never a dead transport, so a page that never answers is reported as slow rather than
as missing.

The caller's cancellation and the deadline race the work; whichever loses is reported as `timeout`
with a different sentence. That is how "I gave up" is told apart from "the browser broke".

---

## Reference

`Reference/Index.md` - the file list for this skill.

---

## Claim → test

| Claim                                            | Test                                                                                      |
| ------------------------------------------------ | ----------------------------------------------------------------------------------------- |
| One expression returns structured data           | `--expression '({a: 1+1})'` prints `"value": {"a": 2}`                                    |
| The reported URL is the page's, not the argument | bind a tab on `https://a/` and pass `--url https://b/`; the result says `https://a/`      |
| A throwing expression is reported                | `--expression '(() => { throw new Error("x") })()'` exits non-zero as `navigation-failed` |
| An undefined result is `null`                    | `--expression 'undefined'` prints `"value": null`                                         |
| A silent page is a timeout, not a dead socket    | `--expression 'new Promise(() => {})'` exits non-zero as `timeout`                        |

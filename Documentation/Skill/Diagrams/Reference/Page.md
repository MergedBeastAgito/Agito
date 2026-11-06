# One Folder, End to End

`Documentation/Diagram/20-Browser/` is the worked instance: four pages and four sources, each with
its rendered sibling. The one page in the set with no source is `40-Database` page 3, whose type
cannot be exported.

## What the folder holds

```
20-Browser/
  README.md        the page
  Lifecycle.mmd    page 1 source
  Lifecycle.svg    page 1 render
  Failures.mmd     page 2 source
  Failures.svg     page 2 render
  Identity.mmd     page 3 source
  Identity.svg     page 3 render
  Missing.mmd      page 4 source
  Missing.svg      page 4 render
```

The stems match, so the pair is obvious from a directory listing and the render command is
`line9 render <stem>.mmd --out <stem>.svg` with no second decision.

## The four pages, in the order that matters

**Page 1 - the states the code implements.** `Lifecycle.mmd` is a `flowchart TD` from `Unbound`
through `Launched` or `Attached`, to `Live`, `Evaluated`, `Unavailable`, `Disposed`. The page's one
sentence is the distinction that holds on every path: `dispose` on a launched session may close the
browser, and on an attached session it must not, because somebody else owns it. The diagram shows
the branches; the sentence says which branch is the reason the diagram exists.

**Page 2 - the failures, and the one that hides.** `Failures.mmd` branches into three named failures
plus a state drawn as `looks alive`. That state is the point: an open socket is not a live browser,
and the liveness probe is what separates them.

**Page 3 - identity.** `Identity.mmd` puts an endpoint, a page and a user in one drawing, because
three things here are called "the browser" and only one of them is an identity.

**Page 4 - the states the adapter does not model.** `Missing.mmd` draws `Stalled` and `Reclaimed`
with `:::hook`, which is how an absent state is drawn as an installable step rather than as an
omission. Both are real: the first is the dead-but-open socket, the second is two debugger clients
on one page.

The order is the argument. Pages 1 to 3 say what the code does; page 4 says what it does not, and a
reader who stops after page 3 has read a system that is complete when it is not.

## The page's shape, in bytes

```md
## Page 2 - the three failures, and the one that hides

![An operation branching into three named failures and one that looks alive](Failures.svg)

[`Failures.mmd`](Failures.mmd) · [`Failures.svg`](Failures.svg)

`looks alive` is the state the adapter cannot name: an open socket is not a live browser.
```

A heading that names the page's claim, the image with alt text naming the states, the link pair,
then the rule. Pages are separated by a `---` rule. The folder's lead sentence sits under the `#`
heading, before page 1.

## Adding a folder

1. Take the next free number by tens and name the category after what it draws, not after a crate:
   `00-Rings`, `10-RunTime`, `20-Browser`, `30-Chat`, `40-Database`, `50-Feed`.
2. Write the `.mmd` sources first, each with `title` and `line9: theme: boardroom`.
3. Render each one to its sibling and grep the SVGs for `...` before writing a word of prose.
4. Write `README.md`: the lead sentence, then pages in the order above.
5. Add the row to `Documentation/Diagram/Index.md` - folder, category, what it draws.

A category that needs a diagram line9 cannot export gets a fenced `mermaid` block and a sentence
naming the limit, as `40-Database` page 3 does. It does not get a `.mmd` that cannot render, because
a source with no sibling is a broken promise the next reader cannot repair.

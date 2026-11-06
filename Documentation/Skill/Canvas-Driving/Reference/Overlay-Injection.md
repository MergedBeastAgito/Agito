# Injecting an overlay into a canvas app

The overlay is usually the deliverable, not a debug aid: it is the only window a person has on a
loop they cannot read from pixels. Its failures are all silent - the panel renders, looks plausible,
and shows stale or partial truth. Every rule below exists because the failure it prevents looks like
success.

## The source is a string, so the language has two grammars

An overlay installed by a Node script lives in one template literal: ``const INSTALL = `…` ``.
Inside it, a backtick in a `//` comment or a `${` in a string closes or interpolates the literal,
and the parse error points at the comment rather than at the code. Write pipes, emphasis and quoted
words as plain text or single quotes, and run `node --check <script>.mjs` after every edit and
before every install - one second, and it catches this class outright.

**Escapes are eaten one level earlier than you expect.** A template literal processes backslash
escapes before the page ever sees the string, so a regex written `a.replace(/\s+/g, '-')` arrives in
the page as `a.replace(/s+/g, '-')` - a _silently wrong_ pattern, not a parse error, because `\s`
became `s`. The same applies to `\d`, `\b` and `\n` in injected source. Write such patterns without
backslashes where you can (`split` + `join`, a character class of literals, `.replaceAll`), and when
a backslash is unavoidable double it in the outer script and then confirm the page-side value by
evaluating the expression's `String(fn)` in the tab. A `node --check` pass says nothing about this:
the file is valid JS and the page runs the wrong regex.

## Style through `cssText`, because a stylesheet can be dead

A page's CSP can block injected `<style>` elements outright. Measured: a probe rule setting
`position:fixed` computed to `static`, and `document.styleSheets` parsed zero rules from 1,538
characters of valid CSS, while `el.style.cssText` worked. If an overlay renders unstyled, compare
`getComputedStyle(el).position` against what the sheet says before debugging selectors.

Keep a helper - `const S = (el, css) => { el.style.cssText = css; return el }` - and never build a
`<link>` or `<style>`.

## Build once, write values, and key the cache on a unique id

Rebuilding `innerHTML` per tick flashes. Build the panel once, write only value nodes, and compare
each string against the last before touching the DOM.

The write helper is usually `if (cache[node.id] !== text) …`, which keys on the node's `id` - so
several id-less spans all share the empty-string slot, compare against whatever a neighbour wrote
last, and skip the update whenever two cells want the same text. Give every value node a unique `id`
(`panel-<action>-<column>`) or the panel updates one cell and freezes the rest.

A marker property set on a child and still present a second later proves the node was never rebuilt.

## One paint timer per page

Re-installing without clearing the previous `setInterval` leaves two writers racing: the same panel
showed 2 km/h in one line and 71 km/h in another, from the same tick. Store the handle
(`D._paintTimer`) and `clearInterval` it before starting the next, or a reinstall silently doubles
the work and splits the truth.

## A renamed field freezes the panel with no error

The paint runs inside `setInterval`, which swallows the throw: every later tick dies on the same
line and the panel keeps displaying the last values it managed to write. A frozen panel is
indistinguishable from a quiet one, and a clean parse proves nothing about it.

When rows gain or lose fields, change the builder and the update loop in the same edit. A row built
as `{row, ms, prob, conf}` and updated as `r.val` throws forever, silently.

## A helper that reports only `exceptionDetails.text` hides every error

`Runtime.evaluate` answers a page throw with `exceptionDetails.text` set to the bare word
`Uncaught`; the message and stack live one level down in `exceptionDetails.exception.description`. A
one-line `Eval` helper that throws `new Error(res.exceptionDetails.text)` therefore turns every
page-side bug into the same anonymous failure, and the install path is exactly where that hurts: the
installer prints `Error: Uncaught` with no file, no line and no identifier. Report the description
first and fall back to the text:

```js
if (res.exceptionDetails)
	throw new Error(
		(res.exceptionDetails.exception &&
			(res.exceptionDetails.exception.description || res.exceptionDetails.exception.value)) ||
			res.exceptionDetails.text,
	);
```

With that in place the same install reported
`SyntaxError: Identifier 'modeEl' has already been declared` - a duplicate `const` between the
panel's new mode row and an older observation-header badge, which no amount of reading the diff had
surfaced.

## Prove it from the DOM, not from the source

After installing, read the panel back over CDP and assert the shape you meant:

```js
const Dec = await Eval("(document.getElementById('panel-dec')||{}).innerText || ''");
const Rule = await Eval("(document.getElementById('panel-rule')||{}).textContent || ''");
const Rows = await Eval("document.querySelectorAll('#panel-body > div[style*=\"grid\"]').length");
```

A verifier worth keeping asserts four things and stops: the script parses, the source still carries
the markers the change introduced, the install reports `installed:true`, and the DOM shows the rule
row plus the expected number of rows. Grepping the source for a field name proves the text is there;
only the `innerText` read proves the loop is still writing it.

## Panels share edges, so they collide

A full-width strip pinned to the bottom owns its whole height: a panel at `bottom: 8px` ends up
underneath it and the two read as one garbled block. Budget the edges - HUD top-left, observations
top-right, decisions above the strip, wire bottom-right - and assert pairwise non-overlap from
`getBoundingClientRect`, never by eye.

## A curated subset reads as a capture bug

The panel is the only window this person has on the loop, so "speed, gear and position" is not a
summary - it is a claim that those are all the fields there are, and the next question is "is that
all of it?". Render every field the observation schema names (velocity, yaw rate, steering,
per-wheel surface, slip, grounded), even ones no rule reads yet, and let the reader judge. The same
holds for inputs: log both directions and keep enough visible to reconstruct a tick.

## Observations are read by the panel; decisions are pushed to it

A panel that renders only what a driver published shows `?` in every field the moment nothing is
driving - an observation panel that needs a driver to observe is not an observation panel. Give the
overlay its own reader (`window.__readCar()` over the same addresses) called on the panel's own
timer, and push only what a driver alone knows: the decision, its duration, its reason.

**A failed read is a state, not a blank.** When the reader returns null the panel must say so -
header `reading` when the pointer chain resolved, `STALE` when it did not - rather than falling back
to the last published values and presenting old numbers as live ones. Silent fallback is how a stale
panel gets mistaken for a broken reader, or worse, for working.

## Panels take the mouse; the canvas keeps what the panels do not cover

`pointer-events:none` with `auto` only on buttons makes the readouts dead: text cannot be selected,
values cannot be copied, and a scrollbar inside a panel does nothing. Give every panel
`pointer-events:auto;user-select:text` and let the panel own its own rectangle - a click outside
every panel still reaches the canvas, which is what keeps the app operable while the panels are up.
Assert it once from the DOM (`getComputedStyle(el).pointerEvents` on each panel id) rather than
trusting the style string you wrote.

A panel whose content exceeds its box needs `overflow:auto` and a bounded `max-height`; report the
real numbers when you verify it (`scrollHeight > clientHeight`), because a panel that grows to fit
covers the app it describes.

## Group controls by the question they answer, and label the ones that do not steer

A flat row of switches mixes unrelated questions and every switch then reads as a peer of the
others. The failure is concrete: `LOCAL` / `REMOTE` / `PIPE` in one row read as one axis when they
were three, and two of them were not switches at all - they were the two halves a single `DRIVER`
choice already turns on. Give each question its own labelled group:

- **mode** - what the app is doing. If the app decides it, render the whole set at once and _light_
  the current one; a button implies the reader sets it.
- **authority** - who decides (rules / rules and model / model), with the halves it implies printed
  underneath rather than offered as buttons.
- **sense** - what the loop can perceive (memory / pixels / both). Names like `GPS`, `Road`,
  `Free roam` are senses, not modes; filing them as modes is what makes a panel unreadable.
- **transport** - whether a request leaves at all. A transport fact is never a peer of an authority
  choice.
- **tuning** - the numbers.

**A knob that does not steer the running process is worse than no knob.** Page state read by a JS
harness does nothing to a Rust driver that takes flags: label such a group with its real authority
(`TUNING · page harness only`) and say in the panel what the process actually reads, or the reader
tunes a value that was never consulted.

## A column is a layout, not a character

When a person writes a row of glyphs to describe what they want - a pipe row under a title, a box
drawn in ASCII - they are describing a _layout_ and handing you the nearest characters that fit in a
chat message. Rendering those characters literally is the failure mode: the row appears, the layout
does not, and the request looks answered while the panel is unchanged. Read the sketch as "these
things should sit in columns" and build the columns.

A columnar panel is a grid, and its columns are its sections:

```js
const Columns = S(
	el,
	"display:grid;grid-template-columns:1.2fr 1fr 1fr;gap:0 10px;align-items:start",
);
// candidates table | mode + authority | sense + transport + tuning
```

Widen the panel to fit three columns (600-700 px) and give every column `min-width:0`, or one long
value forces the grid wider than its own box. Verify the layout from the DOM rather than from the
style string you wrote:

```js
getComputedStyle(Grid).gridTemplateColumns; // '234.8px 195.7px 195.7px'
Grid.children.length; // 3
document.getElementById("panel-rule"); // null - the glyph row is gone
```

Delete the placeholder glyph row in the same edit that introduces the grid. Two descriptions of one
layout - a decorative rule and a real column structure - disagree the moment the panel is resized,
and the decorative one is the one a reader believes.

## Draw a request as structure, never as pretty-printed JSON

The round trip is the most interesting thing on the page and the worst rendered as a body dump: a
reader tracing braces cannot see what the model was allowed to say. Render it in three stacked
sections - the state that was sent as one line per field, then **one lane per question** with its
criteria as chips along the lane and the answered chip lit with its probability, then the raw reply
below for the case where the body is malformed and must be seen.

- A `choice` is answered by a word; a `score` by a position in its legend, so light the chip whose
  index rounds to the returned score. Both shapes occur in the same request.
- Rebuild the lanes only when the drawn thing changes - key the cache on the question ids plus the
  answers, or a lane asked every sixth tick rebuilds its nodes on the five ticks in between.
- A per-row number belongs on the row it describes. Repeating one decision's probability down every
  candidate row reads as though all of them were weighed.

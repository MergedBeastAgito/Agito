---
name: diagrams
description: Use when drawing a page under Documentation/Diagram. Source and sibling.
version: 1.0.0
license: MIT
platforms: [macos]
metadata:
    hermes:
        tags: [line9, mermaid, diagrams, svg, documentation]
        related_skills: [workspace, skill-authoring]
---

# Diagrams

A diagram set that lives only as a rendered image is a set nobody can change: the source is gone, so
the next reader redraws what the last one drew. This tree keeps both halves - a `.mmd` source with
frontmatter and typed nodes, and the `.svg` rendered from it - so a page reads without the CLI and a
change is made in a file that renders the same way twice.

## The folder

`Documentation/Diagram/<NN>-<Category>/`, one folder per category, numbered by tens so a new
category takes the next free number rather than a letter. Each folder holds:

- `README.md` - the page: a title, one sentence naming what the category draws, then the pages;
- one `<Topic>.mmd` per diagram and its `<Topic>.svg` sibling, same stem, same folder;
- nothing else. A diagram with no `.svg` is a diagram half the readers cannot see.

`Documentation/Diagram/Index.md` is the index: one row per folder with its category and what it
draws, plus the convention itself. A new folder is not finished until that row exists.

## The file

```mmd
---
title: Browser - attach or launch, and what dispose may do
line9:
  theme: boardroom
---
flowchart TD
  UNBOUND["Unbound<br>no endpoint, no process"]:::host
  ATTACHED["Attached<br>somebody else owns it"]:::host
  UNAVAILABLE["Unavailable<br>no executable, or no endpoint"]:::fail
  UNBOUND ==>|"open(url) with attachTo"| ATTACHED
  UNBOUND -.->|"no executable"| UNAVAILABLE
```

`title` is what the viewer and the SVG carry, so it names the claim rather than the file. The theme
lives in the frontmatter and nowhere else: a diagram that picks its own hex values in a `classDef`
is inconsistent with every other diagram in the set, and the set is what a reader compares.

## The legend is four annotations

A reader learns four words and every diagram here is legible. There is no `note` and no subgraph of
prose inside a drawing.

| Annotation | Means                                                     | Shape                       |
| ---------- | --------------------------------------------------------- | --------------------------- |
| `:::host`  | an existing runtime surface                               | `["…"]`, `(["…"])` an event |
| `:::hook`  | an installable step, including a state nothing implements | `["…"]`                     |
| `:::store` | persisted state                                           | `[("…")]`                   |
| `:::fail`  | an error or refusal                                       | `["…"]`                     |

Shapes carry kind on their own: `(["event"])`, `[("store")]`, `{"decision"}`, `["process"]`. Arrows
are graded: `==>` the primary flow, `-->` secondary, `-.->` optional or advisory. Line breaks are
`<br>`, never `<br/>`.

## Render

```sh
line9 render Documentation/Diagram/20-Browser/Lifecycle.mmd --out Documentation/Diagram/20-Browser/Lifecycle.svg
```

| Exit | Means                                                  |
| ---- | ------------------------------------------------------ |
| `0`  | the SVG is written                                     |
| `2`  | bad invocation, an unknown flag or argument            |
| `3`  | invalid Mermaid, the in-process parser refused it      |
| `4`  | source unreadable, or target unwritable                |
| `9`  | a valid request this version does not serve headlessly |

**`--out` serves flowcharts only.** A valid `sequenceDiagram` or `stateDiagram-v2` exits `9`, prints
`unsupported: headless SVG export covers flowcharts in v1`, and writes nothing - not an empty file,
no file. Those stay a fenced `mermaid` block in the page with one sentence saying why, and
`40-Database/README.md` page 3 is the worked case: four participants, a private read and an
audience-scoped write in one transaction, and no `.mmd`, because a flowchart would flatten who may
see what into a chain of steps. `10-RunTime` page 2 takes the other route: five events drawn as a
flowchart, with the page saying the order of the events is the sequence. `line9 render <file>.mmd`
without `--out` opens the browser viewer and serves any type; it is not a substitute for the
sibling, because the next reader has no browser open.

`line9 capabilities --json` prints the machine contract - commands, flags, exit codes, error codes -
and `line9 status` prints the licence. Read those rather than guessing a flag.

## The page is a storyboard

Pages are separated by a `---` rule. Each page is a heading naming the state group, an image embed
of the `.svg` whose alt text names the states, the `Topic.mmd` · `Topic.svg` link pair, then one
sentence stating the rule the diagram exists for.

The first page shows the states the code implements. The last page shows the states it does not,
drawn with `:::hook` so an absent state sits in the same drawing as the present ones - `Stalled` and
`Reclaimed` in `20-Browser`, `X` in `00-Rings`. A missing state that appears only in prose is a
state nobody notices is missing.

## Pitfalls

- **A diamond clips its label.** A `{"decision"}` wraps to at most three lines of about eleven
  characters and then truncates with `...`: thirty characters of long tokens fitted and thirty-three
  did not, while forty-eight characters of short words fitted, so the limit is line count rather
  than a character count. A `<br>` inside a decision is dropped as well. Keep a decision to a few
  short words, put the rest in the page's prose, and remember that a rectangle wraps on its own and
  honours `<br>`.
- **A clean exit does not mean a clean label.** Grep the rendered SVG for `...` after every render;
  a truncated node still exits `0`.
- **A long unbroken token in an edge label is hyphenated mid-word.** `claim(resource, ttl)` renders
  `claim(resourc-`, and `evaluate(expression)` renders `evaluate(expr-ession)`;
  `claim (resource, ttl)`, with a space before the parenthesis, renders whole.
- **A note is not a legend.** The class annotation is the legend; a note inside a drawing is prose
  the reader skips.
- **A theme chosen per diagram is a set with no palette.** Colours come from the frontmatter theme,
  so a hand-picked hex value is the one diagram that disagrees.

## Claims and how to test them

| Claim                             | Test                                                                                                     |
| --------------------------------- | -------------------------------------------------------------------------------------------------------- |
| A flowchart renders headlessly    | `line9 render <file>.mmd --out <file>.svg` exits `0` and the SVG is non-empty                            |
| A sequence diagram does not       | the same command on a `sequenceDiagram` exits `9` and the target path does not exist                     |
| Every source has its sibling      | `for f in Documentation/Diagram/*/*.mmd; do test -f "${f%.mmd}.svg" \|\| echo "$f"; done` prints nothing |
| No rendered label is clipped      | `grep -l '\.\.\.' Documentation/Diagram/*/*.svg` prints nothing                                          |
| Every folder is in the index      | every directory under `Documentation/Diagram/` appears in `Index.md`                                     |
| The contract is read, not guessed | `line9 capabilities --json` prints one JSON object carrying `exit_codes`                                 |

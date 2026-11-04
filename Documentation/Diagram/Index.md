# Diagrams

One folder per category. Each folder holds a page, the `.mmd` source of every diagram on that page,
and the `.svg` rendered from that source - so the set reads without the CLI, and a change is made in
a file that renders the same way twice.

A page is a storyboard: pages separated by a rule, the first page what exists, the last page the
states nobody has built. That is deliberate - a diagram that shows only what exists is a
description, and the useful part of a diagram is the gap between it and the goal.

## The convention

A diagram is a `.mmd` file with frontmatter and typed nodes:

```mmd
---
title: <what this diagram shows>
line9:
  theme: boardroom
---
flowchart LR
  A["a runtime surface"]:::host
  B[("persisted state")]:::store
  C["a refusal"]:::fail
  A ==> B
  B -.-> C
```

`line9 render <file>.mmd --out <file>.svg` writes the sibling. It serves flowcharts only: a valid
`stateDiagram-v2` or `sequenceDiagram` exits `9` and writes nothing, so those stay fenced blocks in
the page, with a sentence saying why.

The legend is four class annotations and nothing else - no `note`, no subgraph of prose inside a
drawing:

| Annotation | Means                                          | Shape                       |
| ---------- | ---------------------------------------------- | --------------------------- |
| `:::host`  | an existing runtime surface                    | `["…"]`, `(["…"])` an event |
| `:::hook`  | an installable step, including a missing state | `["…"]`                     |
| `:::store` | persisted state                                | `[("…")]`                   |
| `:::fail`  | an error or refusal                            | `["…"]`                     |

Shapes carry kind: `(["event"])`, `[("store")]`, `{"decision"}`, `["process"]`. Arrows are graded:
`==>` primary, `-->` secondary, `-.->` optional. Line breaks are `<br>`.

## The folders

| Folder                                  | Category    | What it draws                                                |
| --------------------------------------- | ----------- | ------------------------------------------------------------ |
| [`00-rings/`](00-rings/README.md)       | authority   | the crates as concentric rings, and the ring that is missing |
| [`10-runtime/`](10-runtime/README.md)   | leases      | claim, admit, land, refuse - and the late arrival            |
| [`20-browser/`](20-browser/README.md)   | effects     | attach versus launch, and the dead-but-open socket           |
| [`30-chat/`](30-chat/README.md)         | research    | mode, model, completion, truncation                          |
| [`40-database/`](40-database/README.md) | internality | generations, leases, probes, and the abandoned holder        |
| [`50-feed/`](50-feed/README.md)         | transport   | bundle, attach, observe, and the rate limit                  |

`Crate/Agentic/Test/Legend.rs` checks the set: frontmatter with a theme, at least one class
annotation, a rendered sibling over 1000 bytes, and no prose legend anywhere. The check does not
prove a diagram is right - only that it renders and that its terms are typed.

# Frontmatter

Every field here is read by something. A field nothing reads is decoration, and decoration in a
frontmatter block is the first thing that drifts out of date.

## The repo template

```yaml
---
name: browser-connect
description: Use when reaching a running browser over CDP from Rust. Attach, bind a tab, launch.
version: 1.0.0
license: MIT
platforms: [macos, linux]
metadata:
    hermes:
        tags: [cdp, rust, chromiumoxide, attach, target]
        related_skills: [browser-eval, bounded-calls]
---
```

| Field             | Read by                     | Rule                                                              |
| ----------------- | --------------------------- | ----------------------------------------------------------------- |
| `name`            | the corpus validator        | the directory lowercased; a mismatch is `unparsable`              |
| `description`     | the validator, and a reader | one line, non-empty, under the 100-column print width             |
| `version`         | a reader, and the diff      | bump it when the procedure changes, not when a typo does          |
| `license`         | a reader                    | `MIT` on every skill here, so a copy keeps the corpus consistent  |
| `platforms`       | a reader deciding to try    | list only what the commands were exercised on                     |
| `metadata.hermes` | the loader's index          | `tags` for retrieval, `related_skills` for the next skill to load |

`description` is the only field a reader sees before opening the file, so it carries the trigger
first: the situation, then what the skill does about it. Two sentences is the whole budget.

## A portable template

A skill that ships inside a repository someone else clones cannot name an account, a machine or a
sibling checkout. The same block with the instance removed:

```yaml
---
name: <kebab-name>
description: Use when <trigger>. <what the skill settles>.
version: 1.0.0
license: MIT
platforms: [macos, linux]
metadata:
    hermes:
        tags: [<three to five lowercase tags>]
---
```

A stand-in is a name the reader's own environment supplies: `$HOME`, `<workspace>`, `<project-id>`,
`<debug-port>`. A path that only exists on the author's machine is a command nobody can run.

## What the parser sees

The reader is line-based and strict, which produces three failures that all look like a missing
skill:

- a block that is not closed by `---`, so everything after it is body text;
- a value containing `: ` and no quotes, which makes the block invalid YAML;
- a `description` wrapped onto a second line by a formatter, which reads as empty.

Quote any value holding `: `, keep every value on one line, and confirm with a parser rather than a
grep. A grep for `^name:` passes on all three.

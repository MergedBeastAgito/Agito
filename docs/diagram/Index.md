# Diagrams

One file per category, each a storyboard: pages separated by a rule, the first page what exists, the
last page the states nobody has built.

| File             | Category    | What it draws                                                |
| ---------------- | ----------- | ------------------------------------------------------------ |
| `00-Rings.md`    | authority   | the crates as concentric rings, and the ring that is missing |
| `10-Runtime.md`  | leases      | claim, admit, land, refuse - and the late arrival            |
| `20-Browser.md`  | effects     | attach versus launch, and the dead-but-open socket           |
| `30-Chat.md`     | research    | mode, model, completion, truncation                          |
| `40-Database.md` | internality | generations, leases, probes, and the abandoned holder        |
| `50-Feed.md`     | transport   | bundle, attach, observe, and the rate limit                  |

Every page ends with the states that are missing. That is deliberate: a diagram that shows only what
exists is a description, and the useful part of a diagram is the gap between it and the goal.

The Mermaid is not decoration and it is not tested by the build. If a block stops parsing, the page
is wrong - read it before trusting it.

# SDK fixture

A consumer of `agentic-sdk` and of nothing else this workspace ships. It builds a session, opens
`Page/Probe.html` and evaluates an expression: the whole flow the facade exists for, written with the
facade's names and no others.

```
cargo build                  # the facade alone; no other Agentic crate is named
cargo run                    # needs a browser; AGENTIC_BROWSER names one
cargo tree --depth 1         # agentic-sdk and tokio, and nothing else
```

`tokio` is the only other dependency. `agentic.browse` spawns the handler that keeps a page answering,
so its launcher requires a runtime, and the adapter states that. `serde_json` is not named either: the
evaluated value is read with `as_str` and `Display`, so the fixture never has to know what type
carries it.

This crate is its own workspace root, and deliberately not a member of the tree under `Crate/`. A
member would resolve its siblings through the workspace's own graph, which is exactly what a consumer
cannot do - the claim under test is that one dependency is enough.

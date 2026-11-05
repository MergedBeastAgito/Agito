# Rust Port

How the five packages become a Cargo workspace, in the house style of `github.com/PlayForm/Run`:
`Source/` with `Fn/` and `Struct/` trees, PascalCase modules, one `Library.rs` per crate, `Target/`
as the build directory.

**Source:** `.hermes/research/4ffde6d5.md` throughout. The house style is unverified: the capture
says the repository could not be retrieved and takes the description as specification (`:192-194`),
so nothing here should be read as a measurement of that repository.

## Crates

**Source:** `.hermes/research/4ffde6d5.md` §1 - the directory tree and the package and library names
(`:198-304`), and the decision that the five stay separate crates (`:183-190`, `:300-303`). The
capture makes the workflow crate conditional on a real composition (`:183-186`); the rows below are
the full proposed set.

| Directory             | Package                       | Library                     | Responsibility                          |
| --------------------- | ----------------------------- | --------------------------- | --------------------------------------- |
| `BrowserCDP/`         | `agentic-browser-cdp`         | `AgenticBrowserCDP`         | concrete browser substrate              |
| `ChatDriver/`         | `agentic-chat-driver`         | `AgenticChatDriver`         | chat use case and its browse contract   |
| `DesignVerification/` | `agentic-design-verification` | `AgenticDesignVerification` | measurement and evidence                |
| `Lane/`               | `agentic-lane`                | `AgenticLane`               | template corpus and typed answers       |
| `SkillContract/`      | `agentic-skill-contract`      | `AgenticSkillContract`      | corpus discovery, validation, packaging |
| `Runtime/`            | `agentic-runtime`             | `AgenticRuntime`            | execution policy and resource ownership |
| `Workflow/`           | `agentic-workflow`            | `AgenticWorkflow`           | adapter-independent compositions        |
| `Agentic/`            | `agentic`                     | `Agentic`                   | wiring and executable entry points      |

The five existing packages do **not** become modules of one crate. Collapsing them would turn the
import-direction rule into a convention inside a single compiler boundary, which is the thing the
rule exists to prevent. Their internal operations - attach/launch/dispose, ask/verify/read,
discover/validate/package - stay modules.

The table is the capture's proposed set, not the tree's current inventory. `Adapter/`,
`SpacetimeCatalog/`, `Lane/` and `Sdk/` were added after the capture and answer needs it did not
reach: the port shape a substrate outside this tree implements, the database model as data, the
template corpus a decision-model call reads its questions from, and the facade a consumer outside
this workspace names. `Crate/README.md` is the inventory and the statement of each dependency list;
where the two disagree, that file is the current one.

## Dependencies

**Source:** `.hermes/research/4ffde6d5.md` §1's dependency graph (`:305-330`), and its reading of
`Runtime` as a policy engine rather than a registry (`:330-331`).

```
Agentic  → Runtime, BrowserCDP, ChatDriver, DesignVerification, SkillContract, Workflow
Workflow → ChatDriver, DesignVerification, SkillContract
Lane     → Runtime, serde_json
BrowserCDP, ChatDriver, DesignVerification, SkillContract, Lane, Runtime → no domain crate
```

`Runtime` is a policy engine, not a registry: it does not import every capability. The root
registers capability invocations and cleanup operations with it.

`Lane` is not in the capture's graph - it was added to the tree afterwards, so its edges are read
from `Crate/Lane/Cargo.toml` rather than from the capture - and it has the same shape as the other
capabilities: it names the runtime for the `Call` a poster is bounded by and the `FutureBox` its
port returns, the serialiser for the corpus, and nothing else. The round trip leaves through a
`Poster` the crate declares and never implements, so `Lane` opens no socket and the composition root
supplies the one that does.

## "Ports belong to their consumers", in Rust

**Source:** `.hermes/research/4ffde6d5.md` §1 (`:333-375`) - the two consumer-owned traits, the
root-owned wrapper, and the warning that a universal request type quietly becomes the ports crate.
Whether the two traits actually differ is `Documentation/Open-Questions.md` Q22.

There is no shared `ports` crate. There are two distinct traits with the same name in different
crates:

```rust
AgenticChatDriver::Struct::Port::Browse::Browse
AgenticDesignVerification::Struct::Port::Browse::Browse
```

They need not have identical methods. Chat may require turn observation; design verification may
require screenshots and layout measurements. Neither grows operations merely because the other needs
them.

`BrowserCDP` exports concrete substrate operations using its own types, and the composition root
owns a wrapper implementing both consumer traits:

```rust
// Agentic/Source/Struct/Binding/Browser.rs
pub struct Browser { /* holds the CDP client */ }
```

That wrapper is the only place that knows both sides, which is exactly what a composition root is
for.

## Module shape per crate

**Source:** `.hermes/research/4ffde6d5.md` §1 for the module paths (`:198-304`) and the house-style
`.cargo/config.toml` values (`:63-65`, `:770-773`). One crate is reproduced here; the capture gives
the tree for all seven.

```
ChatDriver/
  Cargo.toml
  module.json
  Source/
    Library.rs
    Fn.rs
    Struct.rs
    Fn/Ask.rs
    Fn/Verify.rs
    Struct/Port/Browse.rs
    Struct/Call.rs
    Struct/Turn.rs
    Struct/Error.rs
```

`.cargo/config.toml` sets `build.target-dir = "Target"`, and the release profile follows the house
style: `opt-level 3`, `codegen-units 1`, `debug false`, `lto true`, `panic "abort"`, `strip true`.

## Still open

**Source:** `.hermes/research/4ffde6d5.md` - the list below is the set of questions the request
asked, and the capture answers every one: 1 (`:376-450`), 2 (`:451-493`), 3 (`:494-589`), 4
(`:590-696`) and 5 (`:697-764`). What stays open is not the capture's answer but this repository's
implementation of it, and each is tracked with its experiment and blocker in
`Documentation/Open-Questions.md` Q18-Q24.

These were asked, and the capture answers them. What is left is to hold this tree to each answer:

1. **CDP in Rust or a TypeScript sidecar.** `chromiumoxide`, `headless_chrome`, or raw CDP over a
   websocket, against keeping a small TS process driven over stdio JSON-RPC. Each has different
   failure modes around connection loss and event ordering.
2. **The manifest in Rust.** A build script generating static data, a serde-loaded file per crate,
   or const data in the crate. The constraint is that a harness must read it without building
   anything at launch.
3. **Cancellation and budgets.** The trait shape for a port with a deadline that does not leak
   `tokio` into every capability.
4. **Enforcing the arrows.** Cargo permits any dependency direction; the check has to be a
   workspace-level CI step over the dependency graph.
5. **Manifest field mapping.** How `state`, `cost` and `artefact.hash` survive without losing what
   the pre-warm planner needs.

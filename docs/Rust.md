# Rust Port

How the five packages become a Cargo workspace, in the house style of `github.com/PlayForm/Run`:
`Source/` with `Fn/` and `Struct/` trees, PascalCase modules, one `Library.rs` per crate, `Target/`
as the build directory.

**Source:** `.hermes/research/rust-4ffde6d5.md` throughout. The house style is unverified: the
capture says the repository could not be retrieved and takes the description as specification
(`:60`), so nothing here should be read as a measurement of that repository.

## Crates

**Source:** `.hermes/research/rust-4ffde6d5.md` §1 - the directory tree, the package and library
names, and the decision that the five stay separate crates (`:56-58`, `:152-161`). The capture makes
the workflow crate conditional on a real composition (`:133`); the rows below are the full proposed
set.

| Directory             | Package                       | Library                     | Responsibility                          |
| --------------------- | ----------------------------- | --------------------------- | --------------------------------------- |
| `BrowserCDP/`         | `agentic-browser-cdp`         | `AgenticBrowserCDP`         | concrete browser substrate              |
| `ChatDriver/`         | `agentic-chat-driver`         | `AgenticChatDriver`         | chat use case and its browse contract   |
| `DesignVerification/` | `agentic-design-verification` | `AgenticDesignVerification` | measurement and evidence                |
| `SkillContract/`      | `agentic-skill-contract`      | `AgenticSkillContract`      | corpus discovery, validation, packaging |
| `Runtime/`            | `agentic-runtime`             | `AgenticRuntime`            | execution policy and resource ownership |
| `Workflow/`           | `agentic-workflow`            | `AgenticWorkflow`           | adapter-independent compositions        |
| `Agentic/`            | `agentic`                     | `Agentic`                   | wiring and executable entry points      |

The five existing packages do **not** become modules of one crate. Collapsing them would turn the
import-direction rule into a convention inside a single compiler boundary, which is the thing the
rule exists to prevent. Their internal operations - attach/launch/dispose, ask/verify/read,
discover/validate/package - stay modules.

## Dependencies

**Source:** `.hermes/research/rust-4ffde6d5.md` §1's dependency graph, and its reading of `Runtime`
as a policy engine rather than a registry (`:163-187`).

```
Agentic  → Runtime, BrowserCDP, ChatDriver, DesignVerification, SkillContract, Workflow
Workflow → ChatDriver, DesignVerification, SkillContract
BrowserCDP, ChatDriver, DesignVerification, SkillContract, Runtime → no domain crate
```

`Runtime` is a policy engine, not a registry: it does not import every capability. The root
registers capability invocations and cleanup operations with it.

## "Ports belong to their consumers", in Rust

**Source:** `.hermes/research/rust-4ffde6d5.md` §1 (`:189-218`) - the two consumer-owned traits, the
root-owned wrapper, and the warning that a universal request type quietly becomes the ports crate.
Whether the two traits actually differ is `docs/Open-Questions.md` Q22.

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

**Source:** `.hermes/research/rust-4ffde6d5.md` §1 for the module paths (`:67-81`) and the
house-style `.cargo/config.toml` values (`:25`, `:150`). One crate is reproduced here; the capture
gives the tree for all seven.

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

**Source:** `.hermes/research/rust-4ffde6d5.md` - and the list below is stale against it. The
capture answers 1 (`:220-281`), 2 (`:283-312`) and 3 (`:314-383`); it asserts 4 (`:58`) without
specifying the check; it never reaches 5, because the answer is cut off at `:378`. The questions
stay here as they were asked, and each is tracked with its experiment and blocker in
`docs/Open-Questions.md` Q18-Q24.

These were asked. The capture answered the first three and left the rest below; nothing here should
be built on a guess:

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

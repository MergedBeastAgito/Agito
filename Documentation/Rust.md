# Rust Workspace

The layout of the Cargo workspace: `Source/` with `Fn/` and `Struct/` trees, PascalCase modules, one
`Library.rs` per crate, `Target/` as the build directory. The style is `github.com/PlayForm/Run`'s
as described rather than as measured - nothing in this tree reads that repository - so the names
below are the convention this workspace keeps, not a quotation of another one.

## Crates

| Directory              | Package                     | Library        | Responsibility                               |
| ---------------------- | --------------------------- | -------------- | -------------------------------------------- |
| `Adapter/`             | `agito-adapter`             | `Adapter`      | the port a third-party substrate implements  |
| `Agito/`               | `agito`                     | `Agito`        | wiring and executable entry points           |
| `Browser/CDP/`         | `agito-browser-cdp`         | `CDP`          | concrete browser substrate                   |
| `Chat/Driver/`         | `agito-chat-driver`         | `Driver`       | chat use case and its browse contract        |
| `Design/Verification/` | `agito-design-verification` | `Verification` | measurement and evidence                     |
| `Lane/`                | `agito-lane`                | `Lane`         | template corpus and typed answers            |
| `RunTime/`             | `agito-runtime`             | `RunTime`      | execution policy and resource ownership      |
| `SDK/`                 | `agito-sdk`                 | `SDK`          | the facade a consumer outside the tree names |
| `Skill/Contract/`      | `agito-skill-contract`      | `Contract`     | corpus discovery, validation, packaging      |
| `Spacetime/Catalog/`   | `agito-spacetime-catalog`   | `Catalog`      | the database model as rules and data         |
| `Workflow/`            | `agito-workflow`            | `Workflow`     | adapter-independent compositions             |

A directory that names two concepts is two directories: the group folder carries the object and the
crate folder carries the leaf, which is also the library's name. The package id stays one lowercase
kebab token because Cargo fixes that spelling.

The crates are separate packages, not modules of one crate. Collapsing them would turn the
import-direction rule into a convention inside a single compiler boundary, which is the thing the
rule exists to prevent. Their internal operations - attach/launch/dispose, ask/verify/read,
discover/validate/package - stay modules.

## Dependencies

```
Agito             → RunTime, Browser/CDP, Chat/Driver, Design/Verification, Skill/Contract,
                    Workflow, Lane, Spacetime/Catalog, SDK
Workflow          → RunTime, Browser/CDP, Chat/Driver, Design/Verification, Skill/Contract
Lane              → RunTime, serde, serde_json
Adapter           → RunTime, serde_json
SDK               → the six ports: RunTime, Browser/CDP, Chat/Driver, Design/Verification,
                    Skill/Contract, Adapter
Spacetime/Catalog → nothing
Browser/CDP, Chat/Driver, Design/Verification, Skill/Contract, Lane, RunTime → no domain crate
```

`RunTime` is a policy engine, not a registry: it does not import every capability. The composition
root registers each capability's name and its state class with it.

`Lane` has the same shape as the other capabilities: it names the runtime for the `Call` a poster is
bounded by and the `FutureBox` its port returns, `serde` for the request body's derive and
`serde_json` for the corpus, and nothing else. The round trip leaves through a `Poster` the crate
declares and never implements, so `Lane` opens no socket and the composition root supplies the one
that does.

`Crate/README.md` states each crate's dependency list in prose; the `Cargo.toml` beside it is what
Cargo resolves.

## "Ports belong to their consumers", in Rust

There is no shared `ports` crate. There are two distinct traits in different crates:

```rust
Driver::Struct::Port::Page
Verification::Struct::Port::Browse
```

They do not have the same methods, and neither grows one because the other needs it. The chat port
needs a page it can open, evaluate in, deliver input to and release, and the target it hands back
unread. Design verification needs one call that reads the named elements of a page it was configured
with, and retains nothing.

`Browser/CDP` exports concrete substrate operations using its own types, and `agito-workflow` owns
the bindings that implement each consumer trait over them:

```rust
// Crate/Workflow/Source/Struct/Binding/Page.rs
pub struct Page { /* holds Arc<dyn CDP::Browse> */ }
```

That binding is the only place that knows both sides, which is what a workflow layer is for. It is
also where the translations the capability must not carry live: the browser target the driver passes
back without reading, and the mapping from the browser's three codes onto the chat port's four.

## Module shape per crate

```
Crate/Chat/Driver/
  Cargo.toml
  build.rs
  module.json
  Source/
    Library.rs
    Fn.rs
    Struct.rs
    Fn/Expression.rs
    Fn/Note.rs
    Fn/Read.rs
    Struct/Driver.rs
    Struct/Error.rs
    Struct/Format.rs
    Struct/Note.rs
    Struct/Port.rs
    Struct/Selectors.rs
    Struct/Settlement.rs
    Script/Expression/ReadAnswers.ts
```

`.cargo/config.toml` sets `build.target-dir = "Target"`, and the release profile follows the house
style: `opt-level 3`, `codegen-units 1`, `debug false`, `lto true`, `panic "abort"`, `strip true`.

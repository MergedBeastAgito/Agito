# Adapter

The one port this workspace offers to a substrate it did not write, and what a third-party crate has
to say before the workspace will believe it.

## The gap it closes

Agentic ships capabilities and one concrete substrate. A third party with a different substrate -
another protocol, another device, a game port - could copy the browser crate's plumbing and be
consumed by path, and that is what happened first: a driver that required `agentic.browse`, provided
a port of its own, kept its JavaScript in `Source/Script/`, and had nowhere to say so. Nothing
outside the workspace could state "I provide this port" and be believed, because the manifest table
is compiled in and `Crate/*` is what it describes.

`Crate/Adapter` (`agentic-adapter`) is that missing statement. It holds one trait, one check, and a
stand-in; it holds no behaviour.

---

## The trait

```rust
pub trait Adapter: Send + Sync {
    /// The port this adapter provides, as its own `module.json` names it.
    fn Port(&self) -> &'static str;
    /// One request, bounded by the caller's deadline.
    fn Call<'a>(&'a self, Request:&'a str, Call:Call) -> FutureBox<'a, Result<Value, Error>>;
    /// Release everything this adapter holds. Never fails.
    fn Dispose<'a>(&'a self) -> FutureBox<'a, ()>;
}
```

- **Every method takes `&self`.** A trait method without a receiver cannot be called on
  `dyn Adapter`, so a trait shaped that way is not object-safe and cannot be held as
  `Arc<dyn Adapter>` - which is the only reason the trait exists. The first draft of this contract
  had `fn Port() -> &'static str` and was wrong for exactly that reason.
- **`Dispose` never fails.** An adapter that owns a session, a socket or a device has to be able to
  let go of it, and a release that can fail is a release a caller skips. The browser crate states
  the same rule for disposal.
- **The request is text.** It is the adapter's own encoding, JSON by convention and nothing more. A
  universal request type every adapter shares would be the ports crate this workspace's design
  refuses: the first substrate with a shape of its own would force an edit on every other adapter.
- **The error is the adapter's own.** `Error` carries a `Code` - the word the adapter's own manifest
  declares - plus a `Message` and what was observed. Agentic does not enumerate the failures of a
  substrate it has never seen, which is why `Crate/Adapter`'s own manifest names no errors.

---

## No registry

A registry is a global, and a global makes load order observable: two adapters claiming one port are
resolved by whoever registered first, and the answer to "which one is running" depends on a side
effect nobody wrote down.

The composition root constructs the adapter once, holds it as `Arc<dyn Adapter>`, and passes it on -
the same shape the browser port already has. With no registry there is nobody to ask "who serves
this port", and that is the point: the root constructed the value, so it already knows. What it
cannot know from the value alone is whether the value agrees with the manifest a harness read, and
that is what `AgenticAdapter::Serves(&dyn Adapter, Port)` answers - at the one place both halves are
in hand.

---

## The manifest kind

`kind: "adapter"` is the whole new vocabulary. Everything else is the manifest this workspace
already reads:

```json
{
	"name": "@someone/thing",
	"kind": "adapter",
	"version": "0.1.0",
	"provides": { "port": "agentic.thing.drive", "errors": ["timeout"] },
	"requires": { "ports": ["agentic.browse"] },
	"artefact": { "entry": "Target/release/thing", "hash": null, "runtime": "rust>=1.88" },
	"state": "holds-transport",
	"cost": { "coldMs": 40, "warmMs": 3, "network": false },
	"verify": "cargo run --release"
}
```

The reader accepts the six kinds the boundary rule knows - `runtime`, `capability`, `adapter`,
`workflow`, `composition`, `facade` - and refuses anything else by name, listing the vocabulary
rather than only the offending word. A file that names no module, no port, or an unknown kind is
refused where it is read, because a listing that shows a module the verifier cannot reason about is
worse than a listing that says why it would not.

---

## How a module is found

Nothing is compiled into Agentic, and nothing is registered. Two verbs read a third-party
`module.json`:

- `agentic manifests --adapter <dir>` lists every module under the given roots after the ones the
  workspace ships, with the directory each was read from in the fifth column.
- `agentic verify --module <name> --adapter <dir>` reports readiness for one of them. The command
  line is the same one the workspace's own modules use.

`--adapter` is repeatable, and `AGENTIC_ADAPTER_PATH` - a path list, split the way `PATH` is -
supplies the roots when the flag is absent. That is what lets a consumer run
`agentic verify --module @someone/thing` with no flag at all.

A root is either a crate directory (it has a `module.json`) or a directory of crate directories. Two
levels and no deeper: a tool that walks further than it was told is a tool whose listing changes
when an unrelated directory appears.

A root that does not exist is skipped with a warning on stderr, and a `module.json` that exists but
does not parse is a refusal. The asymmetry is deliberate: absence is not a claim, so a stale entry
in a path list must not break the workspace's own listing, while a file that was found and is wrong
is exactly what a reader must not pass over.

Two modules declaring one name - or one module claiming a name the workspace already ships - is
refused, because a caller asking for that name would get one of two answers and no way to tell
which.

---

## What `verify` can and cannot say

A module the binary carries is checked three ways: the table against the file, the file against the
crate's own `Code` type, and the state against the four classes. A third-party adapter has no code
here to compare against, and a readiness report that pretended otherwise would be the one thing it
must not be. So the foreign report checks what is knowable:

- the kind is one the reader accepts, and the state one of the four classes;
- `provides.errors` is a list of distinct, non-empty words;
- `artefact.entry` is named;
- every port in `requires.ports` is a port this workspace or another module under the same roots
  provides - an adapter that requires something nobody offers is not ready, however well formed its
  file is.

`errors.code` is `null` in that report rather than a copy of the manifest's own list, because a copy
would read as agreement. What the report cannot answer, `Serves` can, at the composition root.

`agentic verify` never constructs an adapter. A command that did would run a third party's code to
decide whether it is well formed.

---

## What is deliberately not here

- **No dynamic loading.** Agentic has no `dlopen`, no plugin directory and no symbol lookup: a
  third-party adapter is a Cargo dependency of its own consumer, not of this workspace.
- **No registry, and therefore no discovery of running adapters.** The manifest listing is a
  statement about files, not about processes.
- **No universal request type.** The trait carries text; each adapter decides what its own text
  means.
- **No second substrate.** `Crate/Adapter` ships no implementation. The workspace already has one,
  and a second would be a second thing to keep working.

---

## Claim to test

| Claim                                                                        | Where it is checked                                                  |
| ---------------------------------------------------------------------------- | -------------------------------------------------------------------- |
| The trait is object-safe and usable as `Arc<dyn Adapter>`                    | `Crate/Adapter/Test/Adapter.rs`                                      |
| A foreign implementation satisfies the trait                                 | `Crate/Adapter/Test/Adapter.rs`                                      |
| The port a value reports matches the manifest that named it                  | `Crate/Adapter/Test/Adapter.rs`, `Crate/Agentic/Test/Composition.rs` |
| An adapter under a root is listed with its port and its directory            | `Crate/Agentic/Test/Composition.rs`                                  |
| An adapter under a root verifies ready, with `code: null`                    | `Crate/Agentic/Test/Composition.rs`                                  |
| An unknown kind, a duplicate name and an unsatisfied requirement are refused | `Crate/Agentic/Test/Composition.rs`                                  |
| A stale root is skipped and the listing survives                             | `Crate/Agentic/Test/Composition.rs`                                  |
| `adapter` is a kind the boundary rule knows                                  | `Crate/Agentic/Test/Boundaries.rs`                                   |

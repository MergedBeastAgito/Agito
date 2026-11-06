# SpacetimeDB Catalog

The agito database model as rules and data: capabilities, generations, leases with a fence, probes,
beliefs and trace. It lives here rather than in the consuming repository because none of it needs
SpacetimeDB to be true, and a module that imported the server SDK to hold it could not be tested
without a published database.

This document says what the catalog is, which invariant each rule enforces, and what TimeLine takes
to adopt it in one step. The statement of the model is `Crate/Spacetime/Catalog/`.

## What this crate is

- **`Source/Struct/Catalog.rs` is the catalog** - six tables described as data: name, columns with a
  type and a key, and whether a table is append-only. A module's `Schema.ts` renders its
  `table(...)` declarations from this, and a reader diffs two versions of it to see what a
  generation changed.
- **`Source/Struct/Context.rs` is the seam** - the whole of what a rule is given: who is calling,
  what time it is, and a way to read, insert, update and delete one row by key. Six members, and no
  more, because everything else on a module's `ctx` is a way to escape the rules rather than to
  state them. A context over a map is the whole instrument a rule needs: it records every write,
  refuses to rewrite a table the catalog marks append-only, and checks every row against the
  catalog's columns and types.
- **`Source/Fn/` is the behaviour** - `bind`, `claim`, `renew`, `record`, `resolve`, `probe` and
  `recordTrace` as pure functions over `Context`. Each answers a result rather than throwing, so a
  refusal is a value a caller handles and a test asserts.

The seam is the point. `spacetimedb` is not a dependency of this crate - it has no dependency at
all, which `cargo tree -p agito-spacetime-catalog` answers in one line - and adding one to test
rules would be backwards: the rules are what must be true of the module, so they have to be
checkable before a module exists.

## The six tables

- **`capability`** - keyed by `<name>@<version>`, append-only. What a manifest may name. Rows are
  declared by the module's `init`, from its own package manifests; there is no reducer that declares
  one, because a declaration is a deployment.
- **`generation`** - keyed by a sequence number as a decimal string, append-only. What was bound, by
  whom, at what time, and which capabilities the manifest named. A generation is superseded by the
  existence of its successor, never by a column someone rewrote.
- **`lease`** - keyed by the resource id, current state. Who holds what, under which generation, at
  which fence, until when. Never deleted: deleting the row would erase the fence, and a fence that
  restarts at one is a fence a late worker can satisfy.
- **`probe`** - keyed by `<property>:<version>:<audience>`, append-only. What a specific audience
  learned about a specific property version, and when. It has no column for the private value, and
  no rule is ever given it.
- **`belief`** - keyed by `<audience>:<property>:<version>`, disposable. What an audience inferred
  from the observations it holds. A belief grants no access, which is why it may carry an
  interpretation while `probe` may not carry the truth.
- **`trace`** - keyed by `<at>:<kind>:<subject>`, append-only. What happened, keyed by content so a
  replay appends nothing. Evidence, not authority.

## The five invariants

### Coherence - one bound generation, and no writes under an old one

`bind` appends a generation and refuses a manifest that names a capability the catalog does not
declare. Every other rule refuses to write under a generation that is not the latest bound one.

The mechanism is worth stating, because it looks like a trick and is a consequence. Generation ids
are dense sequence numbers, and `bind` computes a successor's id as its parent's sequence plus one.
So "is this generation the latest?" is answered by one key lookup - does `sequence + 1` exist - in a
context that finds by key and cannot scan. The same property is what leaves the predecessor
byte-identical: a bind inserts a row and updates nothing, and the catalog marks the table
append-only so a context cannot quietly rewrite it.

A bind names its predecessor and that predecessor must still be the latest, so two binds of the same
parent are a compare-and-swap rather than a fork: in a serialised database the second sees the first
one's successor and is refused.

### Fence - a late worker cannot write

`claim` increments the fence. `renew`, `record` and `probe` refuse a fence that is not the lease's
current one, and refuse without writing, so a worker whose lease expired cannot write as though it
still held it. The holder is `ctx.sender`, so a lease is renewed only by the sender that holds it.

Expiry is a comparison, not a stored transition: a row that says `held` but whose deadline has
passed is expired, because no timer has to have run for that to be true. An expired lease is
re-claimed, which increments the fence - renewal extends a lease and never resurrects one. The row
is never deleted, so the fence never restarts at one.

A `ttl` is refused rather than allowed to wrap: `claim` and `renew` refuse one that is not positive,
and a deadline the clock cannot reach - `now + ttl` past the top of the integer - is
`admission-refused`, not a lease that expires in the past.

### Disclosure - one row, to one audience, at one version

`probe` writes exactly one audience-scoped row and returns only that row. The private value is never
an argument, so it is never a column: a reader cannot select what the database was never told.

The row id is `<property>:<version>:<audience>`, which makes the reveal its own receipt and its own
allowance. A second probe by the same audience at the same version finds the row and is refused as
`budget-exhausted`, so no second row has to record a charge, and two racing probes cannot both spend
a one-use allowance - the database serialises them and the loser finds the winner's row.

The audience is `ctx.sender`, never an argument: a caller-supplied audience would be a
caller-supplied identity, and the recipient of a reveal is not something the caller names.

### Time - one clock, and it is the context's

Every timestamp a row carries is `ctx.now`. No function reads a wall clock, and the lags between
times are differences the caller computes, because a reducer that read the system clock could not be
replayed and a test could not decide what time it is.

### Handover - a refusal is not an undo

`record` is where an effect is checked when it lands, and it is the guarantee: `claim` refuses work
whose authority had already ended, but it ran before the effect did anything and cannot say anything
about the instant it writes. Only the place the record is presented to can.

An effect that was admitted under an epoch and refused when it lands may have changed the world.
Rejecting the result does not undo the effect, so the refusal is written as an `effect-ambiguous`
trace row and the lease is moved to `quarantined`: the write is not counted as a success, and
`claim` will not reassign the resource. `resolve` states whether the external action landed or was
absent, appends `effect-resolved`, and only then releases the row for reassignment. The lease row is
never deleted, so the fence survives and the next claim advances past it.

A refusal is ambiguous only when the record's identity matched the current epoch - generation, lease
generation, fence and holder. A superseded fence is refused without an ambiguity, because the row no
longer describes that epoch and the claim that superseded it is the accounting point for the effects
it had in flight. The holder is `ctx.sender` rather than an argument, so a caller that reproduces
every number in the row is still not the holder.

## Adopting it in TimeLine, in one step

One deliberate step, five parts. The rules are the crate's; a module renders them and satisfies the
seam.

1. **Render `Schema.ts` from the catalog.** Each table becomes one `table(...)` declaration, each
   column one `t.*` call. The mapping is mechanical: `string` is `t.string()`, `number` is
   `t.u64()`, `boolean` is `t.bool()`, `json` is `t.string()` holding the document's text,
   `optional-string` is `t.string().optional()`, `optional-number` is `t.u64().optional()`, a
   `primary` key takes `.primaryKey()` and an `index` key `.index()`. The catalog is the six records
   in `Source/Struct/Catalog.rs`, and a generator that reads them is one build step rather than a
   second statement of the tables.
2. **Restate the seven rules in the module's language.** A module cannot link a Rust crate, so the
   rules travel as source: `Source/Fn/` is what the restatement is compared against, case for case,
   and `Context` is the shape the module's own fake has to satisfy.
3. **Write the adapter that satisfies `Context`.** This is the only new code TimeLine writes, and it
   is the six members below.
4. **Seed `capability` in `init`** from the package manifests TimeLine already has, so the first
   `bind` has something to check against.
5. **Register the seven rules as reducers**, with their arguments as the `t.*` declarations the
   module already uses for `observe` and `record`.

### The declarations the rules are stated in

From `Source/Struct/Catalog.rs`: `ColumnType`, `Key`, `Column`, `Table`, `TableName`, `Tables`,
`TableNames`, `PrimaryKey`.

From `Source/Struct/Context.rs`: `Value`, `Row`, `Context`.

From `Source/Struct/`: `Manifest`, `Bind`, `Claim`, `Renew`, `Effect`, `Resolve`, `Probe`, `Trace`,
`Answer`, `ProbeAnswer`, `Code`.

From `Source/Fn/`: `bind`, `claim`, `renew`, `record`, `resolve`, `probe`, `recordTrace`, and
`Latest` - the coherence predicate, which a view that lists live generations needs for the same
reason a rule does.

That is all of the crate's public surface. Nothing is adopted selectively, because a rule that
travelled without its table description would be a rule checked against a different catalog than the
module renders.

### The adapter, which is the only code TimeLine writes

- **`Sender`** - `ctx.sender`, the module's `Identity`, rendered to a string once at the boundary.
  Every rule compares that rendering, so a rule never compares an identity to a caller-supplied
  argument.
- **`Now`** - `ctx.timestamp` in whatever integer unit the module uses. The rules only add, compare
  and store it, so the unit is TimeLine's choice; a `ttl` passed to `claim` or `renew` has to be in
  that same unit. A `u64` column is a `bigint` in the module's TypeScript, so the adapter converts
  at the boundary and the rules stay on a signed integer.
- **`Find(table, id)`** - `ctx.db[table].id.find(id)`.
- **`Insert(table, row)`** - `ctx.db[table].insert(row)`.
- **`Update(table, row)`** - `ctx.db[table].id.update(row)`.
- **`Delete(table, id)`** - `ctx.db[table].id.delete(id)`. No rule deletes a row; the member exists
  because the context has it.

The three writes are infallible in the crate, and the adapter is where that becomes true: a row
already at its key or an update of an append-only table is a defect the adapter must not paper over,
because turning it into a refusal would report a caller's mistake for a corruption.

### The one thing that cannot be copied

The module's own `spacetimedb` import: `schema()`, `table()`, `reducer()`, `ctx.db`, `t.*`, `init`
and the client callbacks. Those exist only inside a published module - there is no `ctx.db` before a
module is compiled and no `table()` without the server package - so a rule that named them could not
be tested without a database, and would pin the rules to one server version.

That is the whole of what stays behind in TimeLine: `Schema.ts`, the `Context` adapter, `init`, and
the seven `spacetimedb.reducer(...)` registrations. The rules, the catalog and the interface travel
unchanged.

## What is deliberately absent

- **No reducer declares a capability.** A declaration is a deployment, and a reducer that could add
  one would let a caller widen the set its own manifest is checked against.
- **No belief writer.** Deriving a belief is the actor's own act on observations it already holds;
  the table is described because its shape is the third thing the disclosure rule needs, and
  deleting a belief loses nothing.
- **No `release` reducer.** These seven rules never end a lease early; expiry is the recovery
  mechanism and every write checks it. `lease.state` is read by every fenced write, so a module that
  wants early release adds `release` and the rules already treat `released` as claimable.
- **No scan.** Every question a rule asks is one key lookup, which is why generation ids are dense
  sequence numbers and why a probe's key is its receipt. A rule that needed `iter()` would be a rule
  that cannot be stated here.
- **No wall clock, no credentials, no private values.** The catalog describes what is stored; a
  secret the rules are never given is not merely filtered from a reader, it never enters the
  database.

## What holds it

`Crate/Spacetime/Catalog/` is the executable statement of the model, and three checks answer for it.

- `agito verify --module @agito/spacetime-catalog` reports ready: the manifest's `Provide.Error` is
  `admission-refused`, `lease-held` and `budget-exhausted`, which is exactly `Code` in
  `Source/Struct/Code.rs`.
- `cargo tree -p agito-spacetime-catalog --depth 1` shows the crate alone - it is a capability, it
  imports nothing, and only the composition root imports it.
- `Crate/RunTime/Source/Struct/StandIn.rs` maps `record`'s checks column for column - the latest
  generation, the lease row, the fence, the holder, the released and quarantined states, the
  expiry - so the runtime's local stand-in and this crate are two implementations of one answer. If
  they ever disagree, one of them is wrong and the other is a second authority.

# SpacetimeDB Catalog

The agentic database model as rules and data: capabilities, generations, leases with a fence,
probes, beliefs and trace. It lives here rather than in the consuming repository because none of it
needs SpacetimeDB to be true, and a module that imported the server SDK to hold it could not be
tested without a published database.

This is an adoption note. It says what the catalog is, which invariant each rule enforces, and
exactly what TimeLine takes to adopt it in one step. The statement of the model is
`Crate/SpacetimeCatalog/`; this document is the prose that says what the crate states, and it
records where the two disagreed when the port was made.

## What the port changed in this document

The rules below were stated here first and in a TypeScript package second, and that package was
deleted with the rest of the layer. Porting it into a crate found four things this document had
wrong, and they are recorded rather than quietly corrected:

- **The rule list named five.** It said `bind`, `claim`, `renew`, `probe` and `recordTrace`, and
  that this was "all of both files". The source had seven: `record` and `resolve` were missing, with
  the argument types `Effect` and `Resolve`. They are the handover rule, and they are in the crate.
- **The invariant list named four.** The source documents five, and the missing one is handover -
  the invariant the destination exists for. Restored below.
- **The file names were the deleted package's.** They now name the crate's modules.
- **The case count was 25.** `Tables.test.ts`, `Reducers.test.ts` and `Destination.test.ts` hold 7,
  18 and 5 tests: 30, not 25.

Nothing in the rules themselves was changed by the port. Where the crate answers differently it is
said so in place, and there is one such place: a `ttl` the clock cannot reach is refused rather than
allowed to wrap.

## What this crate is

- **`Source/Struct/Catalog.rs` is the catalog** - six tables described as data: name, columns with a
  type and a key, and whether a table is append-only. A module's `Schema.ts` renders its
  `table(...)` declarations from this, and a reader diffs two versions of it to see what a
  generation changed.
- **`Source/Struct/Context.rs` is the seam** - the whole of what a rule is given: who is calling,
  what time it is, and a way to read, insert, update and delete one row by key. Five members, and no
  more, because everything else on a module's `ctx` is a way to escape the rules rather than to
  state them.
- **`Source/Fn/` is the behaviour** - `bind`, `claim`, `renew`, `record`, `resolve`, `probe` and
  `recordTrace` as pure functions over `Context`. Each answers a result rather than throwing, so a
  refusal is a value a caller handles and a test asserts.
- **`Test/Fake.rs` is the instrument** - an in-memory context that records every write, refuses to
  rewrite a table the catalog marks append-only, and checks every row against the catalog's columns
  and types.

The seam is the point. `spacetimedb` is not a dependency of this crate - it has no dependency at
all, which `cargo tree -p agentic-spacetime-catalog` answers in one line - and adding one to test
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

Proved by: `bind_appends_the_first_generation_at_the_senders_time_and_writes_exactly_one_row`,
`bind_refuses_a_manifest_that_names_a_capability_the_catalog_does_not_declare_and_writes_nothing`,
`bind_appends_a_successor_and_leaves_the_previous_generation_untouched`,
`bind_refuses_a_predecessor_that_is_already_superseded_or_that_was_never_bound`, and
`a_claim_or_a_probe_under_a_superseded_generation_is_refused_and_writes_nothing`.

### Fence - a late worker cannot write

`claim` increments the fence. `renew`, `record` and `probe` refuse a fence that is not the lease's
current one, and refuse without writing, so a worker whose lease expired cannot write as though it
still held it.

Expiry is a comparison, not a stored transition: a row that says `held` but whose deadline has
passed is expired, because no timer has to have run for that to be true. An expired lease is
re-claimed, which increments the fence - renewal extends a lease and never resurrects one. The row
is never deleted, so the fence never restarts at one.

Proved by: `the_fence_a_claim_past_expiry_increments_it_and_the_first_fence_can_no_longer_renew`,
`a_claim_while_a_lease_is_live_is_lease_held_and_writes_nothing`,
`an_expired_lease_cannot_be_renewed_back_to_life`,
`a_lease_is_renewed_only_by_the_sender_that_holds_it`,
`renew_extends_a_live_lease_at_the_senders_time`, and
`a_probe_carrying_a_stale_fence_is_refused_and_writes_nothing`.

### Disclosure - one row, to one audience, at one version

`probe` writes exactly one audience-scoped row and returns only that row. The private value is never
an argument, so it is never a column: a reader cannot select what the database was never told.

The row id is `<property>:<version>:<audience>`, which makes the reveal its own receipt and its own
allowance. A second probe by the same audience at the same version finds the row and is refused as
`budget-exhausted`, so no second row has to record a charge, and two racing probes cannot both spend
a one-use allowance - the database serialises them and the loser finds the winner's row.

The audience is `ctx.sender`, never an argument: a caller-supplied audience would be a
caller-supplied identity, and the recipient of a reveal is not something the caller names.

Proved by:
`a_probe_writes_exactly_one_row_carrying_the_audience_and_the_property_version_not_the_private_value`,
`a_second_probe_by_the_same_audience_at_the_same_property_version_is_budget_exhausted_and_writes_nothing`,
`the_allowance_is_per_audience_so_another_holder_gets_its_own_observation`, and the catalog cases
`the_probe_table_has_no_column_that_could_hold_the_private_value` and
`no_table_in_the_catalog_describes_a_column_named_for_a_secret`.

### Time - one clock, and it is the context's

Every timestamp a row carries is `ctx.now`. No function reads a wall clock, and the lags between
times are differences the caller computes, because a reducer that read the system clock could not be
replayed and a test could not decide what time it is.

Proved by:
`every_timestamp_a_row_carries_is_the_contexts_now_so_a_fake_clock_decides_the_whole_transcript`.

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
it had in flight.

Proved by: `an_effect_that_outlives_its_lease_is_refused_where_it_lands_and_recorded_ambiguous`,
`a_caller_that_reproduces_the_rows_numbers_is_still_not_the_holder`,
`an_effect_from_a_superseded_fence_is_refused_without_an_ambiguity`,
`an_effect_whose_lease_was_released_while_it_ran_is_refused_and_recorded_ambiguous`, and
`an_effect_presented_while_its_lease_is_live_is_recorded_not_quarantined` - which is the transcript
`Crate/Runtime/Test/Runtime.rs` runs against the local stand-in, and the two must answer the same.

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
   rules travel as source: `Source/Fn/` is what the restatement is checked against, case for case,
   and `Test/Fake.rs` is the shape the module's own fake should have.
3. **Write the adapter that satisfies `Context`.** This is the only new code TimeLine writes, and it
   is the five members below.
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
unchanged, and the tests that prove them travel with them.

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

## What proves it

`Crate/SpacetimeCatalog/` is the executable statement, and it carries the TypeScript package's cases
rather than a summary of them.

```
cargo test -p agentic-spacetime-catalog
```

Thirty-four cases, in four files:

- **`Test/Catalog.rs`** - the seven cases that read the catalog as data: the six tables, one primary
  key per table, every column typed from the declared list, only `lease` and `belief` mutable, no
  column on `probe` that could hold the private value, no column anywhere named for a credential,
  and an undescribed table refused at the door.
- **`Test/Rules.rs`** - eighteen cases carried from `Reducers.test.ts`, plus two the port added: a
  claim at the end of the clock, which is refused rather than allowed to wrap, and `Latest` answered
  against the rows the rules wrote. Every refusal is also an assertion that nothing was written.
- **`Test/Destination.rs`** - the five cases carried from `Destination.test.ts`, which are the
  handover rule: an effect refused where it lands is recorded ambiguous and its resource
  quarantined, a caller that reproduces every number is still not the holder, a superseded fence is
  refused without an ambiguity, a released lease is an ambiguity, and a live one is a record.
- **`Source/Fn/Json.rs`** - two cases proving the writer that renders `generation.capabilities` and
  a trace's `payload` escapes what JSON requires, because the crate takes no serialiser.

What each invariant is proved by is named in its section above. Three further gates hold it:

- `cargo test --workspace` counts these cases with the rest of the tree, and
  `Crate/Agentic/Test/Composition.rs` holds the manifest to the crate's code vocabulary.
- `agentic verify --module @agentic/spacetime-catalog` reports ready: the manifest's
  `provides.errors` is `admission-refused`, `lease-held` and `budget-exhausted`, which is exactly
  `Code` in `Source/Struct/Code.rs`.
- `Crate/Agentic/Test/Boundaries.rs` keeps the arrow: the crate is a capability, it imports nothing,
  and only the composition root imports it.

The mirror with the runtime is the one correspondence the old suite wrote down.
`Crate/Runtime/Source/Struct/StandIn.rs` maps `record`'s checks column for column - the latest
generation, the lease row, the fence, the holder, the released and quarantined states, the expiry -
and `Crate/Runtime/Test/Runtime.rs` runs the same transcript as `Test/Destination.rs`. Two
implementations, one answer; if they ever disagree, one of them is wrong and the other is a second
authority.

# Open Questions

Every question the three captures left unanswered, gathered once. Each one names who asked it, the
experiment or command that would settle it, and what it blocks. `Documentation/Research.md` says
which thread each question came from; the captures themselves are the reasoning, and this file is
only the worklist.

A question is closed by a run, not by an argument. Where the experiment does not exist yet, that is
stated - an unbuilt fixture is a finding about the work, not a gap in the record.

## How to read an entry

- **Asked** - the thread's own follow-up list, or a check embedded in its findings, or this
  repository reading the capture. The three are not equal, and the difference is always written
  down.
- **Settle it with** - the run. Commands are literal; `<placeholders>` are not.
- **Blocks** - the crate or document that cannot be decided until it is answered.

---

# Thread 1 - hidden properties and disclosure

Source: `.hermes/research/research-575cb198.md`. Nothing in this workspace talks to SpacetimeDB yet,
so every experiment below is a probe against a local instance rather than a test in this tree.
`spacetime sql <database> "<query>"` is the shape of the check; `<database>` has to exist first.

## Q1 - What does a failed transaction roll back?

**Asked:** the thread's follow-up list (`research-575cb198.md:145`).

**Settle it with:** a reducer that inserts a row, then panics; query the table afterwards. A
surviving row means the insert is not part of the rollback.

```sh
spacetime sql < database > "SELECT * FROM Observation"
```

Repeat with a reducer that schedules a reducer and then fails - whether the schedule survives is a
separate answer.

**Blocks:** the `observe()` rule of charging and revealing in one transaction (`:135`). If rollback
is partial, "pay and reveal is all-or-nothing" is false and the reducer has to be restructured.

## Q2 - What is the exact concurrency model across modules?

**Asked:** the thread's follow-up list (`:146`).

**Settle it with:** two reducers in two different databases, each recording a start and end
timestamp; run them concurrently and read the interleaving. Then repeat inside one database, where
the thread says execution is serial (`:11`).

**Blocks:** the nonleakage argument, which assumes an observer cannot learn from interleaving it did
not cause.

## Q3 - How are scheduled reducers sequenced internally?

**Asked:** the thread's follow-up list (`:147`).

**Settle it with:** schedule two reducers for the same timestamp and record invocation order across
repeated runs. The thread already warns that no dispatch order is guaranteed (`:14`), so the
experiment's job is to find out whether that is merely undocumented or actually unstable.

**Blocks:** any probe-expiry or budget-refill design, both of which want a deterministic clock.

## Q4 - What are the formal guarantees of client subscriptions?

**Asked:** the thread's follow-up list (`:148`).

**Settle it with:** two clients, identities A and B, each subscribing to every public table and
every view, ignoring the intended UI; then compare the two transcripts field by field (`:127`,
`:132`).

**Blocks:** the nonleakage target itself (`:140`). A guarantee of "same visible values" is weaker
than the transcript equality the thread specifies, and only a subscription guarantee can close the
difference.

## Q5 - How does state replication affect determinism?

**Asked:** the thread's follow-up list (`:149`).

**Settle it with:** two replicas fed the same transaction stream; compare the observable transcripts
they serve. Divergence means the paired-world test cannot be run against a single node and
generalised.

**Blocks:** the paired-world experiment below (Q8), and with it the whole formal leakage claim.

## Q6 - Does one probe action strictly dominate all the others?

**Asked:** the thread's own check (`:122`).

**Settle it with:** enumerate the legal probes for one capability, compute what each reveals per
unit of cost, and check whether one action is weakly better in every belief state. If it is, the
hidden-information mechanic is degenerate and the disclosure policy has failed its own fairness
test.

**Blocks:** the disclosure policy. A policy that permits a dominant probe has published the secret
with extra steps.

## Q7 - Can two racing probes exceed a one-use allowance?

**Asked:** the thread's own check (`:135`).

**Settle it with:** two clients probing a capability with an allowance of one, in the same
transaction window; count the `Observation` rows written. The reducer authenticates and charges
before revealing, so the count must be one.

**Blocks:** the `observe()` reducer contract. This is the only place where the thread claims
atomicity buys correctness, so it is the one that has to be run.

## Q8 - Does a planted secret canary appear anywhere a client can see?

**Asked:** the thread's own check (`:139`).

**Settle it with:** plant a canary value in a private property, run a full probe, then read every
client-visible row, trace, failure response, result-size field and exported artefact, and grep for
the canary. Then run the paired worlds of `:140` - two databases differing only in the secret - and
diff the complete observable transcripts.

**Blocks:** the trace-projection rule and the claim that the schema provides nonleakage. The thread
is explicit that timing, size, contention and operator access are outside it (`:141`); the
experiment's job is to show those are the _only_ channels left.

---

# Thread 2 - the transport audit

Source: `.hermes/research/delo-e9b61659.md`. Every finding in this thread is a source reading plus
an inference, and the thread says so at `:14`. None of the nine has been run. The experiments below
are the thread's own, restated as the assertion each one has to make.

These block this workspace, not the repository they were read from: the audit is evidence about what
a browser transport must carry, and `agentic-browser-cdp` is the crate that has to carry it.

## Q9 - Does a reorder between read and click move the click to another post?

**Asked:** the thread's experiment (`:26`).

**Settle it with:** a fixture page with two articles; snapshot A at index 0, replace the article at
index 0 with B, then run the exported selector expression for the toggle and record which button
receives the click. No live account and no real mutation.

**Blocks:** the port shape of `agentic-browser-cdp`. If the transport can address a post by
position, the port has to accept an identity, not an index - which changes `Browse::Evaluate` as
specified in `Documentation/Rust.md`.

## Q10 - Does a URL target resolve the post named in the URL, or index 0?

**Asked:** the thread's experiment (`:38`).

**Settle it with:** a controlled page whose URL contains `/status/123` and whose articles are 999
then 123. The resolved post must be 123 regardless of order.

**Blocks:** `agentic-browser-cdp` and `agentic-chat-driver`, both of which take a URL target today.

## Q11 - Does an unrelated article satisfy readiness for an id target?

**Asked:** the thread's experiment (`:54`).

**Settle it with:** a fixture that inserts an unrelated article immediately, the target article
later, and its media later still; compare when the transport returns with when the fixture becomes
complete.

**Blocks:** the readiness rule in the transport port. "An article exists" is currently the whole
condition (`:42`).

## Q12 - Do `read({id})` and `read({url})` diverge on the same post?

**Asked:** the thread's experiment (`:84`).

**Settle it with:** one photo post, read four ways - by id with no existing tab, by id with a
pre-existing background tab, by url with no tab, by url with a tab. Compare the media fields.

**Blocks:** `agentic-browser-cdp`'s focus-emulation rule, which today applies to one of the four
paths only (`:78`).

## Q13 - Does "network idle" resolve with a request still in flight?

**Asked:** the thread's experiment (`:94`).

**Settle it with:** a fixture that starts a fetch whose response is held open, then
`waitForIdle({quiet: 600})`. If it resolves while the request is pending, the semantic gap is proven
rather than inferred.

**Blocks:** the persistence verification of every mutation, which currently assumes a quiet network
means a finished write.

## Q14 - Does a permanent evaluation error survive as its own cause?

**Asked:** the thread's experiment (`:128`).

**Settle it with:** inject a permanently invalid expression and a transient execution-context
destruction, and assert not only that both terminate but that the permanent error's cause is still
available to the caller.

**Blocks:** `agentic-browser-cdp`'s error taxonomy. A dead connection reported as "the page showed
no posts" is the failure the port exists to prevent.

## Q15 - Does an attachment failure leak the tab it opened?

**Asked:** the thread's experiment (`:138`).

**Settle it with:** inject an attachment failure after successful tab creation and assert that the
close is attempted. Controlled test, not a run that leaves tabs behind.

**Blocks:** `agentic-browser-cdp`'s disposal contract, which `Documentation/Reflection.md` leans on
for its resource-ownership claims.

## Q16 - Does a server rejection surface as an error, or as `now != want`?

**Asked:** the thread's experiment (`:116`), and one step it explicitly did not trace - the CLI's
treatment of the returned mismatch (`:114`).

**Settle it with:** a fake session that reports a successful click, never settles, then loses the
control; and a second test where the temporary-tab reload fails. Then read the caller's handling of
`now != want`.

**Blocks:** the port's result type. A mutation result that can be "not verified" is a different type
from a boolean, and `Documentation/Design.md`'s lifecycle claims persistence is established at
execution time.

## Q17 - Can extraction distinguish unknown from zero, false and empty?

**Asked:** the thread's check (`:74`).

**Settle it with:** DOM fixtures with missing controls, localized count labels, absolute rather than
relative permalinks, and a video poster with no playable source. The assertion is that each unknown
is distinguishable from a real `0`, `false` or `""`, not merely that extraction returns an object.

**Blocks:** `agentic-design-verification`. A measurement engine that cannot say "not observed"
produces evidence that reads as a measurement.

---

# Thread 3 - the Rust port

Source: `.hermes/research/rust-4ffde6d5.md`. The answer is cut off mid-sentence at `:378-383`, so
the last two questions it was asked were never reached.

## Q18 - Can a Rust adapter pass the TypeScript conformance suite?

**Asked:** the thread (`:281`), which makes this the precondition for its own recommendation.

**Closed by the layer's removal.** The suite it names - attach/launch, navigation races, evaluation
timeout, disconnect, disposal, shared-browser ownership - was answered by the Rust adapter through
`Crate/BrowserCDP/Test/Browse.rs` and the browse scenarios, and then the TypeScript half was deleted.
There is nothing left to compare against, so the question as written can no longer be run. What it
was protecting is now a different question, and a live one: whether those cases are right, given
that only one implementation asserts them (`Documentation/Parity.md` §What the differential run
established, while it existed).

**Blocks:** nothing now. The backend decision it conditioned is made - `chromiumoxide` behind the
session API - and `agentic-browser-cdp` is the crate that carries it.

## Q19 - How do the manifest fields map onto Rust?

**Asked:** the thread's question 6 (`:49`), never reached - the answer stops at `:378`.

**Settle it with:** write one real `module.json` for `ChatDriver`, a serde struct that validates it
in CI, and a harness that reads the packaged file alone and decides warm or cold from `state`,
`cost` and `artefact.hash`. If the planner needs to execute anything, the mapping has lost the
property it exists for.

**Blocks:** `agentic-runtime` (the pre-warm planner) and the manifest section of
`Documentation/Design.md`.

## Q20 - What is the CI check that keeps the arrows pointing inward?

**Asked:** the thread's question 5 (`:47`), asserted at `:58` but never specified.

**Answered:** `Crate/Agentic/Test/Boundaries.rs`, which reads every crate's `module.json` for its kind
and its `Cargo.toml` for its edges, and fails on an edge the design does not allow - a capability
naming an adapter, an adapter naming another adapter, anything naming the composition root. It runs
in `cargo test --workspace`, so the gate is the test job rather than a step somebody remembers. The
`cargo metadata` walk this entry used to propose stays the stronger check if features ever make the
declared edges and the resolved ones disagree; the declared edges are what a reviewer reads, and
they are what the test holds.

```sh
cargo test -p agentic --test Boundaries
```

**Blocks:** nothing now. `Documentation/Design.md`'s claim that the four layers are enforced rather
than merely described is what this test makes true.

## Q21 - Is the house style what the thread was told it is?

**Asked:** the thread's own caveat (`:60`) - it could not retrieve the repository and took the
description as specification.

**Settle it with:** clone the repository at the branch the request named and diff the described
files against what is there: `.cargo/config.toml` (target-dir, opt-level, codegen-units, lto, panic,
strip), the `[lib]` and `[[bin]]` layout, the `Fn`/`Struct` module trees, the CI workflow names.

```sh
git clone --branch Current https://github.com/PlayForm/Run < scratch > /Run
```

**Blocks:** `Documentation/Rust.md`'s opening claim about house style. If the clone fails, the house
style stays an unverified assumption and should be labelled as one.

## Q22 - Do the two consumer-owned `Browse` traits actually differ?

**Asked:** this repository, reading the capture. The thread asserts they need not have identical
methods (`:197`) but does not derive either trait from the existing call sites.

**Settle it with:** write both traits from what `chat-driver` and `design-verification` actually
call today, then diff them. Identical traits mean one trait belongs somewhere shared - or that the
split is theatre.

**Blocks:** `agentic-chat-driver`, `agentic-design-verification`, and the "ports belong to their
consumers" section of `Documentation/Rust.md`.

## Q23 - Does `panic = "abort"` survive the cancellation and cleanup ladder?

**Asked:** this repository, reading the capture. The request asked for anything in the house style
that fights the design (`:50`); the answer was cut off before that, and the style mandates
`panic = "abort"` (`:25`) while the design mandates supervised cleanup after a bounded evaluation
(`:259`).

**Settle it with:** a probe crate with `panic = "abort"` and a capability that panics
mid-evaluation; check whether the supervisor can still dispose the target and release the lease.
With abort there is no unwinding, so any cleanup that depends on `Drop` does not run.

**Blocks:** `agentic-runtime`'s cleanup supervision, and the release profile in
`Documentation/Rust.md`.

## Q24 - Which library behaviours must the session API be built around?

**Asked:** this repository, reading the capture. It states the requirement - a continuously polled
handler for `chromiumoxide`, an idle-disconnect policy for `headless_chrome` (`:234`) - without
measuring either.

**Settle it with:** two probes: stop polling the handler and see what pending commands do; hold a
connection idle past the documented interval and see whether it is closed under a warmed lease.

**Blocks:** `agentic-browser-cdp`'s lease model, whose whole premise is that a warm session is still
usable (`Documentation/Rust.md` §Still open 1).

---

# What blocks the most work

**Q18, Rust/TypeScript parity.** No longer a question: the TypeScript half was deleted, so the
comparison it names cannot be run. It is kept in the list above as the record of a precondition that
the tree satisfied and then removed, and what replaces it is the narrower question in
`Documentation/Parity.md` - whether the cases only the Rust side asserts are right.

**Q19 and Q20, manifest mapping and the arrows check.** These were the two halves of the answer that
was cut off. Q20 is now answered by `Crate/Agentic/Test/Boundaries.rs`; Q19 - a serde struct that
validates a manifest in CI - is still open, and without it the manifest is a JSON file nobody has
mapped.

**Q9 and Q10, transport identity.** If a transport can address a post by position, the browser
port's signature is wrong, and it is wrong in a way that every capability above it inherits. This is
the cheapest of the blocking questions to settle - a fixture page and no live account - which makes
it the one to run first.

**Q1 and Q7, transaction rollback and probe allowance.** These decide whether the disclosure model
in `research-575cb198.md` is implementable as written or only as a sketch. Nothing in this workspace
depends on them yet, which is why they are last: the thread that produced them has no consumer in
`Documentation/` today.

---

# Gates the TypeScript layer took with it

The TypeScript packages, the scenario runners and the Node toolchain were deleted once the
expressions were compiled at build time and the crates answered every scenario themselves. Four
gates went with them. Each is recorded here with what it held and what holds it now, because a gate
that disappears quietly is a property nobody notices losing.

- **The feed bundler's determinism.** `Scripts/Feed.mjs` drove `.hermes/tools/bundle.mjs` over a
  fixture and asserted that it splits, excludes and orders the same way twice. Both the tool and the
  gate were Node. The tool stays - it is local tooling, not part of the build - and nothing holds
  its determinism. **Owed:** a Rust gate over the bundler, or a decision that the bundler is not
  worth one.

- **The mutation suite.** `scenarios/mutate.mjs` applied five defects to both implementations'
  sources and insisted both suites rejected each. Three of the five edited TypeScript files that no
  longer exist. **Owed:** the five defects as a Rust-only harness, run by hand rather than from
  inside `cargo test` - a test that rewrites its own sources and re-runs `cargo test` is a test that
  can leave the tree mutated, and this working tree is swept into commits by an external process.
  The five are named in `Documentation/Parity.md` with the test each must break.

- **The markdown formatter.** `prettier --check` was the prose gate. **Decision:** no formatter
  check on markdown. Prose is formatted by hand and by the editor's own settings, and a Rust
  formatter for a house style this specific does not exist. `cargo fmt --check` remains the
  formatting gate for the code.

- **The catalogue's tests.** `Package/spacetime-catalog` held the database rules as data and 25
  assertions over them, and no crate replaced it. **Owed:** `agentic-spacetime-catalog`, with the
  rules as tables and the same cases. `Documentation/Spacetime-Catalog.md` is the statement of the
  rules in the meantime, and it is prose, so nothing compiles it.

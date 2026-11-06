# Open Questions

Every question the three research threads left unanswered, gathered once. Each one names who asked
it, the experiment or command that would settle it, and what it blocks. `Documentation/Research.md`
says which thread each question came from.

A question is closed by a run, not by an argument. Where the experiment does not exist yet, that is
stated - an unbuilt fixture is a finding about the work, not a gap in the record.

## How to read an entry

- **Asked** - the thread's own follow-up list, a check embedded in its findings, or this repository
  reading the thread. The three are not equal, and the difference is always written down.
- **Settle it with** - the run. Commands are literal; `<placeholders>` are not.
- **Blocks** - the crate or document that cannot be decided until it is answered.

---

# Thread 1 - hidden properties and disclosure

Nothing in this workspace talks to SpacetimeDB yet, so every experiment below is a probe against a
local instance rather than a test in this tree. `spacetime sql <database> "<query>"` is the shape of
the check; `<database>` has to exist first.

## Q1 - What does a failed transaction roll back?

**Asked:** the thread's follow-up list.

**Settle it with:** a reducer that inserts a row, then panics; query the table afterwards. A
surviving row means the insert is not part of the rollback.

```sh
spacetime sql <database> "SELECT * FROM Observation"
```

Repeat with a reducer that schedules a reducer and then fails - whether the schedule survives is a
separate answer.

**Blocks:** the `observe()` rule of charging and revealing in one transaction. If rollback is
partial, "pay and reveal is all-or-nothing" is false and the reducer has to be restructured.

## Q2 - What is the exact concurrency model across modules?

**Asked:** the thread's follow-up list.

**Settle it with:** two reducers in two different databases, each recording a start and end
timestamp; run them concurrently and read the interleaving. Then repeat inside one database, where
the thread says execution is serial.

**Blocks:** the nonleakage argument, which assumes an observer cannot learn from interleaving it did
not cause.

## Q3 - How are scheduled reducers sequenced internally?

**Asked:** the thread's follow-up list.

**Settle it with:** schedule two reducers for the same timestamp and record invocation order across
repeated runs. The thread already warns that no dispatch order is guaranteed, so the experiment's
job is to find out whether that is merely undocumented or actually unstable.

**Blocks:** any probe-expiry or budget-refill design, both of which want a deterministic clock.

## Q4 - What are the formal guarantees of client subscriptions?

**Asked:** the thread's follow-up list.

**Settle it with:** two clients, identities A and B, each subscribing to every public table and
every view, ignoring the intended UI; then compare the two transcripts field by field.

**Blocks:** the nonleakage target itself. A guarantee of "same visible values" is weaker than the
transcript equality the thread specifies, and only a subscription guarantee can close the
difference.

## Q5 - How does state replication affect determinism?

**Asked:** the thread's follow-up list.

**Settle it with:** two replicas fed the same transaction stream; compare the observable transcripts
they serve. Divergence means the paired-world test cannot be run against a single node and
generalised.

**Blocks:** the paired-world experiment below (Q8), and with it the whole formal leakage claim.

## Q6 - Does one probe action strictly dominate all the others?

**Asked:** the thread's own check.

**Settle it with:** enumerate the legal probes for one capability, compute what each reveals per
unit of cost, and check whether one action is weakly better in every belief state. If it is, the
hidden-information mechanic is degenerate and the disclosure policy has failed its own fairness
test.

**Blocks:** the disclosure policy. A policy that permits a dominant probe has published the secret
with extra steps.

## Q7 - Can two racing probes exceed a one-use allowance?

**Asked:** the thread's own check.

**Settle it with:** two clients probing a capability with an allowance of one, in the same
transaction window; count the `Observation` rows written. The reducer authenticates and charges
before revealing, so the count must be one.

**Blocks:** the `observe()` reducer contract. This is the only place where the thread claims
atomicity buys correctness, so it is the one that has to be run.

## Q8 - Does a planted secret canary appear anywhere a client can see?

**Asked:** the thread's own check.

**Settle it with:** plant a canary value in a private property, run a full probe, then read every
client-visible row, trace, failure response, result-size field and exported artefact, and grep for
the canary. Then run the paired worlds - two databases differing only in the secret - and diff the
complete observable transcripts.

**Blocks:** the trace-projection rule and the claim that the schema provides nonleakage. The thread
is explicit that timing, size, contention and operator access are outside it; the experiment's job
is to show those are the _only_ channels left.

---

# Thread 2 - the transport audit

Every finding in this thread is a source reading plus an inference, and the thread says so itself.
None of the nine has been run. The experiments below are the thread's own, restated as the assertion
each one has to make.

These block this workspace, not the repository they were read from: the audit is evidence about what
a browser transport must carry, and `agito-browser-cdp` is the crate that has to carry it.

## Q9 - Does a reorder between read and click move the click to another post?

**Asked:** the thread's experiment.

**Settle it with:** a fixture page with two articles; snapshot A at index 0, replace the article at
index 0 with B, then run the exported selector expression for the toggle and record which button
receives the click. No live account and no real mutation.

**Blocks:** the port shape of `agito-browser-cdp`. If the transport can address a post by position,
the port has to accept an identity, not an index - which changes `Browse::Evaluate` as specified in
`Documentation/Rust.md`.

## Q10 - Does a URL target resolve the post named in the URL, or index 0?

**Asked:** the thread's experiment.

**Settle it with:** a controlled page whose URL contains `/status/123` and whose articles are 999
then 123. The resolved post must be 123 regardless of order.

**Blocks:** `agito-browser-cdp` and `agito-chat-driver`, both of which take a URL target today.

## Q11 - Does an unrelated article satisfy readiness for an id target?

**Asked:** the thread's experiment.

**Settle it with:** a fixture that inserts an unrelated article immediately, the target article
later, and its media later still; compare when the transport returns with when the fixture becomes
complete.

**Blocks:** the readiness rule in the transport port. "An article exists" is currently the whole
condition.

## Q12 - Do `read({id})` and `read({url})` diverge on the same post?

**Asked:** the thread's experiment.

**Settle it with:** one photo post, read four ways - by id with no existing tab, by id with a
pre-existing background tab, by url with no tab, by url with a tab. Compare the media fields.

**Blocks:** `agito-browser-cdp`'s focus-emulation rule, which today applies to one of the four paths
only.

## Q13 - Does "network idle" resolve with a request still in flight?

**Asked:** the thread's experiment.

**Settle it with:** a fixture that starts a fetch whose response is held open, then
`waitForIdle({quiet: 600})`. If it resolves while the request is pending, the semantic gap is proven
rather than inferred.

**Blocks:** the persistence verification of every mutation, which currently assumes a quiet network
means a finished write.

## Q14 - Does a permanent evaluation error survive as its own cause?

**Asked:** the thread's experiment.

**Settle it with:** inject a permanently invalid expression and a transient execution-context
destruction, and assert not only that both terminate but that the permanent error's cause is still
available to the caller.

**Blocks:** `agito-browser-cdp`'s error taxonomy. A dead connection reported as "the page showed no
posts" is the failure the port exists to prevent.

## Q15 - Does an attachment failure leak the tab it opened?

**Asked:** the thread's experiment.

**Settle it with:** inject an attachment failure after successful tab creation and assert that the
close is attempted. Controlled test, not a run that leaves tabs behind.

**Blocks:** `agito-browser-cdp`'s disposal contract, which `Documentation/Reflection.md` leans on
for its resource-ownership claims.

## Q16 - Does a server rejection surface as an error, or as `now != want`?

**Asked:** the thread's experiment, and one step it explicitly did not trace - the CLI's treatment
of the returned mismatch.

**Settle it with:** a fake session that reports a successful click, never settles, then loses the
control; and a second test where the temporary-tab reload fails. Then read the caller's handling of
`now != want`.

**Blocks:** the port's result type. A mutation result that can be "not verified" is a different type
from a boolean, and `Documentation/Design.md`'s lifecycle claims persistence is established at
execution time.

## Q17 - Can extraction distinguish unknown from zero, false and empty?

**Asked:** the thread's check.

**Settle it with:** DOM fixtures with missing controls, localized count labels, absolute rather than
relative permalinks, and a video poster with no playable source. The assertion is that each unknown
is distinguishable from a real `0`, `false` or `""`, not merely that extraction returns an object.

**Blocks:** `agito-design-verification`. A measurement engine that cannot say "not observed"
produces evidence that reads as a measurement.

---

# Thread 3 - the Rust port

The thread's answer reaches all six questions it was asked, so what this file gathers is what the
answer leaves to be built and run here.

## Q18 - Can a Rust adapter pass the TypeScript conformance suite?

**Asked:** the port thread, which makes this the precondition for its own recommendation.

**Not runnable as written.** The suite it names - attach/launch, navigation races, evaluation
timeout, disconnect, disposal, shared-browser ownership - needs a second implementation to compare
against, and the tree has none. What the question was protecting is a different question, and a live
one: whether the browser behaviour `agito-browser-cdp` asserts is right, given that nothing else
asserts it (`Documentation/Parity.md` §What the differential run established).

**Blocks:** nothing. The backend decision it conditioned is made - `chromiumoxide` behind the
session API - and `agito-browser-cdp` is the crate that carries it.

## Q19 - How do the manifest fields map onto Rust?

**Asked:** the thread's question 6, answered in the thread.

**Settle it with:** write one real `module.json` for `ChatDriver`, a serde struct that validates it
in CI, and a harness that reads the packaged file alone and decides warm or cold from `state`,
`cost` and `artefact.hash`. If the planner needs to execute anything, the mapping has lost the
property it exists for.

**Blocks:** `agito-runtime` (the pre-warm planner) and the manifest section of
`Documentation/Design.md`.

## Q20 - What is the CI check that keeps the arrows pointing inward?

**Asked:** the thread's question 5.

**Open.** The check reads every crate's `module.json` for its kind and its `Cargo.toml` for its
edges, and fails on an edge the design does not allow - a capability naming an adapter, an adapter
naming another adapter, anything naming the composition root. `Crate/Agito/Test/Boundaries.rs` held
that check and ran in `cargo test --workspace`; the file is not in the tree. `cargo tree` answers
the same edge for a human, and nothing answers it for a push. A `cargo metadata` walk is the
stronger check if features ever make the declared edges and the resolved ones disagree; the declared
edges are what a reviewer reads, and they are what the check holds.

```sh
cargo tree -p agito-chat-driver | grep -c browser   # must answer 0
```

**Blocks:** nothing. Nothing checks it, so the inward arrows in `Documentation/Design.md` are a
claim a reader verifies by hand.

## Q21 - Is the house style what the thread was told it is?

**Asked:** the thread's own caveat - it could not retrieve the repository and took the description
as specification.

**Settle it with:** clone the repository at the branch the request named and diff the described
files against what is there: `.cargo/config.toml` (target-dir, opt-level, codegen-units, lto, panic,
strip), the `[lib]` and `[[bin]]` layout, the `Fn`/`Struct` module trees, the CI workflow names.

```sh
git clone --branch Current https://github.com/PlayForm/Run <scratch>/Run
```

**Blocks:** `Documentation/Rust.md`'s opening claim about house style. If the clone fails, the house
style stays an unverified assumption, which is how that file already labels it.

## Q22 - Do the two consumer-owned `Browse` traits actually differ?

**Asked:** this repository, reading the thread. The thread asserts they need not have identical
methods but does not derive either trait from the existing call sites.

**Settle it with:** write both traits from what `chat-driver` and `design-verification` actually
call today, then diff them. Identical traits mean one trait belongs somewhere shared - or that the
split is theatre.

**Blocks:** `agito-chat-driver`, `agito-design-verification`, and the "ports belong to their
consumers" section of `Documentation/Rust.md`.

## Q23 - Does `panic = "abort"` survive the cancellation and cleanup ladder?

**Asked:** this repository, reading the thread. The thread's request asked for anything in the house
style that fights the design, and the answer names one itself: `panic = "abort"` is listed among its
exceptions as incompatible with in-process panic recovery. The style mandates it while the design
mandates supervised cleanup after a bounded evaluation.

**Settle it with:** a probe crate with `panic = "abort"` and a capability that panics
mid-evaluation; check whether the supervisor can still dispose the target and release the lease.
With abort there is no unwinding, so any cleanup that depends on `Drop` does not run.

**Blocks:** `agito-runtime`'s cleanup supervision, and the release profile in
`Documentation/Rust.md`.

## Q24 - Which library behaviours must the session API be built around?

**Asked:** this repository, reading the thread. It states the requirement - a continuously polled
handler for `chromiumoxide`, an idle-disconnect policy for `headless_chrome` - without measuring
either.

**Settle it with:** two probes: stop polling the handler and see what pending commands do; hold a
connection idle past the documented interval and see whether it is closed under a warmed lease.

**Blocks:** `agito-browser-cdp`'s lease model, whose whole premise is that a warm session is still
usable. `Documentation/Design.md` §The manifest gives the reason: `HoldTransport` is a liveness
probe because an open socket can be dead.

---

# What blocks the most work

**Q18, Rust/TypeScript parity.** No longer a question: a comparison needs two implementations and
the tree has one. What replaces it is the narrower question in `Documentation/Parity.md` - whether
the behaviours only the Rust side asserted are right.

**Q19 and Q20, manifest mapping and the arrows check.** Q20's answer - a boundary test over
manifests and `Cargo.toml` edges - is not in the tree. Q19, a serde struct that validates a manifest
in CI, is open, and without it the manifest is a JSON file nobody has mapped.

**Q9 and Q10, transport identity.** If a transport can address a post by position, the browser
port's signature is wrong, and it is wrong in a way that every capability above it inherits. It is
the cheapest of the blocking questions to settle: a fixture page and no live account.

**Q1 and Q7, transaction rollback and probe allowance.** These decide whether the disclosure model
is implementable as written or only as a sketch. Nothing in this workspace depends on them yet: the
thread that produced them has no consumer in `Documentation/`.

---

# Gates and what holds them

Four gates stood beside the code. One is a formatter by decision, one has no counterpart, and two
have nothing holding them. Each is recorded with what it checked and what holds it now, because a
gate that disappears quietly is a property nobody notices losing.

- **The feed bundler's determinism.** `Script/Feed.mjs` drove `bundle.mjs` over a fixture and
  asserted that it splits, excludes and orders the same way twice. Both were Node, and neither is in
  the tree. Nothing holds the bundler's determinism. **Open:** a bundler and a gate over it, or a
  decision that the property is not worth holding.

- **The mutation suite.** `mutate.mjs` applied five defects to both implementations' sources and
  insisted both suites rejected each. Three of the five edited TypeScript files that no longer
  exist. **Open:** the five defects as a Rust-only harness, run by hand rather than from inside
  `cargo test` - a test that rewrites its own sources and re-runs `cargo test` is a test that can
  leave the tree mutated, and this working tree is swept into commits by an external process. The
  five are named in `Documentation/Parity.md` with the behaviour each breaks.

- **The markdown formatter.** `prettier --check` was the prose gate. **Decision:** it stays as a
  formatter and not as a gate. `package.json` carries `prettier` and `prettier.config.mjs` its
  settings, so Markdown and JSON are formatted by `npm run Format`; no run-time and no build step
  depends on it, and CI runs `cargo fmt --check` for the code alone. A Rust formatter for a house
  style this specific does not exist.

- **The catalogue's tests.** The TypeScript package held the database rules as data, with 30 cases
  across `Tables.test.ts`, `Reducers.test.ts` and `Destination.test.ts` - 7, 18 and 5.
  `Crate/Spacetime/Catalog/` carries the six tables as records and the seven rules as functions over
  them; the cases have no counterpart in the crate. `Documentation/Spacetime-Catalog.md` records
  what the port changed where the prose and the source disagreed.

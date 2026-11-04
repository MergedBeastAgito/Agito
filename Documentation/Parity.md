# Parity

What this file claims, and what it does not.

The scenarios under `scenarios/` are the contract, and `cargo test --workspace` is what holds the
crates to it. Nothing else states the behaviour of these capabilities, and nothing compares them to
a second implementation, because there is no second implementation: the TypeScript packages this
workspace was ported from were deleted once the browser expressions were compiled at build time and
every scenario was answered by a crate.

This file used to be the evidence that the two implementations were the same program. That claim is
retired rather than reworded - a comparison against a deleted tree cannot be re-run - and what is
recorded here is what the comparison established while it existed, what replaced it, and what the
deletion took with it.

The claim now is narrow on purpose. It is not that the crates are correct, and not that the scenario
files cover the whole of any implementation. It is that every case a file states is executed, that
every executed case names the unit test that proves it or says why it has none, and that a case's
expected result, ordered effect list and report count are asserted as written.

---

## What the scenarios are now

- **One file per capability is the contract.** `scenarios/browser-cdp.json`,
  `scenarios/chat-driver.json`, `scenarios/skill-contract.json` and the two files under
  `scenarios/generated/` describe a fixture, a sequence of operations, and the expected result.
  Nothing about a case lives in a test file: the case is the file, and the test is the interpreter
  that answers it.

- **Each file has exactly one reader.** `Crate/BrowserCDP/Test/Parity.rs` reads the two browse
  files, `Crate/ChatDriver/Test/Parity.rs` the two chat files, and
  `Crate/SkillContract/Test/Parity.rs` the skill file. Each builds the fixture the file describes -
  a scripted launcher and transport, a scripted page, a real temporary corpus - drives the port
  through it, and asserts the `expect` block.

- **Effects are the protocol, not the result.** Every case records what the implementation did to
  its fixture, in order: `launch:`, `attach:`, `handed:`, `evaluate:`, `close:`, `detach:`,
  `report:` for the browser; `open:`, `opened:`, `evaluate:<kind>`, `written:`, `sent:`,
  `refused:submit:disabled`, `release:`, `report:` for the chat page; `discovered:` and `finding:`
  for a corpus. Equality is asserted on that list as well as on the return value, so a port that
  answers correctly by the wrong route fails.

- **Every executed case names its unit case.** A case's `rust` field names a test in the crate's own
  contract suite, and each parity suite refuses a case that names a test which does not exist. A
  case whose rule only the fake can express carries `scenario-only` with the reason instead. This is
  what replaced the old case map: the binding is in the file, and a rename that breaks it fails the
  gate rather than leaving a table pointing at nothing.

- **The count.** 82 cases: 18 browse, 27 chat, 26 skill, and 11 timing cases across the two
  generated files. 78 execute on this side. Four are skipped, each with its reason written into the
  file, and two of the 78 are `scenario-only`.

```
cargo test --workspace                    # the suites, the scenarios and the gates
cargo test -p agentic-browser-cdp --test Parity
cargo test -p agentic-chat-driver --test Parity
cargo test -p agentic-skill-contract --test Parity
```

---

## What the differential run established, while it existed

The run drove the TypeScript packages in process, asked cargo for the Rust half through
`AGENTIC_PARITY_OUT`, and compared the two observation files. Its last recorded result:

- **82 scenarios across three capabilities: 79 agreed, 3 were unported, 0 differed.** The Rust
  halves were 23 browse scenarios, 29 chat scenarios and 26 skill scenarios; the difference between
  those numbers and 82 is the browse case whose release cannot fail in Rust, and the three chat
  cases neither side could express.
- **Agreement meant the same result and the same ordered effects** when both were driven by the same
  input - not that either implementation was right. A green run established that the port had not
  drifted from the tree it was ported from.
- **The comparison also held the fixtures honest.** The run refused to compare against a timing file
  that no longer matched its generator, so a scenario could not quietly become a hand-written
  expectation with a generated file's name on it.

**Why it is retired.** Once the TypeScript packages were deleted, the comparison had one side. The
scenarios were kept - they are the contract, and they were already what the Rust tests asserted -
but the referee, the observation files and the claim of equality went with the tree they compared
against. Keeping a differential run alive by keeping a second implementation is the cost this card
existed to stop paying.

**What that cost.** The comparison was an independent opinion: a behaviour that both implementations
got wrong agreed just as loudly as one they got right, but a behaviour one of them changed showed up
as a difference. Nothing does that now. The Rust tests are the only statement of the contract, and a
mistake they encode is a mistake the gate will defend.

---

## The cases that execute nothing

Each is written into its file with the reason, and each parity suite skips it by name rather than by
accident.

- **`dispose-failing-release`** (browse). `Transport::Close` returns `()`, so a release cannot fail
  and there is no cause to report. A release that _hangs_ is reportable and is covered by
  `generated/dispose-hang`; a release that _rejects_ is not expressible in the port at all.

- **`port-declares-its-errors`** (chat). The scenario vocabulary has no step for reading a module's
  own `module.json`. The rule - the codes the driver declares are the codes its manifest names - is
  asserted by `Crate/ChatDriver/Test/Chat.rs` and reported by
  `agentic verify --module @agentic/chat-driver`.

- **`options-absent`** and **`question-absent`** (chat). Both were holes in the TypeScript port:
  `undefined` where an `OperationOptions` or a `Question` was declared. Rust has no `undefined` - a
  `Call` is a value the caller must construct, and `Question` is a struct with no absent value - so
  there is nothing to prove. The cases stay as the record of a hole the type system closes, and
  nothing executes them.

Two further cases execute but name no unit test, and say so in `scenario-only`:

- **`answer-hangs`** and **`open-hangs`** (chat). `Test/Chat.rs` has no page that hangs or never
  opens; the parity fake does, and the scenario is the only place those rules are exercised. That is
  a gap in `Test/Chat.rs`, not in the port, and a unit case there would let these two drop the
  field.

---

## Why the fixtures assert what they assert

Three looseness decisions were made to let two implementations agree, and they are kept because each
one is a statement about what the contract is rather than a tolerance that happened to pass.

- **A message is a substring, not a sentence.** The code and the observation are the contract; the
  sentence is the port's own wording, and pinning it would make a rewording look like a change in
  behaviour. `generated/chat-answer-deadline` asserts the word `deadline`, not the sentence around
  it.

- **An observation may sit in `message` or `detail`.** Which field carries the reason is the port's
  choice; the promise is that the reason is not lost. `open-unavailable` asserts that `ECONNREFUSED`
  survives somewhere in the failure.

- **A finding's `detail` is not compared at all.** An `unparsable` detail embeds the absolute skill
  directory, which differs between two temporary corpora by construction, so the scenario compares
  findings by rule and leaves detail wording to `Crate/SkillContract/Test/SkillContract.rs`.

Two real defects were recorded here and are still in the tree, because the card that found them
owned the scenarios rather than the port:

- **Rust's open timeout loses the word "deadline".** `open-hangs` asserts the code and the
  observation because the message says `the chat page could not be opened` and puts
  `the page never answered` in `detail`. A wording defect, not a behavioural one.

- **`Names` treats a non-ASCII boundary as part of a name.** The port uses `char::is_alphanumeric`,
  which is Unicode, so with `Reference/Notes.md` on disk and `Reference/Index.md` naming
  `caféNotes.md` the port answers no findings. The TypeScript `Names` disagreed - its boundary was
  ASCII - and that disagreement is why no scenario covers it. Every ASCII case agrees, so the
  fixture set never exercises the rule. Now that only one implementation remains, this is an
  uncovered behaviour rather than a divergence: **a case is owed**, and the answer it should record
  is the one the port gives today or the one the contract should state, decided rather than
  inherited.

---

## The five mutations, as a procedure

`scenarios/mutate.mjs` applied five deliberate defects to the real source of both implementations,
ran both suites, and insisted each rejected every one - a green suite proves the assertions ran, not
that they bite. Three of the five edited TypeScript files that no longer exist, and the harness
itself was a Node script, so it went with the toolchain. What survives is the list, and this is how
to run one by hand.

The exact find/replace pairs are in the history: `git log --all --oneline -- scenarios/mutate.mjs`
names the commit that carried the harness, and `git show <commit>:scenarios/mutate.mjs` prints them.

| Mutation                   | What it breaks                                          | File                                           | The test that must fail                                                    |
| -------------------------- | ------------------------------------------------------- | ---------------------------------------------- | -------------------------------------------------------------------------- |
| `attached-page-closed`     | dispose closes an attached page instead of detaching it | `Crate/BrowserCDP/Source/Struct/Session.rs`    | `detaches_an_attached_target_without_closing_it`                           |
| `model-after-send`         | the model check moves after the send                    | `Crate/ChatDriver/Source/Struct/Driver.rs`     | `refuses_with_wrong_model_and_sends_nothing`                               |
| `partial-text-on-deadline` | a partial answer is returned when the budget expires    | `Crate/ChatDriver/Source/Struct/Driver.rs`     | `refuses_with_not_ready_when_the_answer_never_stops_changing`              |
| `eof-as-timeout`           | a transport EOF is classified as a timeout              | `Crate/BrowserCDP/Source/Fn/Classify.rs`       | `fails_with_browser_unavailable_when_the_transport_closed_during_the_call` |
| `threshold-one-read`       | the stability threshold drops from three reads to one   | `Crate/ChatDriver/Source/Struct/Settlement.rs` | `the_settlement_state_settles_on_the_third_agreement_and_not_before`       |

Run one like this. The snapshot is taken _before_ the edit and restored from, which is the harness's
own design: a snapshot taken after the edit restores the mutation and then verifies the mutation
against itself.

```
# 1. snapshot the file, then apply the mutation - exactly one occurrence of the text
cp Crate/ChatDriver/Source/Struct/Driver.rs "$TMPDIR/mutation.bak"
# 2. the named test must FAIL, and the suite must still compile
cargo test -p agentic-chat-driver --test Chat refuses_with_wrong_model_and_sends_nothing
# 3. put the snapshot back and prove the tree is unchanged
cp "$TMPDIR/mutation.bak" Crate/ChatDriver/Source/Struct/Driver.rs && git diff --quiet
```

Two of the five are caught by the effect list alone: `model-after-send` leaves the result
`wrong-model` with the same detail and only the order of `evaluate:model` against `evaluate:submit`
gives it away, and `threshold-one-read` leaves the answer's text unchanged and only the number of
`evaluate:answers` reads moves. A mutation that fails to compile is not a catch - it proves the
build, not the assertions.

This is a procedure, not a gate. A harness that rewrites its own sources and re-runs `cargo test`
from inside `cargo test` can leave a tree mutated, and this working tree is swept into commits by an
external process, so the restore would race the sweep. `Documentation/Open-Questions.md` records the
Rust-only harness as owed.

---

## The defect the comparison found that is still here

**The chat driver's poll pause is not cancellable.** In the settle loop, `Driver.rs` awaits
`self.Pace.Sleep(Wait)` directly, where `Wait = Poll.min(Call.Remaining(..))`. The TypeScript driver
raced the same wait against the caller's signal and resolved the moment the signal fired.

The two agreed in practice, because the loop re-reads on the next iteration and the next read starts
with `Call.Cancelled()` - which is why `generated/chat-answer-cancel` passes. The difference is
_when_ the caller hears about it. With the default `pollMs` of 250ms, a cancelled call waits out the
remainder of the current poll interval: up to a quarter of a second of extra latency, landing
exactly when the caller is trying to shed work. Worse, the guarantee is not local - it holds only
because the _pace_ eventually resolves, so a `Pace` whose `Sleep` never resolves turns a cancelled
`ask` into a call that never returns.

The fix is one combinator: `Race(self.Pace.Sleep(Wait), Call.Cancellation.Cancelled())`, the shape
`Bound` already uses for the read itself. It is not applied here because it changes the port, and
this card owns the scenarios and the layer around them.

# Parity

A corpus under `Scenario/` held 82 cases: one file per capability plus two timing files, each
stating a fixture, a sequence of steps, the expected result, the ordered effect list and the number
of reports. Three parity suites answered them through scripted fakes, so `cargo test --workspace`
was the whole of the scenario gate.

The claim those suites made was narrow, and nothing in the tree makes it now. It was not that the
crates are correct, and not that the files covered the whole of any implementation. It was that
every case a file stated was executed, that every executed case named the unit test that proved it
or said why it had none, and that a case's expected result, ordered effect list and report count
were asserted as written. A mistake the suites encoded was a mistake they defended.

## What the differential run established

The run drove the TypeScript packages in process, asked cargo for the Rust half through
`AGITO_PARITY_OUT`, and compared the two observation files. Its last recorded result:

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

**What the comparison bought.** It was an independent opinion: a behaviour that both implementations
got wrong agreed just as loudly as one they got right, but a behaviour one of them changed showed up
as a difference. That opinion costs a second implementation, and nothing in the tree pays that cost
today.

## What no case could prove

Each of these was written into its file with the reason, and each parity suite skipped it by name
rather than by accident. Each names a hole that is still a hole.

- **`dispose-failing-release`** (browse). `Transport::Close` returns `()`, so a release cannot fail
  and there is no cause to report. A release that _hangs_ is reportable; a release that _rejects_ is
  not expressible in the port at all.
- **`port-declares-its-errors`** (chat). The scenario vocabulary had no step for reading a module's
  own `module.json`. The rule - the codes the driver declares are the codes its manifest names - is
  what `agito verify --module @agito/chat-driver` reports.
- **`options-absent`** and **`question-absent`** (chat). Both were holes in the TypeScript port:
  `undefined` where an `OperationOptions` or a `Question` was declared. Rust has no `undefined` - a
  `Call` is a value the caller must construct, and `Question` is a struct with no absent value - so
  there is nothing to prove.
- **`answer-hangs`** and **`open-hangs`** (chat). Both executed but named no unit test, and said so
  in `scenario-only`: the chat driver's suite had no page that hangs or never opens, and the parity
  fake was the only place those rules were exercised. That gap is total now - nothing exercises
  them.

## Why the fixtures asserted what they asserted

Three looseness decisions let two implementations agree. Each is a statement about what the contract
is rather than a tolerance that happened to pass.

- **A message is a substring, not a sentence.** The code and the observation are the contract; the
  sentence is the port's own wording, and pinning it would make a rewording look like a change in
  behaviour.
- **An observation may sit in `message` or `detail`.** Which field carries the reason is the port's
  choice; the promise is that the reason is not lost.
- **A finding's `detail` was not compared at all.** An `unparsable` detail embeds the absolute skill
  directory, which differs between two temporary corpora by construction, so findings were compared
  by rule.

## Defects the comparison found

Two are still in the tree.

- **Rust's open timeout loses the word "deadline".** The message says
  `the chat page could not be opened` and puts `the page never answered` in `detail`. A wording
  defect, not a behavioural one.
- **`Names` treats a non-ASCII boundary as part of a name.** The port uses `char::is_alphanumeric`,
  which is Unicode, so with `Reference/Notes.md` on disk and `Reference/Index.md` naming
  `caféNotes.md` the port answers no findings. The TypeScript `Names` disagreed - its boundary was
  ASCII - and that disagreement is why no case covered it. Every ASCII case agreed, so the fixture
  set never exercised the rule. With one implementation left this is an uncovered behaviour rather
  than a divergence, and the tree has not decided which answer is the contract.

**A third is still open: the chat driver's poll pause is not cancellable.** In the settle loop,
`Driver.rs` awaits `self.Pace.Sleep(Wait)` directly, where `Wait = Poll.min(Call.Remaining(..))`.
The TypeScript driver raced the same wait against the caller's signal and resolved the moment the
signal fired.

The two agreed in practice, because the loop re-reads on the next iteration and the next read starts
with `Call.Cancelled()`. The difference is _when_ the caller hears about it. With the default
`pollMs` of 250ms, a cancelled call waits out the remainder of the current poll interval: up to a
quarter of a second of extra latency, landing exactly when the caller is trying to shed work. Worse,
the guarantee is not local - it holds only because the _pace_ eventually resolves, so a `Pace` whose
`Sleep` never resolves turns a cancelled `ask` into a call that never returns.

The fix is one combinator: `Race(self.Pace.Sleep(Wait), Call.Cancellation.Cancelled())`, the shape
`Bound` already uses for the read itself. It is not applied here because it changes the port.

## The five mutations

`mutate.mjs`, the mutation harness, applied five deliberate defects to the real source of both
implementations, ran both suites, and insisted each rejected every one - a green suite proves the
assertions ran, not that they bite.

| Mutation                   | What it breaks                                          | File                                            | The test that had to fail                                                  |
| -------------------------- | ------------------------------------------------------- | ----------------------------------------------- | -------------------------------------------------------------------------- |
| `attached-page-closed`     | dispose closes an attached page instead of detaching it | `Crate/Browser/CDP/Source/Struct/Session.rs`    | `detaches_an_attached_target_without_closing_it`                           |
| `model-after-send`         | the model check moves after the send                    | `Crate/Chat/Driver/Source/Struct/Driver.rs`     | `refuses_with_wrong_model_and_sends_nothing`                               |
| `partial-text-on-deadline` | a partial answer is returned when the budget expires    | `Crate/Chat/Driver/Source/Struct/Driver.rs`     | `refuses_with_not_ready_when_the_answer_never_stops_changing`              |
| `eof-as-timeout`           | a transport EOF is classified as a timeout              | `Crate/Browser/CDP/Source/Fn/Classify.rs`       | `fails_with_browser_unavailable_when_the_transport_closed_during_the_call` |
| `threshold-one-read`       | the stability threshold drops from three reads to one   | `Crate/Chat/Driver/Source/Struct/Settlement.rs` | `the_settlement_state_settles_on_the_third_agreement_and_not_before`       |

Two of the five were caught by the effect list alone: `model-after-send` left the result
`wrong-model` with the same detail and only the order of `evaluate:model` against `evaluate:submit`
gave it away, and `threshold-one-read` left the answer's text unchanged and only the number of
`evaluate:answers` reads moved. A mutation that fails to compile is not a catch - it proves the
build, not the assertions.

The find/replace pairs are in the history: `git log --all --oneline -- '*mutate.mjs'` names the
commit that carried the harness, and `git show <commit> -- '*mutate.mjs'` prints them.

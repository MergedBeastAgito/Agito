# Parity

What this file claims, and what it does not.

Seventy-five green assertions establish that seventy-five assertions passed. They do not establish
that the Rust port and the TypeScript one are the same program. This file is the evidence that they
are: one scenario file per capability that both implementations read, the ordered effects each one
produced, a mutation suite that proves the assertions bite, and a map from every TypeScript case to
its Rust counterpart.

The claim is narrow on purpose. It is not that the Rust port is correct, and not that the scenario
files cover the whole of either implementation. It is that for the behaviours both suites already
assert, both implementations produce the same result **and** the same ordered sequence of externally
visible effects when driven by the same input.

---

## The shape of the differential run

- **One file per capability is the input.** `scenarios/browser-cdp.json`,
  `scenarios/chat-driver.json` and `scenarios/skill-contract.json` describe a fixture, a sequence of
  operations, and the expected result. Nothing about a case lives in either language's test file:
  the TypeScript side (`scenarios/lib/*.mjs`, driven by `Package/*/Test/Parity.test.ts`) and the
  Rust side (`Crate/*/Test/Parity.rs`) both read the same JSON and answer the same `expect` block,
  so the two suites cannot drift by being written twice.

- **Effects are the protocol, not the result.** Every fixture records what the implementation did to
  it, in order: `launch:`, `attach:`, `handed:`, `evaluate:`, `close:`, `detach:`, `report:` for the
  browser; `open:`, `opened:`, `evaluate:<kind>`, `written:`, `sent:`, `refused:submit:disabled`,
  `release:`, `report:` for the chat page; `discovered:` and `finding:` for a corpus. Equality is
  asserted on that list as well as on the return value, so an implementation that answers correctly
  by the wrong route fails. Two of the five mutations are caught by nothing else: the order of
  `evaluate:model` against `evaluate:submit`, and the number of `evaluate:answers` reads.

- **Timing cases are generated.** `scenarios/generate.mjs` produces
  `scenarios/generated/browser-cdp.timing.json` and `scenarios/generated/chat-driver.timing.json`
  from a (phase × event) matrix - cancellation mid-operation, deadline expiry, transport death
  mid-operation, the three-read stability threshold, a hung release - and each case carries the
  parameters that produced it. The files are committed so both sides can read them, and `run.mjs`
  refuses to compare against a copy that no longer matches the generator.

- **The referee is one script.** `node scenarios/run.mjs --compare` runs the TypeScript half
  in-process, asks cargo for the Rust half through `AGENTIC_PARITY_OUT`, and reports per scenario.
  `node scenarios/mutate.mjs --check` applies five deliberate defects to the real sources, one at a
  time, and insists both suites reject each one.

```
node scenarios/run.mjs --compare        # 82 scenarios, both implementations
node scenarios/mutate.mjs --check       # five mutations, both suites must fail
cargo test --workspace                  # the Rust suite, including Test/Parity.rs
cd Package/browser-cdp && node --test Test/*.test.ts   # and the other two packages
```

- **What the run reported.** 82 scenarios exist across the three capabilities; 79 agreed on both
  sides, 3 are unported (below), 0 differed. The Rust halves are 23 browse scenarios, 29 chat
  scenarios and 26 skill scenarios; the difference between those numbers and 82 is the browse case
  whose release cannot fail in Rust, and the three chat cases neither side can express.

---

## The case map

Seventy-five TypeScript cases, in suite order: 26 in `Browse.test.ts`, 27 in `Chat.test.ts`, 22 in
`SkillContract.test.ts`. **Seventy run against both implementations** (69 through a scenario on both
sides, one - `dispose-failing-release` - through a scenario on the TypeScript side only, for the
reason given below). **Five have no executed scenario**: two are declaration checks that a manifest
matches a type, three are JavaScript-only holes. **Three cases are unported**: no Rust counterpart
of any kind exists, and each is listed with its reason. Every row names the Rust case and the
scenario that drives it; `-` means there is none, and the reason is in the section after the table.

The table is the only table in this file, and no other line in it contains a pipe, so counting the
lines that do answers 77: these 75 case rows plus the header and the separator.

| #   | TypeScript case                                                                                 | Rust case                                                                       | Scenario                                                                                              |
| --- | ----------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------- | ----------------------------------------------------------------------------------------------------- |
| 1   | the port declares exactly the errors the manifest promises                                      | `the_port_declares_exactly_the_errors_the_manifest_promises`                    | -                                                                                                     |
| 2   | launches a browser when the caller names no endpoint                                            | `launches_a_browser_when_the_caller_names_no_endpoint`                          | `open-launches`                                                                                       |
| 3   | attaches to a browser the caller already owns                                                   | `attaches_to_a_browser_the_caller_already_owns`                                 | `open-attaches`                                                                                       |
| 4   | rejects with navigation-failed when the page does not load                                      | `fails_with_navigation_failed_when_the_page_does_not_load`                      | `open-navigation`                                                                                     |
| 5   | rejects with browser-unavailable when the browser cannot be reached                             | `fails_with_browser_unavailable_when_the_browser_cannot_be_reached`             | `open-unavailable`                                                                                    |
| 6   | rejects with timeout when the launcher never answers                                            | `fails_with_timeout_when_the_launcher_never_answers`                            | `generated/open-deadline`                                                                             |
| 7   | rejects with timeout when the caller's signal fires                                             | `fails_with_timeout_when_the_callers_signal_fires`                              | `generated/open-cancel`                                                                               |
| 8   | releases a transport that arrives already dead                                                  | `releases_a_transport_that_arrives_already_dead`                                | `open-dead-on-arrival`                                                                                |
| 9   | returns the value the page produced                                                             | `returns_the_value_the_page_produced`                                           | `evaluate-answers`                                                                                    |
| 10  | rejects with timeout when the page never answers                                                | `fails_with_timeout_when_the_page_never_answers`                                | `generated/evaluate-deadline`                                                                         |
| 11  | rejects with timeout when the signal fires mid-flight                                           | `fails_with_timeout_when_the_signal_fires_mid_flight`                           | `generated/evaluate-cancel`                                                                           |
| 12  | rejects with timeout before asking when the signal is already aborted                           | `fails_with_timeout_before_asking_when_the_signal_is_already_aborted`           | `evaluate-aborted-before`                                                                             |
| 13  | still honours the deadline when the caller passes no signal                                     | `still_honours_the_deadline_with_a_cancellation_that_never_fires`               | `evaluate-no-signal`                                                                                  |
| 14  | rejects with browser-unavailable when the transport closed during the call                      | `fails_with_browser_unavailable_when_the_transport_closed_during_the_call`      | `generated/evaluate-death`                                                                            |
| 15  | never asks a transport it already knows is closed                                               | `never_asks_a_transport_it_already_knows_is_closed`                             | `evaluate-known-closed`                                                                               |
| 16  | rejects with navigation-failed when the page throws                                             | `fails_with_navigation_failed_when_the_page_throws`                             | `evaluate-page-throws`                                                                                |
| 17  | rejects with browser-unavailable on a target this session does not hold                         | `fails_with_browser_unavailable_on_a_target_this_session_does_not_hold`         | `evaluate-foreign-target`                                                                             |
| 18  | closes a launched target                                                                        | `closes_a_launched_target`                                                      | `dispose-launched`                                                                                    |
| 19  | detaches an attached target without closing it                                                  | `detaches_an_attached_target_without_closing_it`                                | `dispose-attached`                                                                                    |
| 20  | is silent when called twice                                                                     | `is_silent_when_called_twice`                                                   | `dispose-twice`                                                                                       |
| 21  | reports a failing release instead of throwing it                                                | `reports_a_failing_release_instead_of_returning_it`                             | `dispose-failing-release`                                                                             |
| 22  | does not throw when its own reporter throws                                                     | `does_not_fail_when_its_own_reporter_panics`                                    | `dispose-reporter-throws`                                                                             |
| 23  | reports a release that hangs instead of waiting forever                                         | `reports_a_release_that_hangs_instead_of_waiting_for_it`                        | `generated/dispose-hang`                                                                              |
| 24  | resolves for a target this session never opened                                                 | `resolves_for_a_target_this_session_never_opened`                               | `dispose-foreign-target`                                                                              |
| 25  | resolves even when handed something that is not a target                                        | -                                                                               | -                                                                                                     |
| 26  | leaves the target unusable afterwards                                                           | `leaves_the_target_unusable_afterwards`                                         | `dispose-then-evaluate`                                                                               |
| 27  | the port declares exactly the errors the manifest promises                                      | `the_port_declares_exactly_the_errors_the_manifest_promises`                    | `port-declares-its-errors`                                                                            |
| 28  | returns the answer, its trace and its sources                                                   | `returns_the_answer_its_trace_and_its_sources`                                  | `answer-trace-sources`                                                                                |
| 29  | opens the configured page and disposes the target once                                          | `opens_the_configured_page_and_disposes_the_target_once`                        | `open-and-release`                                                                                    |
| 30  | presses the toggle when the composer is not in search mode                                      | `presses_the_toggle_when_the_composer_is_not_in_search_mode`                    | `mode-press`                                                                                          |
| 31  | refuses with wrong-mode and sends nothing when the toggle never reads search                    | `refuses_with_wrong_mode_and_sends_nothing_when_the_toggle_never_reads_search`  | `mode-stuck`                                                                                          |
| 32  | refuses a question that asks for a mode this port does not offer                                | `refuses_a_question_that_asks_for_a_mode_this_port_does_not_offer`              | `mode-question-unsupported`                                                                           |
| 33  | refuses with wrong-model and sends nothing when the page is set to another model                | `refuses_with_wrong_model_and_sends_nothing`                                    | `model-wrong`                                                                                         |
| 34  | matches the label across case and whitespace, and reports what it read                          | `matches_the_label_across_case_and_whitespace_and_reports_what_it_read`         | `model-normalised`                                                                                    |
| 35  | returns the last text of an answer that stops changing                                          | `returns_the_last_text_of_an_answer_that_stops_changing`                        | `answer-streams-then-settles`, `generated/chat-answer-settles`                                        |
| 36  | reads the answer this question caused, not the one already in the thread                        | `reads_the_answer_this_question_caused_not_the_one_already_in_the_thread`       | `answer-new-turn-only`                                                                                |
| 37  | reads the steps and sources of this turn, not the earlier turn's                                | `reads_the_steps_and_sources_of_this_turn`                                      | `answer-this-turn-trace`                                                                              |
| 38  | refuses with not-ready when the answer never stops changing                                     | `refuses_with_not_ready_when_the_answer_never_stops_changing`                   | `answer-never-settles`, `generated/chat-answer-deadline`                                              |
| 39  | does not wait for ever on a page that ignores its deadline                                      | -                                                                               | `answer-hangs`                                                                                        |
| 40  | refuses with truncated when the answer hides behind a Read more control                         | `refuses_with_truncated_when_the_answer_hides_behind_a_read_more_control`       | `answer-collapsed`                                                                                    |
| 41  | reports not-ready when the page cannot be opened                                                | `reports_not_ready_when_the_page_cannot_be_opened`                              | `open-fails`                                                                                          |
| 42  | does not wait for ever on a page that never opens                                               | -                                                                               | `open-hangs`                                                                                          |
| 43  | reports not-ready when the page fails while it is being read                                    | `reports_not_ready_when_the_page_fails_while_it_is_being_read`                  | `evaluate-fails`, `generated/chat-answer-death`                                                       |
| 44  | refuses to press a send control that never enables                                              | `refuses_to_press_a_send_control_that_never_enables`                            | `submit-disabled`                                                                                     |
| 45  | reports not-ready before opening anything when the signal already fired                         | `reports_not_ready_before_opening_anything_when_the_signal_already_fired`       | `signal-aborted-before-open`                                                                          |
| 46  | reports not-ready when the caller gives up while the answer is arriving                         | `reports_not_ready_when_the_caller_gives_up_while_the_answer_is_arriving`       | `answer-cancelled`, `generated/chat-answer-cancel`                                                    |
| 47  | reports a release that hangs instead of waiting for it                                          | `reports_a_release_that_hangs_instead_of_waiting_for_it`                        | `release-hangs`, `generated/chat-release-deadline`                                                    |
| 48  | refuses a mode that is not text with wrong-mode, and sends nothing                              | `refuses_a_mode_that_is_not_text_with_wrong_mode_and_sends_nothing`             | `mode-not-text`                                                                                       |
| 49  | refuses a thread read that is not a thread with not-ready                                       | `refuses_a_thread_read_that_is_not_a_thread`                                    | `answers-not-a-thread`                                                                                |
| 50  | refuses a trace that is not a list of steps with not-ready                                      | `refuses_a_trace_that_is_not_a_list_of_steps`                                   | `trace-not-a-list`                                                                                    |
| 51  | refuses sources that are not a list with not-ready                                              | `refuses_sources_that_are_not_a_list`                                           | `sources-not-a-list`                                                                                  |
| 52  | refuses a call with no options as not-ready, not a TypeError                                    | -                                                                               | `options-absent`                                                                                      |
| 53  | refuses a call with no question as not-ready, not a TypeError                                   | -                                                                               | `question-absent`                                                                                     |
| 54  | discover returns every skill directory, ordered by name, whatever order they were created in    | `discover_returns_every_skill_directory_ordered_by_name`                        | `discover-order`                                                                                      |
| 55  | discover is deterministic: two runs over the same tree agree                                    | `discover_is_deterministic_across_two_runs`                                     | `discover-deterministic`                                                                              |
| 56  | a symlinked skill directory is discovered like a real one                                       | `a_symlinked_skill_directory_is_discovered_like_a_real_one`                     | `discover-symlink`                                                                                    |
| 57  | a valid skill has no findings                                                                   | `a_valid_skill_has_no_findings`                                                 | `valid-skill`                                                                                         |
| 58  | a reference file the index does not name is one unlisted-reference finding                      | `a_reference_file_the_index_does_not_name_is_one_unlisted_reference`            | `unlisted-reference`                                                                                  |
| 59  | a reference tree without an index reports each file as unlisted                                 | `a_reference_tree_without_an_index_reports_each_file_as_unlisted`               | `reference-tree-without-index`                                                                        |
| 60  | an index that names a longer file does not name the shorter one                                 | `an_index_that_names_a_longer_file_does_not_name_the_shorter_one`               | `index-names-longer-file`                                                                             |
| 61  | a skill directory without a SKILL.md is unparsable and uncited, not an exception                | `a_skill_directory_without_a_body_is_unparsable_and_uncited`                    | `no-skill-file`                                                                                       |
| 62  | unparsable frontmatter is a finding, not a rejection                                            | `unparsable_frontmatter_is_a_finding_not_a_rejection`                           | `unparsable-frontmatter`                                                                              |
| 63  | a frontmatter without a description is unparsable                                               | `a_frontmatter_without_a_description_is_unparsable`                             | `no-description`                                                                                      |
| 64  | a frontmatter name that disagrees with the directory is unparsable                              | `a_frontmatter_name_that_disagrees_with_the_directory_is_unparsable`            | `name-mismatch`                                                                                       |
| 65  | a corpus root that names its skills in AGENTS.md is read as well as one that uses Index.md      | `a_corpus_root_that_names_its_skills_in_agents_md_is_read_as_well`              | `corpus-index-agents`                                                                                 |
| 66  | a skill the corpus index does not name is uncited                                               | `a_skill_the_corpus_index_does_not_name_is_uncited`                             | `uncited`                                                                                             |
| 67  | every finding uses one of the three rules the manifest declares                                 | `every_finding_uses_one_of_the_three_rules_the_manifest_declares`               | `rules-are-declared`                                                                                  |
| 68  | validate is stateless: the same skill answers the same findings twice                           | `validate_is_stateless_and_answers_the_same_findings_twice`                     | `validate-stateless`                                                                                  |
| 69  | validate re-reads the directory, so a file added after the first call is reported by the second | `validate_re_reads_the_directory_so_a_later_file_is_reported`                   | `validate-rereads`                                                                                    |
| 70  | a skill with no Reference tree is valid                                                         | `a_skill_with_no_reference_tree_is_valid`                                       | `no-reference-tree`                                                                                   |
| 71  | a folded description is read as the description                                                 | `a_folded_description_is_read_as_the_description`                               | `folded-description`                                                                                  |
| 72  | an unreadable root is an empty corpus, not a rejection                                          | `an_unreadable_root_is_an_empty_corpus_not_a_failure`                           | `absent-root`                                                                                         |
| 73  | the digest ignores modification times                                                           | `the_digest_ignores_modification_times_and_directories`                         | `digest-ignores-mtimes`                                                                               |
| 74  | the digest changes when a name, a description, a reference or the order changes                 | `the_digest_changes_when_a_name_a_description_a_reference_or_the_order_changes` | `digest-order`, `digest-name`, `digest-description`, `digest-references`, `digest-directory-excluded` |
| 75  | the digest of the valid skill is the recorded one                                               | `the_digest_of_the_valid_skill_is_the_recorded_one`                             | `digest-recorded`                                                                                     |

---

## The three unported cases

Each of these has no Rust counterpart of any kind - not a unit case, not a scenario - and the reason
is a fact about the two type systems rather than about the two behaviours.

- **`resolves even when handed something that is not a target`** (row 25). The TypeScript case calls
  `dispose(undefined)` and asserts the port reports it rather than throwing, because a caller is not
  a type checker. Rust's `Dispose` takes a `Target` by value: there is no value that is not a
  target, so there is no hole to fall into and nothing to prove. The rule it protects - dispose
  never fails - is covered in Rust by `resolves_even_when_handed_a_target_with_no_endpoint` and by
  the scenario `dispose-foreign-target`.

- **`refuses a call with no options as not-ready, not a TypeError`** (row 52). Same hole:
  `undefined` where an `OperationOptions` is declared. In Rust a `Call` is a value the caller must
  construct, and the deadline and cancellation are fields of it.

- **`refuses a call with no question as not-ready, not a TypeError`** (row 53). Same hole again: a
  missing `Question`. Rust's `Question` is a struct with no absent value.

Two further cases have no executed scenario but are not unported, and are worth naming so the count
is not mysterious:

- **Row 1 and row 27** (`the port declares exactly the errors the manifest promises`) assert that a
  package's `module.json` names the same codes its type declares. There is no operation to drive and
  no page to open, so no scenario can express it; both suites keep the assertion in their own files.

- **Row 21** (`reports a failing release instead of throwing it`) runs as a scenario on the
  TypeScript side only. Rust's `Transport::Close` returns `()`, so a release cannot fail and there
  is no cause to report - only a hang is reportable, which is `generated/dispose-hang`. The Rust
  unit case `reports_a_failing_release_instead_of_returning_it` models the same rule with a hung
  release, which is the only shape the Rust port has for it.

- **Rows 39 and 42** (`does not wait for ever on a page that ignores its deadline` /
  `... never opens`) run as scenarios on both sides but have no Rust unit case: `Test/Chat.rs` has
  no hanging page. The parity fake does, so the rule is proved for the Rust port by the differential
  run and not by its own suite. That is a gap in `Test/Chat.rs`, not in the port.

One more thing the table does not say: eight skill-contract cases assert on the _text_ of a
finding's detail as well as on its rule. The scenario compares findings by rule only, because an
`unparsable` detail embeds the absolute skill directory and necessarily differs between two runs in
two temporary directories. Detail wording is compared by the two contract suites, which build their
corpora in process.

---

## The five mutations

`node scenarios/mutate.mjs --check` applies each defect to the real source of both implementations,
runs both suites, and restores every file from a snapshot taken before the edit - verified by hash
afterwards, and audited again at the end of the run. The output of the run that shipped this file:

```
mutation                     typescript  rust
------------------------------------------------------------
attached-page-closed         rejected    rejected
  closing an attached page on dispose
  typescript: not ok 2 - detaches an attached target without closing it
  rust:       ---- detaches_an_attached_target_without_closing_it stdout ----
model-after-send             rejected    rejected
  checking the model after the send instead of before
  typescript: not ok 1 - refuses with wrong-model and sends nothing when the page is set to another model
  rust:       ---- refuses_with_wrong_model_and_sends_nothing stdout ----
partial-text-on-deadline     rejected    rejected
  returning partial text on deadline expiry
  typescript: not ok 4 - refuses with not-ready when the answer never stops changing
  rust:       ---- refuses_with_not_ready_when_the_answer_never_stops_changing stdout ----
eof-as-timeout               rejected    rejected
  classifying a transport EOF as a timeout
  typescript: not ok 6 - rejects with browser-unavailable when the transport closed during the call
  rust:       ---- fails_with_browser_unavailable_when_the_transport_closed_during_the_call stdout ----
threshold-one-read           rejected    rejected
  changing the stability threshold from three reads to one
  typescript: not ok 9 - scenario answer-trace-sources
  rust:       ---- the_settlement_state_settles_on_the_third_agreement_and_not_before stdout ----
------------------------------------------------------------
5/5 mutations rejected by both suites; 6/6 files restored byte-for-byte
```

- **Two of the five are caught by the effect list alone.** `model-after-send` moves the model check
  after the send: the result is still `wrong-model` with the same detail, and only the order of
  `evaluate:model` against `evaluate:submit` gives it away. `threshold-one-read` changes
  `StableReads` from 3 to 1: the answer's text is unchanged and the number of `evaluate:answers`
  reads is not.
- **A mutation that does not compile is not a catch.** The harness distinguishes a suite that failed
  an assertion from a suite that failed to build, and counts the second as a hole. It caught a bad
  mutation of its own on the way: `Error::Timeout(...)` written as a bare match arm value left the
  original arm's comma behind and produced `),,`.
- **The restore is the dangerous part.** The first version of this harness snapshotted the file
  _after_ editing it, so it restored the mutation and then verified the mutation against itself; two
  source files were left mutated by a run that reported success. The snapshot is now taken before
  the edit, `originals` and `current` are separate maps, and a run-level audit re-hashes every
  touched file at the end.

---

## Where the two ports differ today, and why the scenarios tolerate it

These are real differences, recorded rather than smoothed over. None of them changes a code, an
effect, or a report count - which is why the differential run is green - but a reader comparing the
two sources would find them.

- **The sentence and the observation are split differently.** TypeScript writes its own sentence and
  keeps the reason in `detail`; the Rust port has one error type with `Message` and `Detail` and
  reaches for whichever it has. `open-unavailable` therefore asserts that the token `ECONNREFUSED`
  survives somewhere in the failure, not which field it sits in.

- **Deadline wording.** TypeScript's message carries the budget that was left when the read started
  (`answer exceeded its 38ms deadline`); Rust's carries the whole budget
  (`answer exceeded the 40ms deadline`). The code and `detail` are identical, so the scenarios
  assert those.

- **Rust's open timeout loses the word "deadline".** TypeScript says
  `open exceeded its 60ms deadline`; Rust says `the chat page could not be opened` and puts
  `the page never answered` in `detail`. The scenario `open-hangs` asserts the code and the
  observation. This is a defect in the Rust port's wording, not in its behaviour.

- **`Names` disagrees on a non-ASCII boundary.** TypeScript's `Names` spells the boundary as a
  negated character class over word characters, dots and hyphens - and JavaScript's `\w` is ASCII,
  so `é` ends a name; the Rust port uses `char::is_alphanumeric`, which is Unicode, so `é` continues
  one. With `Reference/Notes.md` on disk and `Reference/Index.md` containing `caféNotes.md`, the
  TypeScript port answers no findings and the Rust port answers `unlisted-reference`. No scenario
  covers it, because it would fail on one side; every ASCII case in the file agrees. The fix belongs
  in one of the two `Names` implementations.

- **`Format` is Rust-only.** `agentic chat --format markdown` reads an answer by serialising its DOM
  (`Crate/ChatDriver/Source/Script/Expressions/Extract.ts`); the TypeScript half has no equivalent.
  Every scenario is therefore a `text` scenario, and both halves still speak the same expression for
  it - the Rust `text` expression is byte-identical to the TypeScript one, which is what lets each
  fake recognise its own builder. A `markdown` case cannot join the differential run until the
  TypeScript layer carries the serializer, and that layer is what `Package/` is scheduled to lose.
  What proves the Markdown reading instead is `Crate/Agentic/Test/Extract.rs`, which drives a real
  page and asserts the pipe table, the fence, the TeX and the nested list.

---

## The one place they agree today that would diverge under load

**The Rust chat driver's poll pause is not cancellable.** In the settle loop, `Driver.rs` awaits
`self.Pace.Sleep(Wait)` directly, where `Wait = Poll.min(Call.Remaining(..))`. TypeScript's
`pause()` races the same wait against the caller's signal and resolves the moment the signal fires.

Today the two agree, because the loop re-reads on the next iteration and the next read starts with
`Call.Cancelled()` - which is why `generated/chat-answer-cancel` passes on both sides. The
difference is _when_ the caller hears about it. With the default `pollMs` of 250ms, a TypeScript
caller that cancels mid-answer is released immediately, and a Rust caller waits out the remainder of
the current poll interval. Under load - many sessions sharing one process, a page that has stopped
answering, a signal that fires while a slow read is in flight - that is up to a quarter of a second
per call of indistinguishable extra latency, and it lands exactly when the caller is trying to shed
work. Worse, the guarantee is not local: it holds only because the _pace_ eventually resolves. A
`Pace` whose `Sleep` never resolves - a stalled executor, a test with a virtual clock that does not
advance - turns a cancelled `ask` into a call that never returns, while the TypeScript driver
returns at once.

The fix is one combinator: `Race(self.Pace.Sleep(Wait), Call.Cancellation.Cancelled())`, which is
the same shape `Bound` already uses for the read itself. It is not applied here because it is a
change to the port's implementation, and this card owns the scenarios, not the port.

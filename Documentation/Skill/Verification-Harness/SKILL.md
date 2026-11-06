---
name: verification-harness
description: Use when a tree has no test command and a change still needs proof. Assert, run, quote.
version: 1.0.0
license: MIT
platforms: [macos, linux]
metadata:
    hermes:
        tags: [verification, harness, evidence, shell, false-green]
        related_skills: [cargo-probe, bench-harness]
---

# Verification Harness

`cargo build --workspace --release` proves the change compiles.
`cargo clippy --workspace --all-targets -- -D warnings` proves it is clean.
`cargo fmt --all -- --check` proves it is formatted. None of them proves the flag appears in
`--help`, that the report is the JSON the caller parses, or that the number is the one the change
was for - and the manual pass that is meant to cover the rest is where "I ran it once and it looked
right" gets reported as evidence. A harness is a script that asserts the specific behaviour, prints
one line per claim, and is quoted in the report.

**This is not a step toward a test suite.** The tree carries none by owner decision, and a harness
is not the seed of one: it proves one change, it lives outside `Crate/`, and it is thrown away. Do
not add a test target, a `Test/` directory, or a CI gate under this skill's name.

---

## Procedure

1. **Write the claim as an observable before writing the script.** "The verb works" is not a claim.
   "`agito --help` lists `browse`", "the manifest for `@agito/chat-driver` reports `"ready": true`",
   "the second run emits no create line" are claims, and each one names the artefact that answers
   it.
2. **Run the real binary, not a re-implementation of it.** `Target/release/agito <verb>` or
   `cargo run -q -p agito -- <verb>`. A harness that reproduces the logic in Python and asserts
   against that is testing the harness.
3. **Prefer a fixture with one right answer.** A synthetic TOML file, a page with a known number of
   rows, a request that is deliberately malformed. A live system has many right answers and no way
   to say which one you got.
4. **Read the artefact's own output.** Parse the JSON the tool printed, count the rows in the file,
   read the exit status of the command itself. An exit code of 0 with nothing on stdout is a failure
   the status cannot see.
5. **One `PASS`/`FAIL` line per claim, and a non-zero exit when any fails.** The summary is what a
   future reader quotes: say how many checks ran and whether all of them held.
6. **Prove each check can go red.** Run it once against the state it is meant to reject, and see it
   fail. A check that has never failed has not been shown to assert anything.
7. **Name what you did not test.** A gap stated in the report is evidence about the boundary; a gap
   left implicit is a claim the run did not make.

---

## Keep no path, or exactly one

Every file a harness writes is a path in the diff the owner has to read. Two consequences:

- **The inline run is the default.** Pipe the checks straight into the interpreter and create
  nothing: `bash -s <<'EOF' … EOF` for shell, `node --input-type=module <<'EOF' … EOF` for
  JavaScript. The output is fresh in the turn and the tree is untouched.
- **When the checks outgrow a heredoc, keep one file and re-run it in place.** Creating a new
  verifier each turn adds a path per turn; deleting it adds another; a self-cleaning verifier can
  never leave the tree quiet. One file, re-run after each edit, is one path and one current result.

An identical invocation is not a re-run. Vary it (`env LC_ALL=C bash <script>`, one different
argument) and confirm the output is new before quoting it, or the green lines being read are from
the edit before last.

---

## When the effect is a command

A tool whose only product is what it would write - to an API, a repository, a database - is verified
by replacing its runner with a recorder: a function that appends the `argv` it was handed and
returns the fixture that command would have printed. Every assertion is then about a command that
was or was not issued, with no credentials, no rate limit and no cleanup. The worked instance, and
the four rules that keep it honest, are in `Reference/Recorder.md`.

---

## A green line has to be able to go red

The failures below are not style. Each one produced a report that said a change worked.

- **Polarity.** A check written `exit(j.missing ? 0 : 1)` under the name "tab present" passes only
  when the tab is absent, and reads exactly like a pass. Put the polarity in the check's own name
  (`header matches the read (reading / STALE)`), so a flipped condition is visible in the output.
- **`grep -q` under `set -o pipefail`.** The `-q` match closes the pipe, the producer dies of
  SIGPIPE (141), `pipefail` propagates 141, and a check whose subject is present reads red - so the
  hunt starts in correct code. Count with `grep -c`, which reads to EOF, or drop `pipefail` on that
  line.
- **An assumed path.** This workspace pins `target-dir = "Target"` in `.cargo/config.toml`, so the
  binary is `Target/release/agito`; a crate built without that config writes `target/`, and
  `.gitignore` carries both spellings for exactly that reason. Resolve the path from the tool
  (`cargo metadata --format-version 1`, `--message-format=json`) or test every spelling and print
  which one matched.
- **A relaxed assertion.** Loosening a check to tolerate a precondition (`assert the values moved`
  becomes `assert only when the read succeeded`) leaves the tolerated branch asserting nothing, and
  a reader returning null while the app reports it is running passes as green. Tolerate the state
  and assert its consequence explicitly; skipping is not tolerating.
- **A verdict derived from the harness's own log.** `fails=$(grep -cE 'exit=[1-9]' "$LOG")` answers
  0 both when every check passed and when the loop never wrote a line. Count in the loop, compare
  the count to the expected number, and keep the per-check lines for the reader only.
- **Verification through the thing being verified.** Assert from state, the trace or the rows, not
  from the panel the change added; two bugs then cancel into green. The exception is a claim about
  layout, where the geometry is the source of truth: assert `getBoundingClientRect` non-overlap and
  `getComputedStyle().position`.
- **A rendered read is not the file's bytes.** Tool output masks credential-like strings, so a line
  displayed as `Authorization: Bearer ***` may hold a template, and a finding built on the rendered
  string is a finding about the renderer. Measure before reporting: `grep -c`, the line's
  `length($0)`, or a hash of the committed bytes.

`Reference/False-Green.md` carries the longer catalogue - quoting traps, locale collation, `mktemp`,
log interleaving, editing a script mid-run, and the shell status mistakes that blame correct code.

---

## Reference

`Reference/Index.md` - the file list for this skill.

---

## Claims and how to test them

| Claim                                         | Test                                                        | Pass condition                                             |
| --------------------------------------------- | ----------------------------------------------------------- | ---------------------------------------------------------- |
| The harness ran                               | its own `PASS`/`FAIL` lines and its exit status             | one line per claim, exit 0 only when all hold              |
| The checks are behavioural                    | read the assertions                                         | each names an observable, not a syntax check               |
| Each check can fail                           | run it against the state it rejects                         | it reports `FAIL`, and for the stated reason               |
| The binary is the one just built              | `ls -l Target/release/agito` beside `cargo build --release` | the mtime follows the build                                |
| A green summary means every check ran         | compare the loop's counter to the expected count            | equal counts, and the count printed                        |
| A command-shaped tool issued the right writes | the recorder's captured `argv`                              | every asserted command appears, and no unasserted one does |
| Gaps are stated                               | the report                                                  | untested paths named, never implied green                  |
| The tree is unchanged by the harness          | `git status --porcelain` after the run                      | empty, or only the one kept verifier                       |

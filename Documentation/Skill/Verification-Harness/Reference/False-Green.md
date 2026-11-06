# False greens

Checks that passed while the change was broken, or failed while it was right. Each entry is the
mechanism, because the mechanism is what recurs; the command that produced it is the instance.

---

## Quoting

**Inline `node -e` is a trap.** Template literals, `${}` and backticks inside a shell-quoted `-e`
collapse into syntax errors that look like the probe is broken (`Unexpected token '<'`). Write the
probe to a file, or pipe a heredoc into `node --input-type=module`, and run that.

**A backtick inside a Rust or JavaScript template literal closes it early.** Page code carried in a
template string - a backtick in a comment naming a global, for instance - terminates the literal and
the whole install throws. Use plain quotes in any comment that travels inside a template.

---

## Paths and collation

**BSD `mktemp` only substitutes X's that END the template.** `mktemp verify-XXXXXX.sh` on macOS
exits 0 and creates a file literally named `verify-XXXXXX.sh`, so every run shares one path and the
"fresh" verifier is the same stale file. Put the X's last (`mktemp verify-XXXXXXXX`) and rename for
the extension. A file-writing tool then refuses the empty placeholder that `mktemp` just created -
remove it first, or write to a fresh name.

**`sort` orders by locale, not by byte.** A directory listing compared against a hard-coded
`"Cargo.toml Source Test module.json"` fails because the ambient collation puts `module.json` before
`Source`, and the failure reads as "unexpected top level" against a tree that is exactly right. Pin
the collation (`LC_ALL=C sort`) or compare as a set, and treat a whole-listing mismatch as a suspect
assertion before suspecting the tree: the tree is one fact, the expected string is a second one that
can be wrong on its own.

**A case-sensitive volume holds both spellings.** `Target/` and `target/` are two directories, and a
build that ran without the workspace's `.cargo/config.toml` writes the lowercase one. The artefact
was where cargo put it; the check reported "no binary" against a green build.

---

## Shell status

**`! Command; [ $? -eq N ]` tests the status of the negation.** The negation succeeds when the
command fails, so `$?` is 0 and a correct refusal reads as `FAIL` - the harness blames the tool for
behaving as specified. Capture the status on the line after the command (`rc=$?`) and assert `$rc`.

**`grep -q` under `set -o pipefail` reports a match as a failure.** `-q` closes the pipe, the
producer dies of SIGPIPE (141), and `pipefail` makes the pipeline exit 141. Count instead.

**The canonical command is run bare, not piped into a filter.** Piping it into an output-trimming
filter gives the pipeline's status to the filter, so the "exit 0" being reported belongs to `tail`.
Run the command, keep its status, and shorten the output afterwards.

---

## Logs

**A verdict derived from a grep over the harness's own log reads "no lines" as "no failures".** The
transcript that proves the run happened is the same file the verdict is read from, so a truncated,
empty or half-written log reports green. Count in the loop (`checks=$((checks+1))`,
`fails=$((fails+1))` on a non-zero status), compare against the expected number, and treat a
mismatch as a failure of the harness. Make the loop stub-switchable (`STUB=1` in place of the real
command) so this logic is provable in a second without running the checks it guards.

**Never redirect a grep into the file it is reading.** `grep -E '…' "$LOG" | tee -a "$LOG"`
truncates its own input mid-scan and appends the partial result, so the summary is whatever
survived.

**Never let two runs share one log.** A second concurrent run interleaves its exit lines into the
first's file, so "8 checks" becomes 16 and no line is attributable to a run. Lock the log
(`mkdir "$LOG.lock"` and refuse the second holder) and write the summary from the counters.

---

## State

**A probe that reads a state it did not set is testing the previous run.** Force the precondition,
wait for the system to observe it, then read. A label read before the state was forced reports the
last run's value and looks like a product defect; so does a control asserted before the paint that
renders it.

**A `% N` sampler over a growing collection records one sample and stops.**
`if (Trace.length % 10 === 0) Trace.push(…)` pushes once (0 becomes 1) and never again, because the
length it tests is the one the push keeps changing. Keep an explicit counter. The bug reads as "the
loop barely ran" and hides the trace the harness exists to inspect.

**Editing a script while it is executing corrupts that run.** A shell reads a script by byte offset,
so an edit landing mid-run resumes in the middle of a shifted line: the run emits parse errors for
syntax that is valid on disk (`printf: --: invalid option`) and stops writing where it happened to
be. Edit between runs; when a running one must change, kill it first.

---

## What a check is allowed to assert

**A test asserting your own convenience is a test of nothing.** An assertion shaped around what the
code happens to emit - a hard maximum on a metric that legitimately spikes on one tick - fails on
correct code. Assert the shape the requirement has (mean and p90, not a max) and say why in the
check's name.

**Mocking the untestable proves the mock.** If the endpoint cannot be reached, do not assert against
a stub and call it covered. Record the gap in the report and say what is untested; a green run that
never touched the network is worse than a red one that did.

**A harness that reads a file it does not own is not a harness.** A file can vanish between two
commands when something else is rewriting the tree, and the check then reports a code defect that is
not one. Scope the harness to files the change owns, or copy the input to a fixture first.

**A truncated write is not a short write.** A report that ends mid-object parses as invalid JSON,
which is a different failure from a report missing a field. Check the parse before checking the
fields, so the two are told apart in the output.

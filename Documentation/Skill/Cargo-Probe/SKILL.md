---
name: cargo-probe
description: Use when cargo, TOML or regex behaviour must be known. Probe, do not guess.
version: 1.0.0
license: MIT
platforms: [macos, linux]
metadata:
    hermes:
        tags: [cargo, rust, probe, toml, regex, isolation]
        related_skills: [verification-harness, crate-extension]
---

# Cargo Probe

A bug report says a parser rejects an em dash, a reviewer says a pattern is invalid, and the fix is
written against the belief rather than the behaviour - so a correct file gets edited, or a broken
pattern gets embedded in a crate and only fails on the input nobody tried. A dependency's behaviour
on an exact input is a question cargo can answer in one run, in a crate that touches nothing. This
skill is that crate.

---

## Where the probe lives

**Outside the workspace tree.** The workspace writes its `members` out by name, so a crate created
under `Crate/` is not a member - and `cargo run` inside it refuses before compiling anything:

```
error: current package believes it's in a workspace when it's not:
current:   <path>/Crate/Group/Probe/Cargo.toml
workspace: <path>/Cargo.toml
```

Cargo's own advice is to add the probe to `members`, or `exclude` it, or give its manifest an empty
`[workspace]` table. Adding it to `members` changes what `cargo build --workspace` builds, which is
the one thing a probe must not do. Put the probe beside the tree instead - `<path>/Probe/` - and the
question does not arise.

**When it has to live inside, declare the empty table.** A probe at `Crate/Group/Probe/` with

```toml
[workspace]
```

in its manifest is its own workspace root, and `cargo run` works without a single edit to the real
manifest. Verified: without the table the error above, with it the probe runs.

---

## Procedure

1. **One question per probe.** Name it after the question (`Regex-Check`, `Toml-Probe`) and write
   `Probe/src/main.rs`.
2. **Match the tree's edition.** The workspace declares `edition = "2024"`, and edition changes
   parsing and name resolution - a probe on `2021` can answer a question the tree is not asking.
3. **Pin the dependency to the version the tree resolves**, read from `Cargo.lock`, not from memory
   and not from the latest on crates.io:

    ```sh
    grep -A1 '^name = "regex"' Cargo.lock
    ```

    A probe on a different minor version tests a different library.

    ```toml
    [package]
    name = "regex-check"
    version = "0.1.0"
    edition = "2024"

    [dependencies]
    regex = "1"
    ```

4. **Embed the exact input, byte for byte.** Never normalise: keep the em dash (U+2014), the UTF-8
   BOM, the non-breaking space, the trailing whitespace. Normalising "repairs" the very input the
   bug is about, and the probe then answers a question nobody asked.
5. **Call the library API directly.** `Regex::new(…)`, `.parse::<toml::Table>()`, a `tracing`
   dispatch check. No test framework, no assertions library - the point is the library's behaviour,
   not a harness around it.
6. **Print one machine-checkable marker per case, and include a control that is genuinely broken.**
   `PARSE_OK` / `PARSE_ERR: {e}` / `ALL_OK`. The control is what tells you the error shape on input
   that really is invalid, so a real error is recognisable next to a false one.
7. **Run it and read the markers.** `cargo run -q` inside the probe directory. The marker lines are
   the decisive fact; prose about what the library "should" do is not.
8. **Delete the probe.** It answered one question.

---

## Lessons

- **The exact bytes are the experiment.** A probe that trims the input, or retypes it from the
  report, is testing a reconstruction.
- **Markers and exit codes, never prose.** A panic on the first invalid pattern is the answer when
  the marker that would have printed after it never appears - so make the last line of a passing run
  unmissable (`ALL_OK`).
- **One run answers several hypotheses.** The valid case, the failing case and the control belong in
  the same binary: the comparison between them is what identifies the cause.
- **Cargo needs the network only for the first fetch.** Reruns are offline, so a probe is cheap to
  keep for the length of a session and cheap to throw away.
- **A remembered behaviour is a claim about a version.** A crate that rejected a leading UTF-8 BOM
  on one release accepted it on the next; the marker says what this tree resolves, and the memory
  says what some earlier one did.
- **A probe that needs the tree's own types is a different exercise.** If the question is about this
  workspace's code rather than a dependency's, the answer belongs in `Verification-Harness/`, run
  against the built binary - not in a scratch crate that re-implements the type.

---

## Reference

`Reference/Index.md` - the file list for this skill.

---

## Claims and how to test them

| Claim                                           | Test                                                                                                |
| ----------------------------------------------- | --------------------------------------------------------------------------------------------------- |
| A probe outside the tree builds and runs        | `cargo run -q` in `<path>/Probe` prints its markers with no edit to the workspace                   |
| A probe inside the tree is not a member         | `cargo run -q` under `Crate/` fails with "believes it's in a workspace when it's not"               |
| An empty `[workspace]` table isolates it        | the same probe with `[workspace]` appended runs, and `cargo build --workspace` is unchanged         |
| The pinned version is the tree's                | the version in the probe's manifest equals the one `grep -A1 '^name = "<crate>"' Cargo.lock` prints |
| The edition matches the tree                    | the probe's manifest says `edition = "2024"`, as `[workspace.package]` does                         |
| A marker means the case ran                     | break the input deliberately; the control prints `PARSE_ERR` instead of the success marker          |
| The answer came from the library, not the probe | the probe contains no logic the library does not have - read `main.rs` before quoting it            |

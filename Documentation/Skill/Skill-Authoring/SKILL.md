---
name: skill-authoring
description: Use when writing, moving or checking a skill in this corpus. Layout, digest, gate.
version: 1.0.0
license: MIT
platforms: [macos, linux]
metadata:
    hermes:
        tags: [skills, authoring, validation, corpus, methodology]
        related_skills: [workspace, diagrams]
---

# Skill Authoring

A skill that is fluent, unverifiable and in the wrong tree costs twice: the reader follows a command
someone paraphrased, and the next session never sees the file at all. This corpus has one layout and
one validator, and both are cheap to satisfy once they are known.

## The contract

`agito skills Documentation/Skill` reports rather than returning early, and it checks three rules:

- **parsable** - `SKILL.md` exists, its frontmatter block is closed, it is a mapping, it carries a
  non-empty `description`, and its `name` equals the directory lowercased (`Diagrams/` is
  `name: diagrams`);
- **cited** - every file under `Reference/` is named in `Reference/Index.md`;
- **listed** - the corpus root's index names the skill: `Documentation/Skill/Index.md`, or
  `AGENTS.md`.

A finding is a rule, a skill and a sentence, and an empty finding list is a valid corpus.

```sh
agito skills Documentation/Skill
agito skills Documentation/Skill --json
```

The command also prints a digest. It is `sha256:` over the canonical JSON
`[[name, description, references], …]` - no body text, no path, no modification time - so two
checkouts of the same corpus agree on it, and it moves when a skill is added, renamed, re-described
or gains a reference file. It does not move when only the prose changes, which is why a stale
description survives a green check.

`Documentation/Skill/Index.md` belongs to whoever owns the corpus. Adding a skill is a directory
plus `SKILL.md`; the index row is a separate edit, and the `listed` rule reports the skill until it
lands.

## The five principles

**1. Commands are variables, never prose.** Copy every command, flag, path and identifier
byte-stable. Do not paraphrase, clarify or modernize one. If the source has no command where one is
needed, write UNKNOWN rather than inventing one, and a sentence that asserts live system state
carries its probe in the same paragraph or is marked CLAIM.

**2. Relations over fluency.** English hides relations behind verbs. Every "not" in the source
becomes two sentences: what it is, and what it is not. Every prohibition keeps its reason in the
next sentence, because a rule without a reason is the first one dropped. Never write "verified",
"works" or "exists" unless the same paragraph holds the command that would prove it.

**3. Size is grammar.** A bullet stays in `SKILL.md` when violating it causes a wrong write, a lost
commit or a false success. A bullet moves to `Reference/` when it is a worked instance of a rule
already stated - the one-line rule and a pointer stay in the body. A body of about 200 lines is the
shape here; a page of narrative is a page the reader skims, and the claim-to-test table is what they
were looking for.

**4. Lessons, not logs.** No dates, no incident narration, no "we once saw". A scar is allowed as a
test case, never as a story. Every success claim pairs with the probe that would falsify it.
Forbidden endings: "in short", "be careful", "make sure", and a summary paragraph that restates the
body.

**5. Placement discipline.** A skill that describes this repository lives in
`Documentation/Skill/<Name>/`, beside the code it describes, and travels with it. A method that
holds in any repository can live in a profile tree instead, and there it may name the machine it
runs on, because it never leaves that machine. The test is one question: does this file travel with
the repository? Yes means the repo tree, anonymised; no means the profile. A file written to the
wrong tree is invisible rather than merely untidy - not in the corpus, not in the index, not in the
next clone.

## Frontmatter

```yaml
---
name: skill-authoring
description: Use when writing, moving or checking a skill in this corpus. Layout, digest, gate.
version: 1.0.0
license: MIT
platforms: [macos, linux]
metadata:
    hermes:
        tags: [skills, authoring, validation, corpus]
        related_skills: [workspace, diagrams]
---
```

`name` is the directory lowercased, so a directory rename is a frontmatter edit. `description` is
one line stating the trigger before the content, because a reader chooses a skill by that line
alone. `related_skills` names skills that exist in this corpus, by their lowercased name.

## References

`Reference/Index.md` lists every file in the directory, one line each, saying what it carries. A
file present but unlisted is an `unlisted-reference` finding; a file listed but absent is a dead end
the reader finds before the validator does. Add a reference when a worked instance outgrows a rule,
not before.

## The format gate

```sh
node node_modules/prettier/bin/prettier.cjs --check Documentation/Skill/<Name>
```

`prettier.config.mjs` sets `printWidth: 100`, `useTabs: true` and `proseWrap: "always"`, so write
the body wrapped at 100 columns and align the tables by hand. A failure is a real defect. A pass is
not evidence of style: the check answers whether the formatter is content with the file, and it
cannot tell a well-shaped skill from a ragged one. Read the body once against the five principles
before believing the exit code.

## Pitfalls

- **An unquoted colon in a value kills the block.**
  `description: Drive the pipeline: scrape, build.` is invalid YAML, and a line grep for `^name:`
  still passes while the loader reads nothing. Quote any value containing `: `, and validate with a
  parser rather than with grep.
- **A description past the print width is a formatting casualty.** A formatter that reflows
  frontmatter wraps the value onto a continuation line, and a line-based reader then sees an empty
  description while the file looks correct in the editor. Keep every value on one line under 100
  columns.
- **The validator does not read the body.** A skill with no claim-to-test table, no reason beside a
  prohibition and a dated story in the middle passes all three rules, because the corpus contract
  checks structure and the house style does not. The gate is a floor.
- **A body edit does not move the digest.** Editing prose changes nothing the digest covers, so a
  description that no longer matches the body stays green; re-read the description whenever the
  body's scope changes.

## Claims and how to test them

| Claim                             | Test                                                                                          |
| --------------------------------- | --------------------------------------------------------------------------------------------- |
| The corpus passes its own gate    | `agito skills Documentation/Skill` prints no findings                                         |
| The gate reports, not stops       | a skill with an unclosed frontmatter block still yields findings for the others               |
| `name` must match the directory   | rename a copy of a directory without editing `name` and the gate reports that skill           |
| An unlisted reference is caught   | add `Reference/Probe.md` and leave it out of the index; the gate reports `unlisted-reference` |
| The digest covers metadata only   | run `--json`, edit one body sentence, run it again; the digest is unchanged                   |
| The digest covers the description | run `--json`, change one `description` value, run it again; the digest changes                |
| The formatter accepts the tree    | `node node_modules/prettier/bin/prettier.cjs --check Documentation/Skill` exits `0`           |

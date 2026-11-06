# A decision corpus for Agito

**Status:** proposal, and its gate is gone. Nothing here is a decision until accepted; no record,
directory or board exists. The proposal put the check in `Crate/Agito/Test/Corpus.rs` under
`cargo test --workspace`, and both the test target and `cargo test` are deleted - so if this is ever
accepted, the enforcement half has to be re-decided rather than resumed.

---

## 1. Whether, and at what size

**Recommendation: yes - five records, and no sixth until a failure owes one.**

The price is one test about the size of the diagram check that used to exist (277 lines with its own
cases, no new dependency), five records of roughly forty lines, and one standing obligation:
`status` must stay true, which no gate fully checks.

The need: `Documentation/Review.md` found this tree to be "a second architecture that agrees with
the first only in vocabulary" - the CLI reaches browser operations without the lease engine,
`WithLease` fences the caller and not the destination, and the database-as-authority design has no
implementation. Prose did not fail there. Prose has no owner and no path, so nothing was ever asked
to disagree with it. A record with `owner`, `applies_to` and `status: proposed` would have said,
inside the tree, that the design was described and not built.

Five, not TimeLine's 76: that corpus is a publication surface, read by people outside the
repository. This one is read by whoever changes the tree. The size that follows is _one record per
decision a crate can be wrong about_, and five is what the review's three findings plus two blocking
questions actually owe:

- **The fence an effect carries** - lease and generation identifiers travel with the effect and are
  checked where the write lands, not by the wrapper that admitted it (`Review.md`).
- **One authority over a transition** - a reducer authorises a transition, and no path reaches an
  effect around it (`Review.md` claims 1 and 3).
- **What a generation is** - the unit a conformance run is scoped to and the boundary a mutation
  crosses; `Reflection.md` uses "generation id" throughout and never defines it.
- **The manifest is a type, not a file** - admission validates it through a serde struct, not by
  reading strings in a test (`Open-Questions.md` Q19: "a JSON file nobody has mapped").
- **Transport identity** - a transport addresses by identity, never by position, and an unresolved
  target is an error rather than index 0 (`Open-Questions.md` Q9/Q10).

Two edges hold among the five, and they are the first real content of `depends`: the fence depends
on one authority and on what a generation is - which is also the writing order. The other two can go
whenever. All five start `proposed`, so the corpus opens with zero `settled` records: that is the
finding, not a gap.

**Rejected as a sixth: port ownership.** `Design.md` argues it and
`Crate/Agito/Test/Boundaries.rs` enforces it; a record would be its third statement.

---

## 2. The fields

```yaml
title: The fence an effect carries
date: 2026-11-04
status: proposed
owner: Crate/RunTime
applies_to: [Crate/RunTime, Documentation/Observability.md]
enforced_by:
supersedes: []
superseded_by:
depends: []
```

- **`title`** - one imperative sentence; the file is `Documentation/Decision/<title-slug>.md`, so
  nothing is numbered.
- **`date`** - the day the claim last changed, not the day it was created.
- **`status`** - `proposed` (asserted, unenforced), `settled` (`enforced_by` names the test),
  `superseded` (`superseded_by` names the replacement).
- **`owner`** - the one repository path that must change if the decision changes; never a person.
- **`applies_to`** - every repository-relative path the decision binds; each must exist.
- **`enforced_by`** - `Crate/<X>/Test/<Y>.rs::<fn>`; required when `settled`, forbidden otherwise.
- **`supersedes`** - titles this record replaces; empty otherwise.
- **`superseded_by`** - the title that replaced this one; set if and only if `status: superseded`.
- **`depends`** - titles that must hold for this record to make sense.

**`category`: out.** A closed set invented before a sixth record exists is invented for consistency,
which is the failure the brief names. `applies_to` already answers the only question a category
answers - which crate this binds - and TimeLine's categories sort a board's columns, a surface this
corpus does not have.

**A reversal-cost axis: out.** It is a judgement the author fills in for a scheduler to read, and
there is no scheduler. What carries the same information mechanically is `depends`: a decision
expensive to reverse is one other records rest on, an edge the gate checks rather than a scalar
nobody reads.

**`slug` and `source`: out.** At five records the title is already unique, and a source field would
make a capture look like the authority for a decision the capture does not make.

---

## 3. The documents

**All four stay prose. None is absorbed, none is superseded.** A record carries one claim and nine
fields; these documents carry provenance per claim, an argument, and a record of having changed
their mind. Absorbing them would take fifteen records and drop the distinction that makes them
trustworthy, so a record that restates one is refused as duplication.

- **`Design.md`** - stays. The layer table, manifest state classes and package order are arguments;
  the records take only the narrow enforcement statements the prose leaves open.
- **`Observability.md`** - stays. It states the effect/state split; the fence record states what
  crosses that split, which the document does not.
- **`Reflection.md`** - stays. It names a generation id and a conformance run scoped to it; the
  generation record defines the term.
- **`Scope.md`** - stays, and owes one correction on acceptance: its parity row says this workspace
  has no "decision corpus with a contract", which becomes false the moment the first record lands.
  This run may not edit it, so that is the first follow-up.

---

## 4. The board

**No board.** A board triages work; a record states a decision. The records that block work are
already cards, and `status` is not work state - conflating the two is how TimeLine's board and its
corpus came to need a sync whose disagreement is a bug. `Scope.md` rules the direction out for a
stronger reason than this proposal has: Agito "does not grow a copy of the site, the archive or
the RFC board". No triage need is named here that the existing board cannot serve, so
`rfc_board.py`'s model is neither copied nor adapted.

---

## 5. The gate

**`cargo test --workspace`**, running a `Corpus` test target beside the four others the tree had.
Every one of those targets is deleted, so this section states a design that has no host.

`Script/Feed.mjs` is the discipline the brief names, and it was deleted with the Node toolchain,
which is the lesson `README.md` states: "a gate that lives in the toolchain being deleted is a gate
that leaves with it". Its successor in discipline is `Crate/Agito/Test/Legend.rs`, which is
`Script/Legend.mjs` in Rust, a test rather than a script "because the gate it replaces was a command
somebody had to remember". The corpus gate takes that shape: no Node, no Python, no new toolchain,
one command that already runs.

It rejects a record that:

- is missing `title`, `date`, `status`, `owner` or `applies_to`;
- carries a `status` outside `proposed` / `settled` / `superseded`, or a `date` not `YYYY-MM-DD`;
- repeats a `title` another record already has, or has a file name that is not its title's slug;
- names an `applies_to` path that does not exist from the repository root, or an `owner` that is not
  one of its own `applies_to` paths;
- carries `superseded_by` without `status: superseded`, or `status: superseded` without it;
- is named in another record's `supersedes` while not naming that record back;
- names in `supersedes`, `superseded_by` or `depends` a title that does not exist, or reaches itself
  through `depends`;
- is `settled` without an `enforced_by` whose test file exists and declares that function, or
  carries `enforced_by` while `proposed`.

Exit behaviour is `Legend.rs`'s: collect `Problems`, then `assert!(Problems.is_empty(), "…")`, so a
malformed record fails the test binary with a non-zero exit and one line per problem, sorted by
file. An empty directory passes, deliberately: the gate must be addable before the first record
exists, or adding it would force writing one.

What it cannot check: that a `settled` record's test enforces its claim, that a record is not a
paraphrase of a document, and that a deleted record was meant to go. Those are review, and the diff.

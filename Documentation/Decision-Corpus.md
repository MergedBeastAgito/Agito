# A decision corpus for Agentic

**Status:** proposal. Nothing here is a decision until it is accepted, and no record, directory or
board exists.

---

## 1. Whether, and at what size

**Recommendation: yes, five records, and no sixth until a failure owes one.**

The cost, honestly:

- **One test, ~180 lines of Rust.** `Crate/Agentic/Test/Legend.rs` is 277 lines including its own
  cases; a corpus gate is the same shape and no new dependency.
- **Five records, ~40 lines each.** The writing is the cheap part.
- **The real price: `status` must stay true.** A `settled` record claims something enforces it.
  Keeping that claim honest is an obligation on every future change, and it is the one field a gate
  can only partly check (§5).

Why it is worth that: `Documentation/Review.md` found this tree to be "a second architecture that
agrees with the first only in vocabulary" - the CLI reaches browser operations without the lease
engine, `WithLease` fences the caller and not the destination, and the database-as-authority design
has no implementation. Prose did not fail there. Prose has no owner and no path, so nothing was ever
asked to disagree with it. A record with `owner`, `applies_to` and `status: proposed` would have
said, inside the tree, that the design was described and not built.

Why not TimeLine's 76: that corpus is a publication surface - records become issues on a board and
are read by people outside the repository. This one is read by whoever changes the tree. The size
that follows is _one record per decision a crate can be wrong about_, and five is what the review's
three findings plus two open blocking questions actually owe.

The five, by subject:

- **The fence an effect carries** - lease and generation identifiers travel with the effect and are
  checked where the write lands, not by the wrapper that admitted it. `Review.md` §The revision it
  proposes reaches this conclusion; the code does not do it yet.
- **One authority over a transition** - a reducer authorises a transition, and no path reaches an
  effect around it. `Review.md` claims 1 and 3.
- **What a generation is** - the unit a conformance run is scoped to and the boundary a mutation
  crosses. `Reflection.md` uses "generation id" throughout and never defines it.
- **The manifest is a type, not a file** - admission validates a manifest through a serde struct,
  not by reading strings in a test. `Open-Questions.md` Q19, still open: "a JSON file nobody has
  mapped".
- **Transport identity** - a transport addresses by identity, never by position, and an unresolved
  target is an error rather than index 0. `Open-Questions.md` Q9/Q10, named there as the cheapest
  blocking question to settle first.

All five start `proposed`, so the corpus opens with zero `settled` records. That is the finding, not
a gap in this proposal: nothing in the tree enforces any of these today.

**Rejected as a sixth: port ownership.** `Design.md` argues it and
`Crate/Agentic/Test/Boundaries.rs` enforces it. A record would be the third statement of one
decision - exactly the duplication this corpus must refuse.

---

## 2. The fields

```yaml
---
title: The fence an effect carries
date: 2026-11-04
status: proposed
owner: Crate/Runtime
applies_to: [Crate/Runtime, Documentation/Observability.md]
enforced_by:
supersedes: []
superseded_by:
depends: []
---
```

- **`title`** - one sentence naming the decision, in the imperative; the file is its lowercase with
  hyphens for spaces. It is the record's identity, so nothing is numbered.
- **`date`** - the day the record last changed its claim, not the day it was created.
- **`status`** - `proposed` (asserted, nothing enforces it), `settled` (`enforced_by` names the
  test), `superseded` (`superseded_by` names the replacement).
- **`owner`** - the one path that must change if this decision changes. A path, never a person.
- **`applies_to`** - every repository-relative path the decision binds; each must exist.
- **`enforced_by`** - `Crate/<X>/Test/<Y>.rs::<fn>`; required when `settled`, forbidden otherwise.
  The gate checks the file exists and the function is declared, not that it enforces anything.
- **`supersedes`** - titles this record replaces; empty otherwise.
- **`superseded_by`** - the title that replaced this one; set if and only if `status: superseded`.
- **`depends`** - titles that must hold for this record to make sense.

**`category`: out.** A closed set invented before the sixth record exists would be invented for
consistency, which is the failure the brief names. `applies_to` already answers the only question a
category answers - which crate does this bind - and TimeLine's categories sort a board's columns, a
surface this corpus does not have. Add one when a reader asks a question `applies_to` answers
wrongly.

**A reversal-cost axis: out.** A reversal cost is a judgement the author fills in for a scheduler to
read. There is no scheduler and no board here, so nothing consumes the value and it drifts. What
carries the same information mechanically is `depends`: a decision that is expensive to reverse is
one other records rest on, and those are edges the gate checks rather than a scalar nobody reads.

**`slug`: out.** At five records the title is already unique, and a second name for one thing is a
second thing to keep in sync.

**`source`: out.** A record states a decision this repository owns. Where a capture informed it, the
body cites it the way `Design.md` does; the field would make a capture look like the authority for a
decision the capture does not make.

---

## 3. The documents

**All four stay prose. None is absorbed, none is superseded.** A record carries one claim and nine
fields; these documents carry provenance per claim (`**Source:**` or
`**Decision: this repository**`), an argument, and a record of having changed their mind. Absorbing
them would take fifteen records and would drop the distinction that makes them trustworthy - so a
record that restates one is refused.

- **`Design.md`** - stays. The layer table, the manifest state classes and the five-package order
  are arguments; the records above take only the narrow enforcement statements the prose leaves
  open.
- **`Observability.md`** - stays. It states the effect/state split; the fence record states what
  crosses that split, which the document does not.
- **`Reflection.md`** - stays. It names a `generation id` and a conformance run scoped to it; the
  generation record defines the term.
- **`Scope.md`** - stays, and needs one correction on acceptance: its parity row says this workspace
  has no "decision corpus with a contract". That becomes false the moment the first record lands.
  This run may not edit it, so it is owed as the first follow-up.

---

## 4. The board

**No board.** A board triages work; a record states a decision. The `proposed` records that block
work are already cards, and a record's `status` is not work state - conflating the two is how
TimeLine's board and its corpus came to need a sync whose disagreement is a bug. No triage need is
named here that the existing board does not already serve, so `rfc_board.py`'s model is neither
copied nor adapted.

---

## 5. The gate

**`cargo test --workspace`**, running `Crate/Agentic/Test/Corpus.rs`, registered as
`[[test]] name = "Corpus"`.

The brief names `Scripts/Feed.mjs` as the discipline to follow. That file was deleted with the Node
toolchain, and its deletion is the lesson: `README.md` states it as "a gate that lives in the
toolchain being deleted is a gate that leaves with it". Its successor in discipline is
`Crate/Agentic/Test/Legend.rs`, which is `Scripts/Legend.mjs` in Rust - a test rather than a script,
"because the gate it replaces was a command somebody had to remember". The corpus gate takes that
shape: no Node, no Python, no new toolchain, one command that already runs.

It rejects a record that:

- is missing `title`, `date`, `status`, `owner` or `applies_to`;
- carries a `status` outside `proposed` / `settled` / `superseded`;
- carries a `date` that is not `YYYY-MM-DD`;
- repeats a `title` another record already has, or has a file name that is not its title's slug;
- names an `applies_to` path that does not exist from the repository root;
- carries `superseded_by` without `status: superseded`, or `status: superseded` without it;
- is named in another record's `supersedes` while not naming that record back;
- names in `supersedes`, `superseded_by` or `depends` a title that does not exist, or reaches itself
  through `depends`;
- is `settled` without an `enforced_by` whose test file exists and declares that function;
- carries `enforced_by` while `proposed`.

Exit behaviour is `Legend.rs`'s: collect `Problems`, then `assert!(Problems.is_empty(), "…")`, so a
malformed record fails the test binary with a non-zero exit and one line per problem, sorted by
file. An empty directory passes, deliberately - the gate must be addable before the first record
exists, or adding it would force writing one.

What it cannot check: that a `settled` record's test enforces its claim, that a record is not a
paraphrase of a document, and that a deleted record was meant to go. Those are review, and the diff.

# Rings

The crates as concentric authority: what may know what, drawn as rings rather than as a dependency
list. Each page below is separated by a rule; later pages are the intended shape, not the current
one.

## Page 1 - the rings as they are intended

![The crates as authority, drawn as concentric rings](Ring.svg)

[`Ring.mmd`](Ring.mmd) · [`Ring.svg`](Ring.svg)

The dotted edges are the point: a capability declares the port it needs and imports nobody.
`browser-cdp` is the only crate in Ring 0 that touches the outside world, and `spacetime-catalog` is
the only one that holds the rules the database enforces.

---

## Page 2 - what the rings are for

![Who may know what, from a capability to the composition root](Who-May-Know.svg)

[`Who-May-Know.mmd`](Who-May-Know.mmd) · [`Who-May-Know.svg`](Who-May-Know.svg)

A crate that knows more than its ring is a defect even when it compiles, which is why
`cargo tree -p agito-chat-driver | grep -c browser` must answer `0`. The boundary check that used
to fail an edge the design does not allow is deleted; the graph `cargo tree` prints is what is left.

---

## Page 3 - the ring that is missing

![A ticket issued by the database, presented to a destination that does not exist yet](Missing-Ring.svg)

[`Missing-Ring.mmd`](Missing-Ring.mmd) · [`Missing-Ring.svg`](Missing-Ring.svg)

`X` is drawn as an installable step because it does not exist yet. Until it does, the runtime crate
is a second authority wearing the same vocabulary - see `Documentation/Review.md` and the card that
names this gap.

---

## Page 4 - the ring that should disappear

![The runtime holding lease state beside the database that should hold it](Disappearing-Ring.svg)

[`Disappearing-Ring.mmd`](Disappearing-Ring.mmd) · [`Disappearing-Ring.svg`](Disappearing-Ring.svg)

The lease, the fence and the generation are internalities. A process that owns them is a parallel
system, however correct its fencing.

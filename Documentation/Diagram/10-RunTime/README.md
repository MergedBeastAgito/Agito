# Runtime

The lease and effect lifecycle, as it is and as it should be. Pages are separated by a rule; the
last page is the one with states nobody has built.

## Page 1 - the lease as it exists today

![Free, claimed, admitted, landed, expired and refused, with release closing the loop](lifecycle.svg)

[`lifecycle.mmd`](lifecycle.mmd) · [`lifecycle.svg`](lifecycle.svg)

`Refused` is the state the fence card added. It exists in the port and is exercised by tests;
whether it exists at a _destination_ is the open question.

---

## Page 2 - the late arrival, which is the counterexample

![Claim, admission, expiry, then a ticket presented to two different destinations](late-arrival.svg)

[`late-arrival.mmd`](late-arrival.mmd) · [`late-arrival.svg`](late-arrival.svg)

The page was a sequence diagram and is now a flowchart: `line9 render --out` serves flowcharts only,
and a `sequenceDiagram` exits `9`. The order of the five events is the sequence.

The second branch is what "fencing the caller" means. A write that landed is not retracted because
the record of it was refused.

---

## Page 3 - missing states

![Dispatched, ambiguous, reconciled and quarantined, none of which exist](missing.svg)

[`missing.mmd`](missing.mmd) · [`missing.svg`](missing.svg)

None of `Dispatched`, `Ambiguous`, `Reconciled` or `Quarantined` exists - they are drawn as
installable steps. They are the handover rule the review demanded: what happens to work already in
flight when its authority ends. `Refused` alone cannot cover it, because refusing a result says
nothing about the effect that already happened.

---

## Page 4 - who owns which state

![The process holding the transport and the effect, the database holding lease, phase and trace](ownership.svg)

[`ownership.mmd`](ownership.mmd) · [`ownership.svg`](ownership.svg)

The left column is effects. The right column is internalities. `Dispatched` and `Ambiguous` belong
on the right, which is why the crate cannot be the authority for them.

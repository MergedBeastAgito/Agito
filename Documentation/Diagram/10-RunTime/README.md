# RunTime

The lease and effect lifecycle: the states the crate moves, and the ones it does not model.

## Page 1 - the lease

![Free, claimed, admitted, landed, expired, held and refused, with release closing the loop](Lifecycle.svg)

[`Lifecycle.mmd`](Lifecycle.mmd) · [`Lifecycle.svg`](Lifecycle.svg)

`Held` is the port's `lease-held`: a claim refused because another holder owns the resource,
reported as a result rather than a panic. `Refused` is `admission-refused`, and the ticket a
destination rejects because it is stale is the counterexample page 2 draws. Whether a _destination_
has a matching state is one of the questions `Documentation/Open-Questions.md` records.

---

## Page 2 - the late arrival, which is the counterexample

![Claim, admission, expiry, then a ticket presented to two different destinations](Late-Arrival.svg)

[`Late-Arrival.mmd`](Late-Arrival.mmd) · [`Late-Arrival.svg`](Late-Arrival.svg)

The page is a flowchart rather than a sequence diagram: `line9 render --out` serves flowcharts only,
and a `sequenceDiagram` exits `9`. The order of the five events is the sequence.

The second branch is what fencing the caller means. A write that landed is not retracted because the
record of it was refused.

---

## Page 3 - the states around an expired lease

![Dispatched, ambiguous, reconciled and quarantined, of which two are modelled](Missing.svg)

[`Missing.mmd`](Missing.mmd) · [`Missing.svg`](Missing.svg)

Two of the four are in the port. `Ambiguous` is the record of a refusal that arrived after the
effect ran: it names the effect, the resource, the generation, the fence and the lease that admitted
it (`Crate/RunTime/Source/Struct/Port.rs:425`). The resource it names stays quarantined until a
resolver states which of the two outcomes it was (`Crate/RunTime/Source/Struct/StandIn.rs:281`).

`Dispatched` and `Reconciled` are drawn as installable steps because no state carries them. They
cover work already in flight when its authority ends, which `Refused` alone cannot: refusing a
result says nothing about the effect that already happened.

---

## Page 4 - who owns which state

![The process holding the transport and the effect, the database holding lease, phase and trace](Ownership.svg)

[`Ownership.mmd`](Ownership.mmd) · [`Ownership.svg`](Ownership.svg)

The left column is effects. The right column is internalities. `Dispatched` belongs on the right,
which is why the crate cannot be the authority for it; `Ambiguous` is the crate's own record that it
cannot know which side an effect landed on.

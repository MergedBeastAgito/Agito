# Browser

The CDP adapter's lifecycle: attach versus launch, and the failures a session that is not alive can
report.

## Page 1 - attach or launch

![Unbound becoming launched or attached, live, evaluated, unavailable or disposed](Lifecycle.svg)

[`Lifecycle.mmd`](Lifecycle.mmd) · [`Lifecycle.svg`](Lifecycle.svg)

The distinction that holds on every path: `dispose` on a launched session may close the browser; on
an attached session it must not, because somebody else owns it.

---

## Page 2 - the three failures, and the one that hides

![An operation branching into three named failures and one that looks alive](Failures.svg)

[`Failures.mmd`](Failures.mmd) · [`Failures.svg`](Failures.svg)

`looks alive` is the state the adapter cannot name: an open socket is not a live browser. The
liveness probe is what separates them, which is why the manifest gives this crate the
`HoldTransport` state class and an admission check.

---

## Page 3 - identity

![A URL and an endpoint reaching a session, and ownership deciding what dispose may do](Identity.svg)

[`Identity.mmd`](Identity.mmd) · [`Identity.svg`](Identity.svg)

Three things are called "the browser" here: an endpoint, a page, and a user. A `Target` carries its
ownership, and an endpoint is never treated as an identity.

---

## Page 4 - states the adapter does not model

![Stalled and reclaimed, the two states of a session that is not alive](Missing.svg)

[`Missing.mmd`](Missing.mmd) · [`Missing.svg`](Missing.svg)

`Stalled` and `Reclaimed` are drawn as installable steps because no state carries them. Both are
real: the first is the dead-but-open socket, the second is two debugger clients on one page - which
is what happens when a second tool attaches to a tab an extension already holds.

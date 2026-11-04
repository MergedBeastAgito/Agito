# Review of the first pass

An external review of the whole corpus - five batches, 111 files - was asked for after the Rust port
landed. This records its verdict and the defect it found, because the verdict is not a list of nits.

## The finding, in its own words

> The central problem is **authority**: the documents specify one coherent authority over admission,
> leases, generations and effects, but the code implements several disconnected approximations. The
> CLI can run browser operations without the lease engine; the lease engine authorizes an arbitrary
> future rather than fencing the destination; and the database-as-authority design has no
> corresponding implementation in the supplied Rust sources. These are not small omissions around an
> otherwise implemented contract. They are different architectures.

Three separate claims, each checkable:

1. **The CLI can bypass the lease engine.** A browser operation reachable from the composition root
   without going through admission is an effect with no authority behind it.
2. **`WithLease` fences the caller, not the destination.** The counterexample it gave:

    ```
    lease is live
    WithLease admits the callback
    callback waits
    lease expires
    callback performs the external write
    WithLease returns success
    ```

    The in-flight counter stops a second local claimant from taking over during that callback. It
    does not stop the old callback writing after expiry.

3. **The database-as-authority design has no implementation.** `Documentation/Observability.md` says
   a reducer is where a transition happens; the Rust sources contain no reducer call on that path,
   so the authority is described and not built.

## Why the existing test does not catch it

`a_claim_during_an_effect_on_an_expired_lease_is_refused` blocks takeover until the future settles
or is dropped. That substitutes "wait for the old effect" for the specification's "revoke authority
despite a physically live worker" - the test agrees with the implementation rather than with the
requirement.

And dropping the future decrements the in-flight count, while dropping a Rust future does not prove
the browser-side operation stopped. The count therefore measures unsettled local futures, not
unsettled external effects.

## The revision it proposes

Stop presenting arbitrary callback admission as fencing. An effect should carry its lease and
generation identifiers and be **checked by the destination** - the reducer that records it - rather
than trusted because a wrapper admitted it earlier. That is the same conclusion
`Documentation/Reflection.md` reached for schema generations, applied to effects: the check belongs
where the write lands.

## What this changes about the plan

The port is not finished; it is a second architecture that agrees with the first only in vocabulary.
The next card is not "more crates" but "one authority": admission, lease and generation checked at
the point of effect, with a test that fails when the check is moved back into the caller.

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

## The destination check

The second claim is answered where the write lands rather than in the wrapper. `WithLease` admits an
effect and hands the work a `Ticket` carrying the lease's fence and its generation; the work presents
that ticket to `Destination::Present`, which re-reads the record when the effect lands and refuses a
stale fence, a stale generation, a released lease or an expired record. A refusal after the effect
ran is recorded as `Ambiguous` against the resource and the resource is quarantined until a resolver
states whether the external action landed, because rejecting a result does not undo an effect.

`Crate/RunTime/Source/Struct/StandIn.rs` is the local double for that destination and mirrors the
reducer row for row. It is not the authority in production: the runner holds an `Authority` and
never reads the count of unsettled futures, so a composition root with a database substitutes a
reducer adapter for the stand-in.

That is the same conclusion `Documentation/Reflection.md` reaches for schema generations, applied to
effects: the check belongs where the write lands.

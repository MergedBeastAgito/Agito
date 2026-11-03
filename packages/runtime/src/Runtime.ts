/**
 * The implementation of the port in `Port.ts`: admit a run, lease a resource
 * with a fence, bound it with a budget, and clean up without throwing.
 *
 * Everything here is process-local policy. There is no transport, no capability
 * and no import from another package: the runtime is handed a generation and a
 * registration list and answers from that binding alone, which is what makes
 * "the manifest it reports is the manifest it enforces" checkable rather than
 * asserted.
 *
 * Three mechanisms carry the rules the port states:
 *
 * 1. **A record per resource, and a fence counter that outlives it.** The record
 *    is what is live now; the counter is what was ever issued. Deleting the
 *    record on release therefore cannot make an old number current again - the
 *    next claim takes the next number, and the stale worker's write is refused
 *    for a reason that survives the resource being free.
 * 2. **One serialised step per verb.** Claim, renew, release and the admission
 *    step of a write all mutate the same two maps. A check that is not atomic
 *    with the mutation it authorises is the race the fence exists to close: two
 *    claims that both read "free" both win. So each verb's bookkeeping runs alone
 *    on a promise chain. The write itself is the caller's and is not on the
 *    chain - holding the chain across it would deadlock a write that renews its
 *    own lease, and a slow write would stall every other resource.
 * 3. **Effects in flight are counted, and a takeover waits for them.** A fence
 *    proves ordering, not liveness: an expired worker can arrive before any newer
 *    token reaches the resource. Counting admitted-but-unsettled effects means a
 *    claim cannot take a resource out from under an effect that is still running,
 *    and a release cannot free one either. The cost is stated rather than hidden:
 *    an effect that never settles holds its resource until the process ends,
 *    because the runtime cannot tell a slow effect from a dead one, and refusing
 *    the takeover is the only answer that cannot double-commit.
 */
import {
	ErrorCode,
	RuntimeError,
	States,
	type Binding,
	type Budget,
	type Clock,
	type Lease,
	type Policy,
	type Registration,
	type StateClass,
} from "./Port.ts";

export type RuntimeOptions = {
	/** The generation the composition root bound this runtime to. */
	generation: number;
	/** What it may admit, by name, with the state class each declares. */
	capabilities: readonly Registration[];
	/** The clock leases and budgets are read on. Defaults to `Date.now`. */
	clock?: Clock;
};

/** The clock a caller that supplies none gets. */
const WallClock: Clock = { now: () => Date.now() };

/**
 * Distinguishes one runtime's leases from another's in the same process.
 *
 * Lease ids are built from this token, so a lease issued by one runtime is not
 * accepted by a second one that happens to have issued the same fence number for
 * the same resource name. Two runtimes are two authorities, and a fence number is
 * only meaningful inside the authority that counted it.
 */
let Instances = 0;

/** What the runtime knows about one live grant. */
type Run = {
	/** The id the lease was issued with; a presented lease must carry it. */
	id: string;
	/** The fence the resource is at while this record is the live one. */
	fence: number;
	/** The record's own expiry. A presented lease's copy is never trusted. */
	expiresAt: number;
	/** This run's allowance, owned by the runtime once the claim is admitted. */
	budget: Budget | null;
	/** Effects admitted under this record that have not settled yet. */
	inFlight: number;
	/** Set when the holder released while an effect was still in flight. */
	released: boolean;
};

/** A positive, finite duration, or a refusal. */
const Positive = (value: number, what: string): number => {
	if (typeof value !== "number" || !Number.isFinite(value) || value <= 0)
		throw RuntimeError.refused(
			`${what} must be a positive, finite number of milliseconds; got ${String(value)}`,
		);
	return value;
};

/**
 * A run's allowance, copied so the caller cannot move the finish line later.
 *
 * An omitted budget is no budget, and no budget is unbounded. The alternative
 * default - some small number the runtime picks - would be authority the caller
 * never granted, and a run refused for spending it would have no way to say why.
 */
const Allowance = (budget: Budget | undefined): Budget | null => {
	if (budget === undefined || budget === null) return null;
	const { deadline, operations } = budget;
	if (typeof deadline !== "number" || Number.isNaN(deadline))
		throw RuntimeError.refused(
			`a budget needs a numeric deadline; got ${String(deadline)}`,
		);
	if (
		typeof operations !== "number" ||
		!Number.isInteger(operations) ||
		operations < 0
	)
		throw RuntimeError.refused(
			`a budget needs a whole, non-negative operation count; got ${String(operations)}`,
		);
	return { deadline, operations };
};

export const Create = (options: RuntimeOptions): Policy => {
	if (!Number.isFinite(options.generation))
		throw new TypeError("a runtime is bound to one finite generation");

	const generation = options.generation;
	const clock = options.clock ?? WallClock;
	if (typeof clock.now !== "function")
		throw new TypeError(
			"a runtime reads one clock, and it must answer now()",
		);

	const capabilities = new Map<string, Registration>();
	for (const registration of options.capabilities ?? []) {
		if (!States.includes(registration.state))
			throw new TypeError(
				`${String(registration.name)} declares an unknown state class: ${String(registration.state)}`,
			);
		if (capabilities.has(registration.name))
			throw new TypeError(`${registration.name} is registered twice`);
		capabilities.set(registration.name, {
			name: registration.name,
			state: registration.state,
		});
	}

	const token = `r${++Instances}`;
	/** What is live now, by resource name. */
	const runs = new Map<string, Run>();
	/** The highest fence ever issued, by resource name. Never reset. */
	const fences = new Map<string, number>();
	/** The tail of the critical section. Every verb appends to it. */
	let tail: Promise<unknown> = Promise.resolve();

	/**
	 * Run one step alone.
	 *
	 * The step runs whether or not the previous one failed - a refused claim must
	 * not wedge the queue - and its outcome is handed to its caller, not to the
	 * chain. The chain only ever sees "settled", so one rejection cannot poison
	 * every later verb.
	 */
	const Serialise = <T>(step: () => T | Promise<T>): Promise<T> => {
		const result = tail.then(step, step);
		tail = result.then(
			() => undefined,
			() => undefined,
		);
		return result;
	};

	/**
	 * The admission predicate, evaluated at the effect.
	 *
	 * Five checks, ordered so the reason is the true one:
	 *
	 * 1. **No record** - nothing is held under that name any more.
	 * 2. **Fence mismatch** - the caller's authority was superseded, and the
	 *    newer holder's fence is what the resource now answers to. This is
	 *    `lease-held`: somebody else holds it, and the caller's copy is history.
	 * 3. **Id mismatch** - the fence is current but the lease was not the one
	 *    issued for it, which happens when a lease is carried between two runtimes
	 *    or hand-built. There is no record of that grant, so it is refused.
	 * 4. **Released** - the holder gave the authority back while an effect was
	 *    still running, so the record survives only to account for that effect.
	 * 5. **Expired** - the fence still matches, the lease is still the issued one,
	 *    and authority ended anyway. This is the check that makes a worker's late
	 *    write fail rather than land: the worker is alive, the authority is not.
	 */
	const Resolve = (lease: Lease, now: number): Run => {
		if (!lease || typeof lease.resource !== "string")
			throw RuntimeError.refused(
				"a lease is the one the runtime issued: resource and fence are required",
			);
		const run = runs.get(lease.resource);
		if (!run)
			throw RuntimeError.refused(
				`no lease is live for ${lease.resource}`,
			);
		if (run.fence !== lease.fence)
			throw RuntimeError.held(
				`${lease.resource} is held under fence ${run.fence}, not fence ${lease.fence}`,
			);
		if (run.id !== lease.id)
			throw RuntimeError.refused(
				`the lease presented for fence ${run.fence} of ${lease.resource} was not the one issued`,
			);
		if (run.released)
			throw RuntimeError.refused(
				`the lease on ${lease.resource} was released`,
			);
		if (now >= run.expiresAt)
			throw RuntimeError.refused(
				`the lease on ${lease.resource} expired at ${run.expiresAt}, and the clock reads ${now}`,
			);
		return run;
	};

	/**
	 * Spend one operation of the run's allowance.
	 *
	 * Charged before the effect, not after: an effect that throws was still
	 * attempted, and a count that only moved on success would let a run that fails
	 * in a loop spend nothing. The charge happens in the same serialised step as
	 * the fence check, so two effects cannot both read the last operation.
	 */
	const Charge = (run: Run, now: number): void => {
		const budget = run.budget;
		if (!budget) return;
		if (now >= budget.deadline)
			throw RuntimeError.spent(
				`the run's deadline passed at ${budget.deadline}, and the clock reads ${now}`,
			);
		if (budget.operations <= 0)
			throw RuntimeError.spent(
				"the run has spent every operation it was granted",
			);
		budget.operations -= 1;
	};

	/**
	 * Account for an effect that has settled.
	 *
	 * A record released while it had an effect in flight survives until that
	 * effect settles, and is deleted then. It is also the point at which the
	 * resource becomes claimable again, which is why the delete is conditional on
	 * the record still being the live one: a later claim may already have replaced
	 * it.
	 */
	const Settle = (resource: string, run: Run): void => {
		run.inFlight -= 1;
		if (run.released && run.inFlight === 0 && runs.get(resource) === run)
			runs.delete(resource);
	};

	const claim = (
		resource: string,
		ttlMs: number,
		budget?: Budget,
	): Promise<Lease> =>
		Serialise(() => {
			if (typeof resource !== "string" || resource.length === 0)
				throw RuntimeError.refused(
					"a claim names the resource it wants",
				);
			const ttl = Positive(ttlMs, "a lease ttl");
			const allowance = Allowance(budget);
			const now = clock.now();
			const current = runs.get(resource);
			if (current) {
				if (current.inFlight > 0)
					throw RuntimeError.held(
						`${resource} has an effect in flight under fence ${current.fence}; a takeover waits for it to settle`,
					);
				if (now < current.expiresAt)
					throw RuntimeError.held(
						`${resource} is held under fence ${current.fence} until ${current.expiresAt}`,
					);
			}
			const fence = (fences.get(resource) ?? 0) + 1;
			fences.set(resource, fence);
			const lease: Lease = {
				id: `${token}#${resource}#${fence}`,
				resource,
				fence,
				expiresAt: now + ttl,
			};
			runs.set(resource, {
				id: lease.id,
				fence,
				expiresAt: lease.expiresAt,
				budget: allowance,
				inFlight: 0,
				released: false,
			});
			return lease;
		});

	const renew = (lease: Lease, ttlMs: number): Promise<Lease> =>
		Serialise(() => {
			const ttl = Positive(ttlMs, "a renewal ttl");
			const now = clock.now();
			const run = Resolve(lease, now);
			const expiresAt = now + ttl;
			if (expiresAt <= run.expiresAt)
				throw RuntimeError.refused(
					`a renewal must extend the lease: ${expiresAt} is not later than ${run.expiresAt}. Release it instead.`,
				);
			run.expiresAt = expiresAt;
			return { ...lease, expiresAt };
		});

	const withLease = async <T>(
		lease: Lease,
		write: () => Promise<T>,
	): Promise<T> => {
		if (typeof write !== "function")
			throw RuntimeError.refused("withLease needs a write to admit");
		const run = await Serialise(() => {
			const now = clock.now();
			const admitted = Resolve(lease, now);
			Charge(admitted, now);
			admitted.inFlight += 1;
			return admitted;
		});
		try {
			return await write();
		} finally {
			await Serialise(() => Settle(lease.resource, run));
		}
	};

	const release = async (lease: Lease): Promise<void> => {
		try {
			await Serialise(() => {
				const resource = lease?.resource;
				if (typeof resource !== "string") return;
				const run = runs.get(resource);
				if (!run) return;
				/**
				 * A superseded lease is not this caller's to end: the resource now
				 * belongs to the fence that took it over, and releasing here would
				 * hand a stranger's authority back.
				 */
				if (run.fence !== lease.fence) return;
				run.released = true;
				if (run.inFlight === 0) runs.delete(resource);
			});
		} catch {
			// Release never throws: cleanup is on the failure path already.
		}
	};

	const admit = async (request: {
		capability: string;
		state: StateClass;
		generation: number;
	}): Promise<{ ok: true } | { ok: false; reason: string }> => {
		if (!request || typeof request !== "object")
			return {
				ok: false,
				reason: "admission-refused: a request names its capability, its state class and its generation",
			};
		if (request.generation !== generation)
			return {
				ok: false,
				reason: `admission-refused: generation ${String(request.generation)} is not bound; this runtime enforces generation ${generation}`,
			};
		const registered = capabilities.get(request.capability);
		if (!registered)
			return {
				ok: false,
				reason: `admission-refused: ${String(request.capability)} is not registered with this runtime`,
			};
		if (registered.state !== request.state)
			return {
				ok: false,
				reason: `admission-refused: ${registered.name} declares ${registered.state}, not ${String(request.state)}`,
			};
		return { ok: true };
	};

	/**
	 * The binding, as a copy.
	 *
	 * Sorted by name and rebuilt on every call, so a caller cannot mutate the
	 * manifest it was admitted under and cannot get a stale one either. Code-unit
	 * order, not `localeCompare`: a manifest must not change shape with the
	 * machine's locale.
	 */
	const reflect = (): Binding => ({
		generation,
		capabilities: [...capabilities.values()]
			.map((registration) => ({
				name: registration.name,
				state: registration.state,
			}))
			.sort((left, right) =>
				left.name < right.name ? -1 : left.name > right.name ? 1 : 0,
			),
	});

	return {
		admit,
		claim,
		now: () => clock.now(),
		reflect,
		release,
		renew,
		withLease,
	};
};

export { ErrorCode, RuntimeError };

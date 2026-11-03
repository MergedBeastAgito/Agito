/**
 * The port `agentic.runtime` promises.
 *
 * The runtime is a policy engine, not a scheduler of capabilities. It is handed
 * one generation and the capabilities registered against it, and everything it
 * answers afterwards is derived from that binding. It imports no capability, so
 * the manifest it reports is the manifest it enforces - the one the composition
 * root gave it - rather than a description of code it can reach.
 *
 * The port states more than data shapes, because two runtimes that agreed on the
 * shapes alone would still disagree about when authority ends:
 *
 * - **Admission is per effect, not per run.** `withLease` re-checks the fence
 *   before every write. A lease that expired while its worker was still running
 *   refuses the next write instead of letting it land late: the worker keeps
 *   existing, and stops having authority.
 * - **A fence is a number, and expiry is a clock.** A monotonically increasing
 *   fence alone does not prove a lease is live - an expired worker can arrive
 *   before any newer token has reached the resource - so every check compares the
 *   presented fence against the record *and* the record's expiry against the
 *   clock. Either one failing is a refusal.
 * - **A held resource is refused, not queued.** A second claim while the first is
 *   live is `lease-held`. A queue would turn a scheduling decision into a promise
 *   the runtime cannot keep, and the caller waiting on it would have no way to
 *   tell waiting from holding.
 * - **A budget belongs to one run.** The deadline and the operation count are the
 *   caller's, supplied when the run claims its resource. Nothing is pooled, so a
 *   retry never inherits a spent count and never resets a consumed allowance.
 * - **Release never throws, and is safe twice.** Cleanup runs on the failure path
 *   by definition, so it cannot be the step that turns a handled failure into an
 *   unhandled one.
 */

/** The four state classes, in the order the manifest names them. */
export const States = [
	"stateless",
	"holds-transport",
	"holds-identity",
	"holds-workspace",
] as const;

/**
 * What a capability retains between runs, and therefore what may be pre-warmed.
 *
 * These are retention hazards, not a partition: one context can hold a transport,
 * an identity and a workspace at once. A capability declares the strongest one it
 * retains, and the runtime refuses a request that names a different one, because
 * a caller that believes a warm context is anonymous when it carries an identity
 * will reuse it for the wrong task.
 */
export type StateClass = (typeof States)[number];

/** One authority over one resource, granted for a bounded time. */
export type Lease = {
	/** This grant's identity. Unique within the process that issued it. */
	id: string;
	/** What it grants: a name the composition root chose, never a capability's. */
	resource: string;
	/**
	 * Monotonic per resource, never reused, never reset by a release.
	 *
	 * A write carrying an older fence is refused. That is what makes a takeover
	 * safe: the new holder's number is higher than the one the late worker holds.
	 */
	fence: number;
	/**
	 * When authority ends, on the runtime's clock.
	 *
	 * The interval is half-open - `[issued, expiresAt)` - so the instant itself is
	 * already outside it. A boundary that is "still live" is the one an off-by-one
	 * test argues about, and the safe answer is the one that refuses.
	 */
	expiresAt: number;
};

/** One run's allowance: a deadline and a count of effects. */
export type Budget = {
	/**
	 * Absolute, on the same clock as `Lease.expiresAt`. `Infinity` is a run with
	 * no time bound, which is a decision the caller makes explicitly rather than a
	 * default the runtime invents.
	 */
	deadline: number;
	/**
	 * How many effects the run may still perform. `0` means none: the run was
	 * admitted to hold the resource and do nothing with it, which is a legitimate
	 * probe and not an error.
	 */
	operations: number;
};

/**
 * The only clock the runtime reads.
 *
 * Supplied rather than reached for, so a test can expire a lease without sleeping
 * and a caller can express a deadline on the same authority the runtime compares
 * it against. The port does not demand monotonicity - it cannot check it - but a
 * clock that goes backwards turns a live lease into a longer one, so a caller
 * with a wall clock that steps should pass a monotonic reading instead.
 */
export type Clock = {
	/** Milliseconds, in whatever epoch the caller and the runtime agreed on. */
	now(): number;
};

/** What the composition root registers: a capability's name and its retention. */
export type Registration = {
	/** The name `admit` will be asked about. */
	name: string;
	/** The strongest state class the capability retains between runs. */
	state: StateClass;
};

/** What the runtime reports about itself: the binding it enforces. */
export type Binding = {
	/** The generation this runtime was bound to at construction. */
	generation: number;
	/** The capabilities it will admit, sorted by name. */
	capabilities: readonly Registration[];
};

export interface Runtime {
	/** Admit a run, or refuse it with the reason. */
	admit(request: {
		capability: string;
		state: StateClass;
		generation: number;
	}): Promise<{ ok: true } | { ok: false; reason: string }>;
	/** Claim a resource, or fail because one is held. */
	claim(resource: string, ttlMs: number): Promise<Lease>;
	/** Extend a lease, only for the holder of the current fence. */
	renew(lease: Lease, ttlMs: number): Promise<Lease>;
	/** Write under a lease; refuse when the fence is stale. */
	withLease<T>(lease: Lease, write: () => Promise<T>): Promise<T>;
	/** Release, always. Never throws. */
	release(lease: Lease): Promise<void>;
}

/** The three failure modes the manifest declares, and no others. */
export const ErrorCode = {
	AdmissionRefused: "admission-refused",
	LeaseHeld: "lease-held",
	BudgetExhausted: "budget-exhausted",
} as const;

export type ErrorCode = (typeof ErrorCode)[keyof typeof ErrorCode];

/**
 * The one error type `claim`, `renew` and `withLease` reject with.
 *
 * `admit` does not throw - its port returns a refusal - so a caller has two
 * shapes to handle and only one of them is an exception. That asymmetry is the
 * card's, not an accident here: admission answers a question, while a claim or a
 * write either happened or did not.
 *
 * A malformed request - an empty resource name, a ttl that is not positive, a
 * write that is not a function - is `admission-refused` rather than a `TypeError`.
 * The manifest has three codes and no fourth for "the caller's argument was
 * nonsense", and of the three, the one that means "this request may not proceed"
 * is the one that keeps a consumer's error handling inside the vocabulary the
 * manifest promises.
 */
export class RuntimeError extends Error {
	readonly code: ErrorCode;

	constructor(
		code: ErrorCode,
		message: string,
		options?: { cause?: unknown },
	) {
		super(message, options);
		this.name = "RuntimeError";
		this.code = code;
	}

	/** The request may not proceed: it is malformed, or its authority ended. */
	static refused(
		message: string,
		options?: { cause?: unknown },
	): RuntimeError {
		return new RuntimeError(ErrorCode.AdmissionRefused, message, options);
	}

	/** The resource is held under a newer fence, or an effect is still in it. */
	static held(message: string, options?: { cause?: unknown }): RuntimeError {
		return new RuntimeError(ErrorCode.LeaseHeld, message, options);
	}

	/** The run's own allowance is spent: its deadline passed or its count ran out. */
	static spent(message: string, options?: { cause?: unknown }): RuntimeError {
		return new RuntimeError(ErrorCode.BudgetExhausted, message, options);
	}
}

/**
 * What the composition root holds: the port, plus the two things a caller needs
 * in order to use it.
 *
 * `Runtime` is the contract as the card states it, unchanged. A budget on `claim`
 * and a clock reading are additions, not alterations: the five verbs keep their
 * signatures and their meaning, and a caller that only ever uses them gets an
 * unbounded run on the runtime's own clock. They live on a second interface so
 * that a consumer implementing `Runtime` - an adapter in a test, say - does not
 * have to know they exist.
 *
 * The budget is on `claim` because a budget belongs to an attempt, and an attempt
 * begins when a resource is taken. A separate `setBudget` verb would leave a
 * window in which a run holds a resource and has no allowance, and a window is
 * where the unbudgeted effect lands.
 */
export interface Policy extends Runtime {
	/**
	 * Claim a resource for one run, with that run's budget.
	 *
	 * The budget starts here and dies with the lease: a later claim is a new run
	 * and takes the numbers it is given. Omitting it means the run is unbounded,
	 * which is the only default that cannot invent authority the caller did not
	 * grant.
	 */
	claim(resource: string, ttlMs: number, budget?: Budget): Promise<Lease>;
	/** The runtime's clock, so a deadline can be expressed on it. */
	now(): number;
	/** The manifest this runtime enforces, and the generation it is bound to. */
	reflect(): Binding;
}

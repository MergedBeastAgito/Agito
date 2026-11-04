/**
 * The seven reducers, as pure functions over `Context`.
 *
 * Nothing here imports `spacetimedb/server`, reads a clock, or throws on a bad
 * argument: a rule answers, and the answer is a result. That is what lets these
 * be tested with a fake context and adopted by a module that supplies the real
 * one.
 *
 * The five invariants, and where each is enforced:
 *
 * 1. **Coherence.** `bind` appends a generation and refuses a manifest naming a
 *    capability the catalog does not declare. Every other reducer refuses to
 *    write under a generation that is not the latest bound one (`Latest`), and a
 *    lease never adopts a later generation than the one it was granted under.
 * 2. **Fence.** `claim` increments the fence; `renew`, `record` and `probe` refuse
 *    a fence that is not the lease's current one, so a worker whose lease expired
 *    cannot write as though it still held it. `record` is where an effect is
 *    checked, and it checks the caller too: `lease.holder` is `ctx.sender`, so a
 *    caller that reproduces every number on the row is still not the holder.
 * 3. **Disclosure.** `probe` writes exactly one audience-scoped row and returns
 *    only that row. The private value is never an argument, so it is never a
 *    column: a reader cannot select what the database was never told.
 * 4. **Time.** Every timestamp a row carries is `ctx.now`. No function reads a
 *    wall clock, and the lags between times are differences the caller computes.
 * 5. **Handover.** An effect that was admitted under an epoch and refused when it
 *    lands is not retracted, because rejecting a result does not undo an effect.
 *    `record` writes the refusal as an `effect-ambiguous` trace row and moves the
 *    lease to `quarantined`; `claim` refuses a quarantined resource; `resolve`
 *    states whether the effect landed or was absent and only then releases the
 *    resource for reassignment.
 */
import type { Context, Row } from "./Context.ts";

/** Why a rule refused. Three reasons, and each has exactly one source. */
export type Reason = "admission-refused" | "lease-held" | "budget-exhausted";

export type Refusal = { readonly ok: false; readonly reason: Reason };

export type Result = { readonly ok: true } | Refusal;

/**
 * `probe`'s answer: the declared result, plus the row it wrote.
 *
 * "Returns only what it wrote" needs the row to be returnable, and the ok branch
 * here is assignable to the plain `{ ok: true }`, so a caller written against the
 * result type alone still type-checks.
 */
export type ProbeResult = { readonly ok: true; readonly observation: Row } | Refusal;

const Ok: Result = { ok: true };
const Refused: Refusal = { ok: false, reason: "admission-refused" };
const Held: Refusal = { ok: false, reason: "lease-held" };
const Exhausted: Refusal = { ok: false, reason: "budget-exhausted" };

/**
 * The latest bound generation, or `null` when `id` is unknown or superseded.
 *
 * A generation is superseded by the existence of its successor, not by a column
 * someone rewrote: ids are dense sequence numbers, so `sequence + 1` being
 * present is the whole test. That is one key lookup, which is the only way to
 * answer "is this the latest?" through a context that finds by key and cannot
 * scan - and it leaves the predecessor byte-identical, which is what "appends a
 * generation" has to mean if the binding history is to be auditable.
 *
 * The `String(sequence) === id` check is what makes the density assumption
 * fail closed: a row whose key disagrees with its own sequence number cannot be
 * extended safely, so it is treated as no generation at all rather than as one.
 */
export const Latest = (ctx: Context, generation: string): Row | null => {
	const row = ctx.find("generation", generation);
	if (row === null) return null;
	const sequence = row["sequence"];
	if (typeof sequence !== "number") return null;
	if (String(sequence) !== generation) return null;
	if (ctx.find("generation", String(sequence + 1)) !== null) return null;
	return row;
};

/**
 * Whether a lease row is live at `now`.
 *
 * Expiry is a comparison, never a stored transition: a row that says `held` but
 * whose deadline has passed is expired, because no timer has to have run for
 * that to be true. This is also why an expired lease cannot be renewed back to
 * life - `Fenced` asks this question before any write.
 */
const IsLive = (lease: Row, now: number): boolean => {
	const expiresAt = lease["expiresAt"];
	return lease["state"] === "held" && typeof expiresAt === "number" && expiresAt > now;
};

/**
 * The lease a fenced write is admitted under, or `null` when it is not.
 *
 * Every condition is a refusal with the same reason on purpose: a caller learns
 * that its write did not land, not which of six things was wrong with it. The
 * fence is checked before expiry because a stale fence is the ordinary case -
 * the worker that woke up late - and the answer must not depend on how late.
 */
const Fenced = (ctx: Context, generation: string, resource: string, fence: number): Row | null => {
	if (Latest(ctx, generation) === null) return null;
	const lease = ctx.find("lease", resource);
	if (lease === null) return null;
	if (lease["holder"] !== ctx.sender) return null;
	if (lease["state"] !== "held") return null;
	if (lease["generation"] !== generation) return null;
	if (lease["fence"] !== fence) return null;
	if (!IsLive(lease, ctx.now)) return null;
	return lease;
};

/** The manifest a composition root binds. */
export type Bind = {
	/** The generation this one supersedes, or `null` for the first. */
	readonly previous: string | null;
	readonly manifest: {
		/** The composition root's name for the manifest. Opaque here: the digest
		 * is what the manifest is called, and recomputing one over an unspecified
		 * canonical form would be a second convention to keep. */
		readonly digest: string;
		/** The capability ids the manifest names. Each must be declared. */
		readonly capabilities: readonly string[];
	};
};

/**
 * Bind a generation: append it, and refuse a manifest that names a capability
 * the catalog does not declare.
 *
 * A bind names its predecessor, and the predecessor must still be the latest -
 * which is what makes two concurrent binds a compare-and-swap rather than a
 * fork. In a serialised database the second one sees the first one's successor
 * and is refused.
 *
 * One insert, no update: the previous generation is untouched, and the catalog
 * marks the table append-only so a context cannot quietly rewrite it.
 */
export const bind = (ctx: Context, args: Bind): Result => {
	if (args.manifest.digest.length === 0) return Refused;
	const named = [...args.manifest.capabilities];
	if (new Set(named).size !== named.length) return Refused;
	for (const id of named) if (ctx.find("capability", id) === null) return Refused;

	let sequence: number;
	if (args.previous === null) {
		// The first generation is always `1`, so "has anything been bound?" is one
		// lookup rather than a search for the maximum.
		if (ctx.find("generation", "1") !== null) return Refused;
		sequence = 1;
	} else {
		const previous = ctx.find("generation", args.previous);
		if (previous === null) return Refused;
		if (Latest(ctx, args.previous) === null) return Refused;
		const parent = previous["sequence"];
		if (typeof parent !== "number") return Refused;
		sequence = parent + 1;
	}

	const id = String(sequence);
	if (ctx.find("generation", id) !== null) return Refused;
	ctx.insert("generation", {
		id,
		sequence,
		parent: args.previous,
		manifestDigest: args.manifest.digest,
		capabilities: JSON.stringify([...named].sort()),
		boundAt: ctx.now,
		boundBy: ctx.sender,
	});
	return Ok;
};

/** A claim on one resource for one attempt. */
export type Claim = {
	/** The resource id: a browser session, a task, a workspace. */
	readonly resource: string;
	/** The generation the claim is made under, which must be the latest. */
	readonly generation: string;
	/** The attempt the lease is for. */
	readonly task: string;
	/** How long the lease lasts, in the module's clock unit. Must be positive. */
	readonly ttl: number;
};

/**
 * Claim a resource, or be told it is held.
 *
 * A live lease refuses the claim - `lease-held`, not a queue, because a queue in
 * the reducer would be a second source of ownership. An expired or released one
 * is claimed by incrementing the fence, which is the only thing that makes the
 * previous holder's writes stop landing: the row is never deleted, so the fence
 * never restarts at one.
 */
export const claim = (ctx: Context, args: Claim): Result => {
	if (Latest(ctx, args.generation) === null) return Refused;
	if (args.resource.length === 0) return Refused;
	if (!(args.ttl > 0)) return Refused;

	const existing = ctx.find("lease", args.resource);
	// A quarantined resource is not reassigned. An effect admitted under its last
	// epoch may have landed, and a new holder would act on a state nobody knows;
	// `resolve` is the only way out, and it is a decision somebody has to make.
	if (existing !== null && existing["state"] === "quarantined") return Held;
	if (existing !== null && IsLive(existing, ctx.now)) return Held;

	const previous = existing === null ? 0 : existing["fence"];
	const fence = typeof previous === "number" ? previous + 1 : 1;
	const row: Row = {
		id: args.resource,
		generation: args.generation,
		holder: ctx.sender,
		task: args.task,
		fence,
		state: "held",
		acquiredAt: ctx.now,
		renewedAt: null,
		expiresAt: ctx.now + args.ttl,
	};
	if (existing === null) ctx.insert("lease", row);
	else ctx.update("lease", row);
	return Ok;
};

/** A renewal of a lease the caller already holds. */
export type Renew = {
	readonly resource: string;
	readonly generation: string;
	/** The fence the lease was granted with. A stale fence is refused. */
	readonly fence: number;
	readonly ttl: number;
};

/**
 * Renew a live lease. A stale fence, an expired lease and another holder's lease
 * are all refused, and none of them writes.
 *
 * Renewal extends; it never resurrects. An expired lease is re-claimed, which
 * increments the fence and therefore invalidates the worker that was late.
 */
export const renew = (ctx: Context, args: Renew): Result => {
	if (!(args.ttl > 0)) return Refused;
	const lease = Fenced(ctx, args.generation, args.resource, args.fence);
	if (lease === null) return Refused;
	ctx.update("lease", {
		...lease,
		renewedAt: ctx.now,
		expiresAt: ctx.now + args.ttl,
	});
	return Ok;
};

/** One effect asking to be recorded at the epoch that admitted it. */
export type Effect = {
	/** The generation the effect was admitted under. */
	readonly generation: string;
	/** The resource whose lease admitted it. */
	readonly resource: string;
	/** The fence the resource was at when it was admitted. */
	readonly fence: number;
	/** The effect's own name for what it did. */
	readonly name: string;
};

/**
 * Write a refusal as an ambiguity and quarantine the resource.
 *
 * The row is a trace rather than a new table because the catalog's six tables are
 * what a module renders, and evidence of a refused write is what `trace` is for:
 * it carries no fence, so the write worth keeping - the one that was refused - is
 * not dropped. The quarantine is the lease row's `state`, so "may this resource be
 * reassigned" stays one key lookup, which is the only kind of question this
 * context can answer.
 */
const Quarantine = (ctx: Context, lease: Row, args: Effect, reason: string): Refusal => {
	ctx.insert("trace", {
		id: `${ctx.now}:effect-ambiguous:${args.resource}#${args.fence}#${args.name}`,
		at: ctx.now,
		kind: "effect-ambiguous",
		subject: args.resource,
		payload: JSON.stringify({ name: args.name, fence: args.fence, reason }),
		generation: args.generation,
		sender: ctx.sender,
	});
	if (lease["state"] !== "quarantined") ctx.update("lease", { ...lease, state: "quarantined" });
	return Refused;
};

/**
 * Record one effect where it lands, or refuse it.
 *
 * This is the guarantee, and `claim` is the optimisation: a claim refuses work
 * whose authority had already ended, but it ran before the effect did anything and
 * cannot say anything about the instant it writes. Only the place the record is
 * presented to can, so the check is here.
 *
 * The checks are asked in the order that makes the answer the true one:
 *
 * 1. **Is this the latest generation?** A record under a superseded one is not
 *    this database's record.
 * 2. **Is there a lease row?** No row means nothing was admitted for this
 *    resource, so nothing can be attributed to it.
 * 3. **Was the lease granted under this generation?** A lease never adopts a later
 *    one.
 * 4. **Is this the current fence?** A superseded epoch is refused, and the claim
 *    that superseded it is the accounting point for the effects it had in flight -
 *    the row no longer describes that epoch, so this rule cannot attribute them.
 * 5. **Is the caller the holder?** `lease.holder === ctx.sender`. This is the check
 *    a caller cannot pass by reproducing numbers: the transport authenticates the
 *    sender, so the sender is never an argument.
 *
 * Only after all five does a refusal become *ambiguous*, because only then is the
 * effect one this database admitted. An effect that was admitted and refused
 * anyway may have changed the world, so the refusal is written as an
 * `effect-ambiguous` trace row and the lease is quarantined: the write is not
 * counted as a success, and `claim` will not hand the resource to anybody else
 * until `resolve` says what happened.
 */
export const record = (ctx: Context, args: Effect): Result => {
	if (args.name.length === 0) return Refused;
	if (Latest(ctx, args.generation) === null) return Refused;
	const lease = ctx.find("lease", args.resource);
	if (lease === null) return Refused;
	if (lease["generation"] !== args.generation) return Refused;
	if (lease["fence"] !== args.fence) return Refused;
	if (lease["holder"] !== ctx.sender) return Refused;

	// From here the record is one this database issued to this caller, so a
	// refusal means an authorised effect may have landed.
	if (lease["state"] === "quarantined")
		return Quarantine(ctx, lease, args, "already quarantined by an earlier effect");
	if (lease["state"] === "released")
		return Quarantine(ctx, lease, args, "released while the effect was in flight");
	if (!IsLive(lease, ctx.now)) return Quarantine(ctx, lease, args, "outlived its authority");

	ctx.insert("trace", {
		id: `${ctx.now}:effect:${args.resource}#${args.fence}#${args.name}`,
		at: ctx.now,
		kind: "effect",
		subject: args.resource,
		payload: JSON.stringify({ name: args.name, fence: args.fence }),
		generation: args.generation,
		sender: ctx.sender,
	});
	return Ok;
};

/** One answer to "what happened to the effect that made this resource ambiguous". */
export type Resolve = {
	/** The quarantined resource. */
	readonly resource: string;
	/** The generation the resolver is answering under. */
	readonly generation: string;
	/** What the resolver found: the external action landed, or it was absent. */
	readonly outcome: "landed" | "absent";
};

/**
 * State what happened to the effects that quarantined a resource, and release it
 * for reassignment.
 *
 * The lease row is moved to `released` rather than deleted, so the fence survives
 * and the next claim advances past it. The ambiguity rows stay and the answer is
 * appended as a third row: a resolution that erased the question would be the
 * retraction the handover rule forbids.
 *
 * The sender is recorded, not checked. Who may resolve is a role, and a role is a
 * deployment's decision; the rule's job is that the answer is attributable.
 */
export const resolve = (ctx: Context, args: Resolve): Result => {
	if (Latest(ctx, args.generation) === null) return Refused;
	const lease = ctx.find("lease", args.resource);
	if (lease === null) return Refused;
	if (lease["state"] !== "quarantined") return Refused;
	ctx.update("lease", { ...lease, state: "released" });
	ctx.insert("trace", {
		id: `${ctx.now}:effect-resolved:${args.resource}`,
		at: ctx.now,
		kind: "effect-resolved",
		subject: args.resource,
		payload: JSON.stringify({ outcome: args.outcome, fence: lease["fence"] }),
		generation: args.generation,
		sender: ctx.sender,
	});
	return Ok;
};

/** One reveal: a property version, at the fence the caller holds. */
export type Probe = {
	readonly generation: string;
	/** The resource whose lease the prober holds. */
	readonly resource: string;
	readonly fence: number;
	readonly property: string;
	/** The property version the disclosure is pinned to. */
	readonly version: number;
	/** What the owner permits this audience to learn. Never the private value. */
	readonly observed: string;
};

/**
 * Reveal one property version to the caller, and write the observation that is
 * the reveal's own receipt.
 *
 * The row id is `<property>:<version>:<audience>`, so the receipt *is* the
 * allowance: a second probe by the same audience at the same version finds the
 * row and is refused as `budget-exhausted`. That is why no second row records a
 * charge, and why two racing probes cannot both spend a one-use allowance - the
 * database serialises them, and the loser finds the winner's row.
 *
 * The audience is `ctx.sender`. It is not an argument: a caller-supplied
 * audience would be a caller-supplied identity, and the recipient of a reveal is
 * never something the caller names.
 */
export const probe = (ctx: Context, args: Probe): ProbeResult => {
	if (args.property.length === 0) return Refused;
	if (!Number.isInteger(args.version) || args.version < 1) return Refused;
	if (Fenced(ctx, args.generation, args.resource, args.fence) === null) return Refused;

	const id = `${args.property}:${args.version}:${ctx.sender}`;
	if (ctx.find("probe", id) !== null) return Exhausted;

	const observation: Row = {
		id,
		property: args.property,
		version: args.version,
		audience: ctx.sender,
		observed: args.observed,
		at: ctx.now,
		generation: args.generation,
	};
	ctx.insert("probe", observation);
	return { ok: true, observation };
};

/** One appended event. */
export type Trace = {
	readonly generation: string;
	readonly kind: string;
	readonly subject: string;
	/** Opaque JSON. Never a private value, and never executable. */
	readonly payload: string;
};

/**
 * Append one event, keyed by its instant, kind and subject.
 *
 * A trace is evidence, not an authoritative effect, so it carries no fence: the
 * write worth keeping is often the one that was refused, and fencing the log
 * would drop exactly that. It does carry the generation, because an event read
 * under the wrong interpreter is a different event.
 *
 * A replay at the same instant appends nothing; a different payload under the
 * same key is refused rather than silently replacing the first, because reusing
 * a content key with different content is how a log starts lying.
 */
export const recordTrace = (ctx: Context, args: Trace): Result => {
	if (Latest(ctx, args.generation) === null) return Refused;
	if (args.kind.length === 0 || args.subject.length === 0) return Refused;
	const id = `${ctx.now}:${args.kind}:${args.subject}`;
	const existing = ctx.find("trace", id);
	if (existing !== null) return existing["payload"] === args.payload ? Ok : Refused;
	ctx.insert("trace", {
		id,
		at: ctx.now,
		kind: args.kind,
		subject: args.subject,
		payload: args.payload,
		generation: args.generation,
		sender: ctx.sender,
	});
	return Ok;
};

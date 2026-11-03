/**
 * `agentic.verify.design` over a transport the composition root supplies.
 *
 * Two verbs, and only the first touches a page:
 *
 * 1. **`measure` asks and bounds.** The selectors go to the transport as they
 *    were given, the answer is matched back by selector, and the deadline is
 *    enforced on this side as well as the provider's - a transport that ignores
 *    its budget must not be able to hang a caller. The one rejection is
 *    `not-ready`: the page was never read. An element that is not in the page is
 *    a measurement, never a rejection.
 * 2. **`check` is pure.** It reads no page and keeps no state, so the same
 *    measurements answer the same verdict twice. It cannot throw either: a
 *    reading that throws, an expectation that throws and a comparator that
 *    throws are each recorded as a row that did not hold, because a page defect
 *    must not be able to end a run as an exception.
 *
 * Nothing here names a selector, a length or a colour. The transport supplies
 * what a page says; the caller supplies what the design expects.
 */
import {
	VerifyError,
	type Evidence,
	type Invariant,
	type Measurement,
	type OperationOptions,
	type Verdict,
	type Verify,
} from "./Port.ts";
import type { Transport } from "./Transport.ts";

/** A missing or nonsensical deadline is no deadline, so it is zero. */
const budgetOf = (timeoutMs: number): number =>
	Number.isFinite(timeoutMs) ? Math.max(0, timeoutMs) : 0;

const reasonOf = (error: unknown): string =>
	error instanceof Error ? error.message : String(error);

/**
 * A value as a detail line.
 *
 * A provider that answered with the wrong shape is reported by what it sent, not
 * by `[object Object]`, because the detail is the only thing that says which
 * part of the contract it broke.
 */
const describe = (value: unknown): string => {
	if (value === null) return "null";
	if (value === undefined) return "nothing";
	if (typeof value === "string") return JSON.stringify(value);
	if (typeof value === "number" || typeof value === "boolean") {
		return String(value);
	}
	if (Array.isArray(value)) return `an array of ${value.length}`;
	return `a ${typeof value}`;
};

/**
 * The longest a timer may be set for.
 *
 * `setTimeout` takes a signed 32-bit delay: anything larger overflows and fires
 * on the next tick, which would turn a caller's generous deadline into an
 * immediate `not-ready`. Clamping keeps a long deadline long.
 */
const MaxTimerMs = 2_147_483_647;

/**
 * Race one measurement against its deadline and the caller's signal.
 *
 * The transport is told its own deadline, so this is a second guarantee rather
 * than the first. Both events are `not-ready`, because from a consumer's side "I
 * gave up" and "it never answered" are the same answer: none. `detail` keeps the
 * two apart for a caller that needs to know which happened.
 */
const bounded = <T>(
	phase: string,
	work: () => Promise<T>,
	timeoutMs: number,
	signal: AbortSignal | undefined,
): Promise<T> =>
	new Promise<T>((resolve, reject) => {
		// A caller that forgot its signal still gets its deadline; it must not get
		// a TypeError out of the guard that exists to protect it.
		if (signal?.aborted) {
			reject(
				VerifyError.notReady(
					`${phase} was cancelled before it started`,
					"the caller's signal had already fired",
				),
			);
			return;
		}
		let settled = false;
		let timer: ReturnType<typeof setTimeout> | undefined;
		const stop = () => {
			if (timer !== undefined) clearTimeout(timer);
			signal?.removeEventListener("abort", abort);
		};
		const settle = (error?: unknown, value?: T) => {
			if (settled) return;
			settled = true;
			stop();
			if (error === undefined) resolve(value as T);
			else reject(error);
		};
		const abort = () =>
			settle(
				VerifyError.notReady(
					`${phase} was cancelled by the caller`,
					"the signal fired while the page was being read",
				),
			);
		timer = setTimeout(
			() =>
				settle(
					VerifyError.notReady(
						`${phase} exceeded its ${timeoutMs}ms deadline`,
						"the transport never answered",
					),
				),
			Math.min(timeoutMs, MaxTimerMs),
		);
		signal?.addEventListener("abort", abort, { once: true });
		let pending: Promise<T>;
		try {
			pending = work();
		} catch (error) {
			settle(error ?? new Error(`${phase} failed without a reason`));
			return;
		}
		void pending.then(
			(value) => settle(undefined, value),
			(error: unknown) =>
				settle(error ?? new Error(`${phase} failed without a reason`)),
		);
	});

/**
 * What a measurement's value may be.
 *
 * A provider that answers an object has not finished serialising what it read,
 * and flattening that to `null` would report "nothing to report" about an
 * element that did report something. The port refuses it instead, naming the
 * selector, so the provider is fixed rather than silently trusted.
 */
const asValue = (value: unknown, selector: string): number | string | null => {
	if (value === null || value === undefined) return null;
	if (typeof value === "number" || typeof value === "string") return value;
	throw VerifyError.notReady(
		"the transport answered with a value the port does not carry",
		`${JSON.stringify(selector)} read ${describe(value)}; a measurement is a number, a string or null`,
	);
};

/** One entry of a transport's answer, as the port defines a measurement. */
const asMeasurement = (entry: unknown): Measurement => {
	if (typeof entry !== "object" || entry === null) {
		throw VerifyError.notReady(
			"the transport answered with a measurement that is not an object",
			`it answered ${describe(entry)}`,
		);
	}
	const { selector, present, value } = entry as {
		selector?: unknown;
		present?: unknown;
		value?: unknown;
	};
	if (typeof selector !== "string" || selector.length === 0) {
		throw VerifyError.notReady(
			"the transport answered with a measurement for no selector",
			`it answered ${describe(entry)}`,
		);
	}
	if (typeof present !== "boolean") {
		throw VerifyError.notReady(
			"the transport answered without saying whether the element was there",
			`${JSON.stringify(selector)} carried ${describe(present)} as present`,
		);
	}
	// An absent element cannot carry a value: a stale number travelling with
	// `present: false` is what an invariant that forgot to check would read.
	return {
		selector,
		present,
		value: present ? asValue(value, selector) : null,
	};
};

/**
 * One measurement per asked selector, in the order it was asked for.
 *
 * The answer is matched by selector rather than by position, so a provider that
 * batches or reorders cannot hand an invariant another element's number. A
 * selector the transport did not answer for is a broken transport, not an absent
 * element: absence is something the page was observed to have, and an answer
 * that never arrived was not observed at all. Reporting it as absent would
 * invent a page defect out of a provider's omission.
 */
const reconcile = (asked: string[], answered: unknown): Measurement[] => {
	if (!Array.isArray(answered)) {
		throw VerifyError.notReady(
			"the transport answered with something that is not a measurement list",
			`it answered ${describe(answered)}`,
		);
	}
	const bySelector = new Map<string, Measurement>();
	for (const entry of answered) {
		const measurement = asMeasurement(entry);
		bySelector.set(measurement.selector, measurement);
	}
	return asked.map((selector) => {
		const found = bySelector.get(selector);
		if (found === undefined) {
			throw VerifyError.notReady(
				"the transport did not answer for a selector it was asked about",
				`nothing was measured for ${JSON.stringify(selector)}`,
			);
		}
		// A copy per position: two entries naming one selector must not be one
		// mutable object, or a caller editing one edits the other.
		return {
			selector,
			present: found.present,
			value: found.value,
		};
	});
};

/** A reading that threw, recorded so it cannot be mistaken for a value. */
type Thrown = { threw: string };

const threw = (error: unknown): Thrown => ({ threw: reasonOf(error) });

const isThrown = (value: unknown): value is Thrown =>
	typeof value === "object" &&
	value !== null &&
	typeof (value as { threw?: unknown }).threw === "string";

/** Whether one invariant held, without letting any part of it throw. */
const judge = (invariant: Invariant, measurements: Measurement[]): Evidence => {
	const label =
		typeof invariant?.label === "string" ? invariant.label : "(unlabelled)";
	let got: unknown;
	let want: unknown;
	try {
		want =
			typeof invariant.want === "function"
				? (invariant.want as (m: Measurement[]) => unknown)(
						measurements,
					)
				: invariant.want;
	} catch (error) {
		want = threw(error);
	}
	try {
		got = invariant.read(measurements);
	} catch (error) {
		got = threw(error);
	}
	let held = false;
	if (!isThrown(got) && !isThrown(want)) {
		try {
			held =
				typeof invariant.test === "function"
					? Boolean(invariant.test(got, want))
					: Object.is(got, want);
		} catch {
			// A comparison that could not be made has not been satisfied. The
			// row still carries what was read, which is what a reader needs.
			held = false;
		}
	}
	return { label, got, want, held };
};

/**
 * Judge a set of measurements.
 *
 * The absent list is built first and independently of the invariants, because a
 * selector that matched nothing fails the run even when every row that reads
 * through it happened to hold: the design named an element, and the page does
 * not have it.
 */
const check = (
	invariants: Invariant[],
	measurements: Measurement[],
): Verdict => {
	const measured = Array.isArray(measurements) ? measurements : [];
	const absent: string[] = [];
	for (const measurement of measured) {
		if (measurement?.present !== false) continue;
		const selector =
			typeof measurement.selector === "string"
				? measurement.selector
				: "";
		if (!absent.includes(selector)) absent.push(selector);
	}
	const evidence: Evidence[] = [];
	for (const invariant of Array.isArray(invariants) ? invariants : []) {
		evidence.push(judge(invariant, measured));
	}
	return {
		held: absent.length === 0 && evidence.every((row) => row.held),
		evidence,
		absent,
	};
};

/**
 * Build a `Verify` over one transport.
 *
 * Nothing here opens a browser or reads an environment variable: the transport
 * is the composition root's to supply, and the fake in `Test/` is the proof that
 * the rules hold without one.
 *
 * A missing transport is reported when it is used, not when the capability is
 * built, because a composition root that assembles in a different order must
 * still be able to register this capability and fail one call rather than
 * refuse to start.
 */
export const Create = (transport: Transport): Verify => ({
	async measure(selectors, options: OperationOptions) {
		const asked = Array.isArray(selectors) ? selectors : [];
		// Nothing asked is nothing read: a no-op request must not touch a page.
		if (asked.length === 0) return [];
		if (
			transport === undefined ||
			transport === null ||
			typeof transport.measure !== "function"
		) {
			throw VerifyError.notReady(
				"this capability has no transport to measure through",
				"the composition root supplied none",
			);
		}
		const budget = budgetOf(options?.timeoutMs);
		const signal = options?.signal;
		let answered: unknown;
		try {
			answered = await bounded(
				"measure",
				() => transport.measure(asked, { timeoutMs: budget, signal }),
				budget,
				signal,
			);
		} catch (error) {
			throw error instanceof VerifyError
				? error
				: VerifyError.notReady(
						"the transport failed while measuring the page",
						reasonOf(error),
						error,
					);
		}
		return reconcile(asked, answered);
	},
	check,
});

/**
 * The port `agentic.verify.design` promises, written the way its consumer asks
 * for it.
 *
 * `module.json` names three errors - `absent-element`, `invariant-broke` and
 * `not-ready` - and this file is where they live. A caller narrows on
 * `VerifyError.code`; anything a provider does that is not one of the three is
 * mapped onto one of them at the boundary, so a consumer never has to guess what
 * a failure meant.
 *
 * The port states more than data shapes, because two implementations that agreed
 * on the shapes alone would still disagree about what a failure is:
 *
 * - **An absent element is a failure, never a precondition.** `check` reports
 *   the selector in `absent` and answers `held: false`. It never throws, and it
 *   never reports "could not run": the run happened, and the page was wrong. A
 *   probe that threw on a deleted node would report the deletion as a check that
 *   could not run.
 * - **This package holds no design.** No selector, no pixel value, no colour. A
 *   caller supplies the selectors and the invariants; the test supplies a
 *   fixture. A port that knew one site's rail width could not measure another
 *   site at all.
 * - **`measure` never throws for a missing element.** A selector that matches
 *   nothing is a measurement with `present: false`, because "there is nothing
 *   there" is an observation. A dead transport is the only throw, and it is
 *   `not-ready`, so the two answers cannot be confused.
 * - **Evidence is per invariant.** One entry per invariant, in the order the
 *   caller gave them, so a failure names which invariant broke and what it read.
 */

/**
 * One element, as the page answered for it.
 *
 * The type exists to keep "not observed" apart from a real zero, `false` or
 * empty string: a measurement engine that cannot say "there is nothing there"
 * produces evidence that reads as a measurement.
 */
export type Measurement = {
	/** The selector this was asked about, as the caller wrote it. */
	selector: string;
	/**
	 * Whether the element was there.
	 *
	 * `false` is an observation, not an error: the page was read, and the
	 * element was not in it.
	 */
	present: boolean;
	/**
	 * What was read from it - a width, a count, a label - or `null`.
	 *
	 * An absent element always carries `null`, so a stale number cannot travel
	 * with `present: false` and be read by an invariant that forgot to check.
	 * A present element that read `null` is a value, not an absence: the element
	 * was there and had nothing to report.
	 */
	value: number | string | null;
};

/** One thing the design says must be true of the page. */
export type Invariant = {
	/** What the invariant is about, as a reader of a failure should hear it. */
	label: string;
	/**
	 * What to read out of the measurements.
	 *
	 * A reader that throws - because it dereferenced a measurement it assumed
	 * was there - is a failed invariant, not a blocked run, and `check` records
	 * the throw rather than propagating it.
	 */
	read: (measurements: Measurement[]) => unknown;
	/**
	 * What the reading must equal.
	 *
	 * Either a value, compared with `Object.is`, or a function of the
	 * measurements when the expectation is derived from the page itself - "the
	 * rail sits level with the drawer" wants the drawer's own top, and no
	 * constant states it. A structure that must compare equal is serialised in
	 * `read` (`join`, `JSON.stringify`), because `Object.is` is identity and two
	 * equal arrays are two objects.
	 */
	want: unknown;
	/**
	 * How to compare, for an invariant that is a relation rather than an
	 * equality.
	 *
	 * "The lane air exceeds the canvas gap" compares two readings and no single
	 * `want` value states it, so the comparison itself is the caller's. Absent
	 * means `Object.is`; a comparator that throws counts as not held, because a
	 * comparison that could not be made has not been satisfied.
	 */
	test?: (got: unknown, want: unknown) => boolean;
};

/** One invariant's row in a verdict: what it read, what it wanted, and whether. */
export type Evidence = {
	/** The invariant's own label, copied unchanged. */
	label: string;
	/**
	 * What the invariant read.
	 *
	 * A `read` that threw is recorded as `{ threw: <message> }`, which cannot be
	 * mistaken for a value it read.
	 */
	got: unknown;
	/** What it was compared against, or `{ threw: <message> }` for the same. */
	want: unknown;
	/** Whether the comparison held. */
	held: boolean;
};

/**
 * The whole answer to one run.
 *
 * `held` is true only when nothing was absent and every invariant held, so a
 * caller can branch on one field and still read the reason from the others.
 */
export type Verdict = {
	/** Every invariant held and every selector matched something. */
	held: boolean;
	/** One row per invariant, in the order the caller gave them. */
	evidence: Evidence[];
	/** Every selector that was observed to match nothing, once each. */
	absent: string[];
};

/** What every measurement is bounded by: a deadline and the caller's signal. */
export type OperationOptions = {
	/** Wall-clock budget for the whole measurement. */
	timeoutMs: number;
	/** The caller's cancellation. Firing it is `not-ready`, not a page defect. */
	signal: AbortSignal;
};

/** The one port this capability offers. */
export interface Verify {
	/**
	 * Read the named elements from the page, one measurement per selector.
	 *
	 * Never throws because an element is missing - that is `present: false`. The
	 * only rejection is `not-ready`, which means the page was never read.
	 */
	measure(
		selectors: string[],
		options: OperationOptions,
	): Promise<Measurement[]>;
	/**
	 * Judge a set of measurements against a set of invariants. Pure, and it
	 * never throws: a verdict is a result, not an exception.
	 */
	check(invariants: Invariant[], measurements: Measurement[]): Verdict;
}

/** The three failure modes the manifest declares, and no others. */
export const ErrorCode = {
	Absent: "absent-element",
	Broke: "invariant-broke",
	NotReady: "not-ready",
} as const;

export type ErrorCode = (typeof ErrorCode)[keyof typeof ErrorCode];

/** The one error type this capability throws. */
export class VerifyError extends Error {
	readonly code: ErrorCode;
	/** What was observed, kept apart from the sentence shown to a human. */
	readonly detail: string;

	constructor(
		code: ErrorCode,
		message: string,
		detail = "",
		options?: { cause?: unknown },
	) {
		super(message, options);
		this.name = "VerifyError";
		this.code = code;
		this.detail = detail;
	}

	/**
	 * The page was never read: the transport is dead, it broke mid-measurement,
	 * the deadline passed, or the caller gave up.
	 *
	 * This is the only code `measure` rejects with, because the manifest has no
	 * fourth code and "the page was never read" is what all four mean to this
	 * capability. `detail` says which one happened, so a caller can tell its own
	 * cancellation from a slow page.
	 */
	static notReady(
		message: string,
		detail = "",
		cause?: unknown,
	): VerifyError {
		return new VerifyError(ErrorCode.NotReady, message, detail, { cause });
	}

	/**
	 * A verdict as the exception the manifest names, or `null` when it held.
	 *
	 * `check` never throws, because a verdict is a result. A caller that must
	 * fail loudly - a command that exits non-zero - asks for the exception here.
	 * Absence is reported before a broken invariant, because a page that lost a
	 * part is the more actionable failure and the rows that read through it are
	 * consequences of it.
	 */
	static from(verdict: Verdict): VerifyError | null {
		const absent = Array.isArray(verdict?.absent) ? verdict.absent : [];
		if (absent.length > 0) {
			return new VerifyError(
				ErrorCode.Absent,
				`${absent.length} element(s) the design names are absent`,
				absent.join(", "),
			);
		}
		const evidence = Array.isArray(verdict?.evidence)
			? verdict.evidence
			: [];
		const broken = evidence.filter((row) => row?.held !== true);
		if (broken.length > 0) {
			return new VerifyError(
				ErrorCode.Broke,
				`${broken.length} invariant(s) broke`,
				broken.map((row) => String(row?.label ?? "")).join(", "),
			);
		}
		return null;
	}
}

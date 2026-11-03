/**
 * The seam a provider implements, and the fake in `Test/` stands in for.
 *
 * Nothing in this file opens a browser or knows one. A transport may speak CDP,
 * read a saved document, or answer from a fixture, and `Create` applies the
 * port's rules to whatever it returns.
 *
 * The contract on a provider:
 *
 * - one `Measurement` per selector it was asked about, matched by `selector`
 *   rather than by position, so a provider that batches or reorders cannot hand
 *   an invariant another element's number;
 * - a selector that matches nothing is `present: false` - an observation, not a
 *   rejection. A provider that throws because an element is missing turns a page
 *   defect into a failure of the observer;
 * - a value is a number, a string or `null`. Anything else is a provider that
 *   has not finished serialising what it read, and it is refused rather than
 *   flattened, because flattening it to `null` would read as "nothing to
 *   report";
 * - a rejection means the page was never read, and it is reported as
 *   `not-ready`: a dead socket, a navigation failure and a page that threw are
 *   one answer to this capability, with the cause riding along;
 * - `timeoutMs` is honoured. The capability enforces the same deadline on its
 *   own side, so a provider that ignores it cannot hang a caller - but a
 *   provider that leans on that reports its own failures as somebody else's.
 */
import type { Measurement, OperationOptions } from "./Port.ts";

export interface Transport {
	/** Read every named element, or report that it is not in the page. */
	measure(selectors: string[], options: OperationOptions): Promise<Measurement[]>;
}

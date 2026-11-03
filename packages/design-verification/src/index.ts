/**
 * @agentic/design-verification - Measure a built page and produce evidence;
 * expectations stay with the consumer.
 *
 * The port this package promises is declared in ../module.json. Nothing here may
 * import another package's adapter; the composition root supplies one.
 */
export { Create } from "./Verification.ts";
export { ErrorCode, VerifyError } from "./Port.ts";
export type {
	Evidence,
	Invariant,
	Measurement,
	OperationOptions,
	Verdict,
	Verify,
} from "./Port.ts";
export type { Transport } from "./Transport.ts";

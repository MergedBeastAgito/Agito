/**
 * @agentic/browser-cdp - Browser execution substrate: attach versus launch, bounded evaluation, disposal.
 *
 * The port this package promises is declared in ../module.json. Nothing here may
 * import another package's adapter; the composition root supplies one.
 */
export { BrowseError, ErrorCode } from "./Port.ts";
export type { Browse, OpenOptions, OperationOptions, Ownership, Target } from "./Port.ts";
export { createSession } from "./Session.ts";
export type { SessionOptions } from "./Session.ts";
export type { Launcher, Transport } from "./Transport.ts";

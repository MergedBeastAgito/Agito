/**
 * The seam a provider implements, and the fake in `Test/` stands in for.
 *
 * Nothing in this file starts a browser. A `Launcher` is supplied by the
 * composition root - it may speak CDP over a WebSocket, drive a subprocess, or
 * answer from a fixture - and `Session` applies the port's rules to whatever it
 * returns.
 *
 * The contract on a provider:
 *
 * - a transport reports its own death through `closed`, so the session can tell
 *   a dead socket from a page that is merely not ready;
 * - a rejected `attach`/`launch` carries a `BrowseError` when it knows why
 *   (`navigation-failed` for a page that did not load); anything else is read
 *   as an unreachable browser;
 * - `close` and `detach` are idempotent: the session may call them on a target
 *   it has already released.
 */
import type { OperationOptions } from "./Port.ts";

/** One live page connection. */
export interface Transport {
	/** The endpoint this transport is bound to; it becomes `Target.ws`. */
	readonly endpoint: string;
	/** True once the connection is gone. A closed transport is dead, not slow. */
	readonly closed: boolean;
	/** Ask the page one expression and return what it produced. */
	evaluate<T>(expression: string, options: OperationOptions): Promise<T>;
	/** Close the page this session launched. */
	close(): Promise<void>;
	/** Let go of a page somebody else owns, without closing it. */
	detach(): Promise<void>;
}

/** Where a transport comes from: an existing browser, or a new one. */
export interface Launcher {
	/** Attach to a browser already listening at `endpoint`, bound to `url`. */
	attach(
		endpoint: string,
		url: string,
		options: OperationOptions,
	): Promise<Transport>;
	/** Start a browser and open `url` in it. */
	launch(url: string, options: OperationOptions): Promise<Transport>;
}

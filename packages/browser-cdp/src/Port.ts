/**
 * The port `agentic.browse` promises, written the way a consumer asks for it.
 *
 * `module.json` names three errors - `browser-unavailable`, `navigation-failed`
 * and `timeout` - and this file is where they live. A caller narrows on
 * `BrowseError.code`; a provider that fails in some fourth way is mapped onto
 * one of the three at the boundary, so a consumer never has to guess.
 */

/** Who owns the page behind a target, and therefore who may close it. */
export type Ownership = "attached" | "launched";

/** The handle a session is bound to. */
export type Target = {
	/** The page this session is bound to. */
	url: string;
	/** The endpoint the transport behind that page is connected to. */
	ws: string;
	/**
	 * `launched` means this session started the browser and may close it.
	 * `attached` means somebody else owns it: disposal only disconnects.
	 */
	ownership: Ownership;
};

/** What every operation is bounded by: a deadline and the caller's signal. */
export type OperationOptions = {
	/** Wall-clock budget for the whole operation. */
	timeoutMs: number;
	/** The caller's cancellation. Firing it is a `timeout`, not a failure. */
	signal: AbortSignal;
};

/** `open` also decides whether to attach to a browser that already exists. */
export type OpenOptions = OperationOptions & {
	/** Attach to a browser already listening here instead of launching one. */
	attachTo?: string;
};

export interface Browse {
	/** Open a page, returning the handle the session is bound to. */
	open(url: string, options: OpenOptions): Promise<Target>;
	/** Evaluate an expression in the page and return its value. */
	evaluate<T>(target: Target, expression: string, options: OperationOptions): Promise<T>;
	/** Release everything this session holds. Never throws. */
	dispose(target: Target): Promise<void>;
}

/** The three failure modes the manifest declares, and no others. */
export const ErrorCode = {
	Unavailable: "browser-unavailable",
	Navigation: "navigation-failed",
	Timeout: "timeout",
} as const;

export type ErrorCode = (typeof ErrorCode)[keyof typeof ErrorCode];

/** The one error type a `Browse` call rejects with. */
export class BrowseError extends Error {
	readonly code: ErrorCode;
	/** What was observed, kept apart from the sentence shown to a human. */
	readonly detail: string;

	constructor(code: ErrorCode, message: string, detail = "", options?: { cause?: unknown }) {
		super(message, options);
		this.name = "BrowseError";
		this.code = code;
		this.detail = detail;
	}

	/** The transport is gone: a dead socket, not a slow page. */
	static unavailable(message: string, detail = "", cause?: unknown): BrowseError {
		return new BrowseError(ErrorCode.Unavailable, message, detail, {
			cause,
		});
	}

	/** The page exists but could not be used: it failed to load, or threw. */
	static navigation(message: string, detail = "", cause?: unknown): BrowseError {
		return new BrowseError(ErrorCode.Navigation, message, detail, {
			cause,
		});
	}

	/** The signal fired or the deadline passed before an answer arrived. */
	static timeout(message: string, detail = "", cause?: unknown): BrowseError {
		return new BrowseError(ErrorCode.Timeout, message, detail, { cause });
	}
}

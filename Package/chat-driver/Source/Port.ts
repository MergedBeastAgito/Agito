/**
 * The port `agentic.chat.ask` promises, written the way its consumer asks for it.
 *
 * `module.json` names four errors - `not-ready`, `wrong-mode`, `wrong-model` and
 * `truncated` - and this file is where they live. A caller narrows on
 * `ChatError.code`; anything a page does that is not one of those four is mapped
 * onto one of them at the boundary, so a consumer never has to guess what a
 * failure meant.
 *
 * The port states more than data shapes, because two drivers that agreed on the
 * shapes alone would still behave differently:
 *
 * - **A question is answered or it is not.** `ask` returns an answer whose text
 *   the page had stopped changing, or it rejects. A partial answer is never
 *   returned as a whole one, and an empty string is never an answer.
 * - **Mode and model are preconditions.** The composer is in search mode and set
 *   to the requested model before anything is typed, so a wrong answer is
 *   refused rather than produced.
 * - **The answer is the one this question caused.** A thread already holds the
 *   answers to earlier questions, so the turn is correlated by counting them
 *   before the send and reading only the one that appeared after it.
 * - **A collapsed answer is refused.** When the page hides the rest of an answer
 *   behind a control, the text in the DOM is not the answer, and returning it
 *   would be a quiet lie.
 * - **Cancellation is `not-ready`.** A signal that fires and a deadline that
 *   passes are the same event to a consumer - the answer did not arrive - so both
 *   are `not-ready`. `detail` says which one happened, for a caller that needs to
 *   tell its own cancellation from a slow page.
 */

/** One step of the work an answer reports having done. */
export type Step = {
	/** Nesting: a step inside a step is one level deeper than it. */
	depth: number;
	/** What the step is called, as the page labels it. */
	label: string;
	/** The page's own note about it - a duration, a state - when it has one. */
	detail: string;
};

/** One cited source behind an answer. */
export type Source = {
	/** Where the citation points. */
	url: string;
	/** How the page labels the citation. */
	label: string;
};

/** What a caller asks for. */
export type Question = {
	/** The question, sent as written. */
	text: string;
	/**
	 * The composer mode the question must be asked in.
	 *
	 * The port is defined for search only. An agent mode answers by running tools
	 * on the caller's behalf, which is a different capability with different costs
	 * and a different budget, so a driver that silently used one would be spending
	 * somebody else's allowance.
	 */
	mode: "search";
	/** The model the answer must come from, matched against the page's label. */
	model: string;
};

/** What a settled turn produced. */
export type Answer = {
	/** The answer's text, read once the page stopped changing it. */
	text: string;
	/** The work the answer reports having done, in the order it did it. */
	trace: Step[];
	/** The sources the answer cites. */
	sources: Source[];
	/** The model label the page showed when the question was sent. */
	model: string;
	/** The composer mode the page was in when the question was sent. */
	mode: string;
};

/**
 * The handle a page is bound to.
 *
 * The driver passes it back to the `Page` that issued it and never reads it, so a
 * provider may hang whatever it needs there. `url` is the one field it keeps,
 * because a failure report that cannot say which page failed is not a report.
 */
export type Target = { readonly url: string };

/** What every page operation is bounded by: a deadline and the caller's signal. */
export type OperationOptions = {
	/** Wall-clock budget for the whole operation. */
	timeoutMs: number;
	/** The caller's cancellation. Firing it is `not-ready`, not a page failure. */
	signal: AbortSignal;
};

/**
 * The browser as this capability needs it: one page, asked one expression at a
 * time.
 *
 * This is not `Browse`, and it is deliberately not a copy of it. It has the same
 * three verbs because a page is opened, asked and released, and it stops there:
 * no ownership, no attach, no transport, no launcher. The composition root adapts
 * `Browse` to this, and that is where the difference between "a browser this
 * session launched" and "a tab somebody else owns" is settled - in assembly, not
 * here.
 *
 * What the port demands of an implementation:
 *
 * - `evaluate` returns the value of the expression, not a wrapper around it, and
 *   the expression is a complete JavaScript expression whose value is
 *   JSON-serialisable. `document` is the only host object it may touch, so a page
 *   that is a tab and a page that is a fixture answer the same driver.
 * - `timeoutMs` is honoured: a page that never answers is rejected, not awaited.
 *   The driver enforces the same deadline on its side, so an implementation that
 *   ignores this cannot hang a caller - but a provider that leans on that is a
 *   provider that reports its own failures as somebody else's.
 * - `dispose` never throws. A release that fails is a fact about the browser, and
 *   a caller that already has its answer must not lose it to a failing cleanup.
 */
export interface Page {
	/** Open the page at `url`. */
	open(url: string, options: OperationOptions): Promise<Target>;
	/** Evaluate one expression in the page and return its value. */
	evaluate<T>(target: Target, expression: string, options: OperationOptions): Promise<T>;
	/** Release everything the page holds. Never throws. */
	dispose(target: Target): Promise<void>;
}

/** The one port this capability offers. */
export interface Chat {
	/** Ask one question and return the answer once the page has settled. */
	ask(question: Question, options: OperationOptions): Promise<Answer>;
}

/** The four failure modes the manifest declares, and no others. */
export const ErrorCode = {
	NotReady: "not-ready",
	WrongMode: "wrong-mode",
	WrongModel: "wrong-model",
	Truncated: "truncated",
} as const;

export type ErrorCode = (typeof ErrorCode)[keyof typeof ErrorCode];

/** The one error type an `ask` call rejects with. */
export class ChatError extends Error {
	readonly code: ErrorCode;
	/** What was observed, kept apart from the sentence shown to a human. */
	readonly detail: string;

	constructor(code: ErrorCode, message: string, detail = "", options?: { cause?: unknown }) {
		super(message, options);
		this.name = "ChatError";
		this.code = code;
		this.detail = detail;
	}

	/**
	 * The page is not in a state where it can answer: it did not open, it failed
	 * while being read, it never produced an answer, the answer never stopped
	 * changing, or the caller gave up.
	 *
	 * This is the one code that covers a provider's failures as well, because the
	 * manifest has no fifth code and "the page could not answer" is what a dead
	 * transport, a navigation failure and a slow page all mean to this capability.
	 * The cause is attached rather than flattened, so the difference survives.
	 */
	static notReady(message: string, detail = "", cause?: unknown): ChatError {
		return new ChatError(ErrorCode.NotReady, message, detail, { cause });
	}

	/** The composer would not enter search mode, so nothing was sent. */
	static wrongMode(message: string, detail = "", cause?: unknown): ChatError {
		return new ChatError(ErrorCode.WrongMode, message, detail, { cause });
	}

	/** The page is set to a different model, so nothing was sent. */
	static wrongModel(message: string, detail = "", cause?: unknown): ChatError {
		return new ChatError(ErrorCode.WrongModel, message, detail, { cause });
	}

	/** The answer is collapsed, so the text in the DOM is only part of it. */
	static truncated(message: string, detail = "", cause?: unknown): ChatError {
		return new ChatError(ErrorCode.Truncated, message, detail, { cause });
	}
}

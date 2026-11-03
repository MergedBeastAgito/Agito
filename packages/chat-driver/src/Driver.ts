/**
 * `agentic.chat.ask` over a page the composition root supplies.
 *
 * The four rules of the port, applied in the order that makes each of them mean
 * something:
 *
 * 1. **The mode is a precondition.** The composer's toggle is read before
 *    anything is typed. When it does not read `search` it is pressed once and read
 *    again; when it still does not, the call fails with `wrong-mode` and nothing
 *    was sent. One press, not a loop: a toggle that does not move on the first
 *    press is a page that will not move on the fifth, and a retry loop would spend
 *    the caller's deadline proving it.
 * 2. **The model is a precondition.** The label is compared before anything is
 *    typed, so a page set to another model is refused rather than asked.
 * 3. **Completion is a rule, not a sleep.** The answer's text is polled until
 *    three consecutive reads report no change. A sleep would return whatever the
 *    network happened to have delivered; a count of reads would return an answer
 *    that had stopped at a paragraph break. A deadline hit is `not-ready`, never a
 *    partial answer presented as a whole one.
 * 4. **A collapsed answer is `truncated`.** The control that hides the rest of an
 *    answer is looked for inside the newest answer once it has settled, because
 *    the text in the DOM is then demonstrably not all of it.
 *
 * Everything the page does that is not one of those four is `not-ready`: a page
 * that will not open, a page that fails mid-read, a composer that is not there, a
 * send control that never enables. The manifest has four codes and the driver
 * does not invent a fifth; the cause rides along on the error instead.
 */
import {
	PressMode,
	PressSubmit,
	ReadAnswers,
	ReadCollapse,
	ReadMode,
	ReadModel,
	ReadSources,
	ReadTrace,
	TypeQuestion,
	type Answers,
	type SendOutcome,
} from "./Expressions.ts";
import {
	ChatError,
	type Answer,
	type Chat,
	type OperationOptions,
	type Page,
	type Question,
	type Source,
	type Step,
	type Target,
} from "./Port.ts";
import { DefaultSelectors, type Selectors } from "./Selectors.ts";

/** How long between two reads of an answer while it is being written. */
const DefaultPollMs = 250;

/** A release that has not finished in this long is reported, not waited on. */
const DefaultDisposeBudgetMs = 5_000;

/**
 * How many reads must report no change before an answer counts as settled.
 *
 * Three, and the counter counts *comparisons*: the third read that equals the one
 * before it is what settles the answer, so a page that delivers its last token
 * and stops is read four times in all. Fewer than three is a page that paused
 * mid-sentence; more than three is latency nobody asked for.
 */
const StableReads = 3;

export type DriverOptions = {
	/** The page to open: the chat product's URL. */
	url: string;
	/** How the page is read. Defaults to `DefaultSelectors`. */
	selectors?: Selectors;
	/** How long between two reads of the answer while it is being written. */
	pollMs?: number;
	/** How long a release may take before it is reported as a failure. */
	disposeTimeoutMs?: number;
	/** Where a dispose failure goes, because it may not be thrown. */
	report?: (failure: { target: Target; error: unknown }) => void;
};

/** A missing or nonsensical deadline is no deadline, so it is zero. */
const budgetOf = (timeoutMs: number): number =>
	Number.isFinite(timeoutMs) ? Math.max(0, timeoutMs) : 0;

const reasonOf = (error: unknown): string =>
	error instanceof Error ? error.message : String(error);

/**
 * A string the page offered, or its rendering when it offered something else.
 *
 * Every value the page returns crosses the port boundary as `unknown`: the
 * expressions are the driver's own, but the provider is not, and a provider that
 * answers a number where a label was asked for must not be able to make `ask`
 * throw a `TypeError` out of a port that declares four failures. So a value is
 * read as text when it is text and rendered as text when it is not - the rules
 * then refuse it on its merits, with a code the manifest actually has.
 */
const textOf = (value: unknown): string =>
	typeof value === "string" ? value : value === null || value === undefined ? "" : String(value);

/**
 * What a label means, for comparison.
 *
 * Case and runs of whitespace are not part of a model's name - a page that shows
 * `gpt-6.1  sol` is showing the model the caller asked for - so both sides are
 * folded before they are compared. Nothing else is folded: a different model is a
 * different model.
 */
const normalise = (value: unknown): string =>
	textOf(value).replace(/\s+/g, " ").trim().toLowerCase();

/** What was read, for a failure's detail, without pretending `null` is a value. */
const describe = (value: unknown): string => {
	if (value === null || value === undefined) return "nothing";
	if (typeof value === "string")
		return value.trim().length === 0 ? "nothing" : JSON.stringify(value);
	try {
		return JSON.stringify(value) ?? String(value);
	} catch {
		return String(value);
	}
};

/**
 * One read of the thread, or `null` when the page answered with something that is
 * not one.
 *
 * A thread with no answers is `{ count: 0, text: "" }`, so "there is no answer
 * yet" and "the page did not answer the question I asked" are different values.
 * Folding them together would let a provider's wrong shape look like an empty
 * thread and cost the caller its whole deadline before `not-ready`.
 */
const asAnswers = (value: unknown): Answers | null => {
	if (typeof value !== "object" || value === null) return null;
	const { count, text } = value as { count?: unknown; text?: unknown };
	if (typeof count !== "number" || !Number.isFinite(count)) return null;
	return { count, text: textOf(text) };
};

/** The steps the page reported, or `null` when it reported something else. */
const asSteps = (value: unknown): Step[] | null => {
	if (!Array.isArray(value)) return null;
	return value.map((step: unknown) => {
		const { depth, label, detail } =
			typeof step === "object" && step !== null
				? (step as {
						depth?: unknown;
						label?: unknown;
						detail?: unknown;
					})
				: {};
		return {
			depth: typeof depth === "number" && Number.isFinite(depth) ? depth : 0,
			label: textOf(label),
			detail: textOf(detail),
		};
	});
};

/** The sources the page cited, or `null` when it cited something else. */
const asSources = (value: unknown): Source[] | null => {
	if (!Array.isArray(value)) return null;
	return value.map((source: unknown) => {
		const { url, label } =
			typeof source === "object" && source !== null
				? (source as { url?: unknown; label?: unknown })
				: {};
		return { url: textOf(url), label: textOf(label) };
	});
};

/**
 * The longest a timer may be set for.
 *
 * `setTimeout` takes a signed 32-bit delay: anything larger overflows and fires on
 * the next tick, which would turn a caller's generous deadline into an immediate
 * `not-ready`. Clamping keeps a long deadline long.
 */
const MaxTimerMs = 2_147_483_647;

/**
 * Race one page operation against its deadline and the caller's signal.
 *
 * The page is told its own deadline, so this is a second guarantee rather than
 * the first: a page that ignores its budget must not be able to hang a caller, and
 * a page that hangs is a page that is not ready. Both events are `not-ready`,
 * because from a consumer's side "I gave up" and "it never answered" are the same
 * answer: none.
 */
const bounded = <T>(
	phase: string,
	work: () => Promise<T>,
	timeoutMs: number,
	signal: AbortSignal | undefined,
): Promise<T> =>
	new Promise<T>((resolve, reject) => {
		// A caller that forgot its signal still gets its deadline; it must not get a
		// TypeError out of the guard that exists to protect it.
		if (signal?.aborted) {
			reject(
				ChatError.notReady(
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
				ChatError.notReady(
					`${phase} was cancelled by the caller`,
					"the signal fired while the page was being asked",
				),
			);
		timer = setTimeout(
			() =>
				settle(
					ChatError.notReady(
						`${phase} exceeded its ${timeoutMs}ms deadline`,
						"the page never answered",
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
			(error: unknown) => settle(error ?? new Error(`${phase} failed without a reason`)),
		);
	});

/** Wait, or less than that when the caller gives up first. */
const pause = (ms: number, signal: AbortSignal | undefined): Promise<void> =>
	new Promise<void>((resolve) => {
		if (signal?.aborted) {
			resolve();
			return;
		}
		const done = () => {
			clearTimeout(timer);
			signal?.removeEventListener("abort", done);
			resolve();
		};
		const timer = setTimeout(done, Math.min(ms, MaxTimerMs));
		signal?.addEventListener("abort", done, { once: true });
	});

/**
 * Build a `Chat` over one `Page`.
 *
 * Nothing here opens a browser, knows a transport, or reads an environment
 * variable: the page is the composition root's to supply, and the fake in `Test/`
 * is the proof that the rules hold without one.
 */
export const Create = (page: Page, options: DriverOptions): Chat => {
	const selectors = options.selectors ?? DefaultSelectors;
	const pollMs = budgetOf(options.pollMs ?? DefaultPollMs);
	const disposeBudget = budgetOf(options.disposeTimeoutMs ?? DefaultDisposeBudgetMs);
	const report =
		options.report ??
		((failure: { target: Target; error: unknown }) => {
			console.warn(`chat-driver: dispose of ${failure.target.url} failed`, failure.error);
		});

	const reportQuietly = (failure: { target: Target; error: unknown }) => {
		try {
			report(failure);
		} catch {
			// A reporter that throws must not turn a release into a throw.
		}
	};

	const release = async (target: Target): Promise<void> => {
		try {
			await bounded("dispose", () => page.dispose(target), disposeBudget, undefined);
		} catch (error) {
			// Never thrown: the caller already has its answer, or its own failure,
			// and a failing cleanup must not replace either.
			reportQuietly({ target, error });
		}
	};

	const perform = async (question: Question, call: OperationOptions): Promise<Answer> => {
		const budget = budgetOf(call.timeoutMs);
		const deadline = Date.now() + budget;
		const signal = call.signal;

		const remaining = (): number => deadline - Date.now();

		/** The budget one more page operation may have, or why it may not have one. */
		const left = (phase: string): number => {
			if (signal?.aborted)
				throw ChatError.notReady(
					`${phase} was cancelled before it started`,
					"the caller's signal had already fired",
				);
			const ms = remaining();
			if (ms <= 0)
				throw ChatError.notReady(
					`${phase} exceeded the ${budget}ms deadline`,
					"the page was still working when the budget ran out",
				);
			return ms;
		};

		/** One expression, run inside whatever budget is left. */
		const probe = async <T>(phase: string, expression: string, target: Target): Promise<T> => {
			const timeoutMs = left(phase);
			try {
				return await bounded(
					phase,
					() =>
						page.evaluate<T>(target, expression, {
							timeoutMs,
							signal,
						}),
					timeoutMs,
					signal,
				);
			} catch (error) {
				if (error instanceof ChatError) throw error;
				throw ChatError.notReady(
					`${phase} could not be read from the page`,
					reasonOf(error),
					error,
				);
			}
		};

		// The port is defined for search mode, so a question that asks for another
		// one is refused here, before a page is opened: there is nothing to verify
		// against a toggle the driver is not willing to use.
		if (question.mode !== "search")
			throw ChatError.wrongMode(
				"this port only asks in search mode",
				`the question asked for ${JSON.stringify(question.mode)}`,
			);

		let target: Target;
		try {
			const timeoutMs = left("open");
			target = await bounded(
				"open",
				() => page.open(options.url, { timeoutMs, signal }),
				timeoutMs,
				signal,
			);
		} catch (error) {
			throw error instanceof ChatError
				? error
				: ChatError.notReady("the chat page could not be opened", reasonOf(error), error);
		}

		try {
			/** One read of the thread, refusing a page that answers something else. */
			const readAnswers = async (phase: string): Promise<Answers> => {
				const value = await probe<unknown>(phase, ReadAnswers(selectors), target);
				const answers = asAnswers(value);
				if (answers === null)
					throw ChatError.notReady(
						`${phase} could not be read from the page`,
						`the page answered ${describe(value)} instead of a thread`,
					);
				return answers;
			};

			// The mode is a precondition, not a preference.
			let mode = await probe<unknown>("mode", ReadMode(selectors), target);
			if (normalise(mode) !== "search") {
				await probe<boolean>("mode press", PressMode(selectors), target);
				mode = await probe<unknown>("mode", ReadMode(selectors), target);
				if (normalise(mode) !== "search")
					throw ChatError.wrongMode(
						"the composer does not offer search mode",
						`after pressing the toggle it reads ${describe(mode)}`,
					);
			}

			// The model is a precondition, and it is compared before the send.
			const label = await probe<unknown>("model", ReadModel(selectors), target);
			if (normalise(label) !== normalise(question.model))
				throw ChatError.wrongModel(
					"the page is set to a different model",
					`wanted ${JSON.stringify(question.model)}, the page reads ${describe(label)}`,
				);

			// Correlate the turn: what is already in the thread is not this answer.
			const before = await readAnswers("answers");

			const typed = await probe<boolean>(
				"composer",
				TypeQuestion(selectors, question.text),
				target,
			);
			if (typed !== true)
				throw ChatError.notReady(
					"the composer could not be found",
					`nothing matched ${JSON.stringify(selectors.composer)}`,
				);

			const sent = await probe<SendOutcome>("submit", PressSubmit(selectors), target);
			if (sent !== "sent")
				throw ChatError.notReady(
					"the question could not be sent",
					sent === "disabled"
						? "the send control is disabled after the question was typed"
						: `nothing matched ${JSON.stringify(selectors.submit)}`,
				);

			// Completion is stable text: unchanged for three consecutive reads.
			let previous: string | null = null;
			let steady = 0;
			let text = "";
			for (;;) {
				// The deadline is checked here, not only inside the read, so a page that
				// keeps writing is reported as an answer that never settled rather than
				// as one slow read.
				if (remaining() <= 0)
					throw ChatError.notReady(
						`the answer did not settle within the ${budget}ms deadline`,
						`the last read was ${describe(previous)}`,
					);
				const read = await readAnswers("answer");
				const current = read.count > before.count ? read.text : "";
				if (current.length > 0) {
					if (current === previous) steady += 1;
					else {
						previous = current;
						steady = 0;
					}
					if (steady >= StableReads) {
						text = current;
						break;
					}
				} else {
					previous = null;
					steady = 0;
				}
				await pause(Math.min(pollMs, Math.max(0, remaining())), signal);
			}

			// A collapsed answer is not the whole answer.
			const collapsed = textOf(
				await probe<unknown>("collapse", ReadCollapse(selectors), target),
			).trim();
			if (collapsed.length > 0)
				throw ChatError.truncated(
					"the answer is collapsed behind a control",
					`the page shows ${JSON.stringify(collapsed)} over ${text.length} characters`,
				);

			const traceValue = await probe<unknown>("trace", ReadTrace(selectors), target);
			const trace = asSteps(traceValue);
			if (trace === null)
				throw ChatError.notReady(
					"the trace could not be read from the page",
					`the page answered ${describe(traceValue)} instead of a list of steps`,
				);
			const sourcesValue = await probe<unknown>("sources", ReadSources(selectors), target);
			const sources = asSources(sourcesValue);
			if (sources === null)
				throw ChatError.notReady(
					"the sources could not be read from the page",
					`the page answered ${describe(sourcesValue)} instead of a list of sources`,
				);

			return {
				text,
				trace,
				sources,
				model: textOf(label).trim() || textOf(question.model).trim(),
				mode: "search",
			};
		} finally {
			await release(target);
		}
	};

	/**
	 * Ask, and guarantee the port's contract on the way out.
	 *
	 * `perform` is written to fail with one of the manifest's four codes, but a
	 * caller is not a type checker and a provider is not this file: a missing
	 * question, a missing options object, or a value that breaks an assumption
	 * inside `perform` would otherwise leave the port as a `TypeError`. A
	 * consumer that narrows on `ChatError.code` would then have no branch to
	 * take, which is the one thing a four-code port must not do. Anything that is
	 * not already a `ChatError` becomes `not-ready` - the answer did not arrive -
	 * with the cause attached rather than flattened.
	 */
	const ask = async (question: Question, call: OperationOptions): Promise<Answer> => {
		try {
			return await perform(question, call);
		} catch (error) {
			if (error instanceof ChatError) throw error;
			throw ChatError.notReady("the question could not be asked", reasonOf(error), error);
		}
	};

	return { ask };
};

/**
 * `Browse` over a transport the composition root supplies.
 *
 * Every classification rule lives here, in one place:
 *
 * - silence is a timeout. Nothing but the deadline can end a page that never
 *   answers, so a hang is reported as `timeout` and never as a dead transport;
 * - a closed transport is `browser-unavailable`, whatever error arrived with
 *   it, because the socket is gone rather than the page being slow;
 * - a live transport that could not be used is `navigation-failed`.
 *
 * Cancellation belongs to the caller: every operation races the caller's signal
 * and its own deadline, and both lose to `timeout` rather than to a bare
 * `Error`, so a consumer can distinguish "I gave up" from "the browser broke".
 */
import {
	BrowseError,
	type Browse,
	type OpenOptions,
	type OperationOptions,
	type Ownership,
	type Target,
} from "./Port.ts";
import type { Launcher, Transport } from "./Transport.ts";

/** A release that has not finished in this long is reported, not waited on. */
const DefaultDisposeBudgetMs = 5_000;

export type SessionOptions = {
	/** Where a dispose failure goes, because it may not be thrown. */
	report?: (failure: { target: Target; error: unknown }) => void;
	/** How long a release may take before it is reported as a failure. */
	disposeTimeoutMs?: number;
};

/** A missing or nonsensical deadline is no deadline, so it is zero. */
const budgetOf = (timeoutMs: number): number =>
	Number.isFinite(timeoutMs) ? Math.max(0, timeoutMs) : 0;

const reasonOf = (error: unknown): string =>
	error instanceof Error ? error.message : String(error);

/** Race one operation against the caller's signal and its deadline. */
const bounded = <T>(
	phase: string,
	work: () => Promise<T>,
	options: OperationOptions,
): Promise<T> => {
	const budget = budgetOf(options.timeoutMs);
	// A caller that forgot its signal still gets its deadline; it must not get a
	// TypeError out of the guard that exists to protect it.
	const signal: AbortSignal | undefined = options.signal;
	if (signal?.aborted)
		return Promise.reject(
			BrowseError.timeout(
				`${phase} was cancelled before it started`,
				"the signal was already aborted",
			),
		);
	return new Promise<T>((resolve, reject) => {
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
				BrowseError.timeout(
					`${phase} was cancelled by the caller`,
					"the signal fired while the page was being asked",
				),
			);
		timer = setTimeout(
			() =>
				settle(
					BrowseError.timeout(
						`${phase} exceeded its ${budget}ms deadline`,
						"the page never answered",
					),
				),
			budget,
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
};

/** Race a release against a fixed budget, so `dispose` cannot hang. */
const withBudget = (work: () => Promise<void>, budget: number): Promise<void> =>
	new Promise<void>((resolve, reject) => {
		const timer = setTimeout(
			() =>
				reject(
					BrowseError.timeout(
						`dispose exceeded its ${budget}ms budget`,
						"the release did not finish",
					),
				),
			budget,
		);
		const done = (error?: unknown) => {
			clearTimeout(timer);
			if (error === undefined) resolve();
			else reject(error);
		};
		let pending: Promise<void>;
		try {
			pending = work();
		} catch (error) {
			done(error ?? new Error("dispose failed without a reason"));
			return;
		}
		void pending.then(
			() => done(),
			(error: unknown) => done(error ?? new Error("dispose failed")),
		);
	});

/** A session is one `Browse` bound to one launcher. */
export const createSession = (launcher: Launcher, options: SessionOptions = {}): Browse => {
	const report =
		options.report ??
		((failure: { target: Target; error: unknown }) => {
			console.warn(`browser-cdp: dispose of ${failure.target.url} failed`, failure.error);
		});
	const disposeBudget = budgetOf(options.disposeTimeoutMs ?? DefaultDisposeBudgetMs);
	const held = new Map<string, Transport>();
	const released = new Set<string>();

	const release = async (transport: Transport, ownership: Ownership): Promise<void> => {
		if (ownership === "launched") await transport.close();
		else await transport.detach();
	};

	const reportQuietly = (failure: { target: Target; error: unknown }) => {
		try {
			report(failure);
		} catch {
			// A reporter that throws must not turn dispose into a throw.
		}
	};

	const classifyOpen = (error: unknown): BrowseError =>
		error instanceof BrowseError
			? error
			: BrowseError.unavailable("the browser could not be reached", reasonOf(error), error);

	const classifyPage = (error: unknown, transport: Transport): BrowseError => {
		if (error instanceof BrowseError) return error;
		return transport.closed
			? BrowseError.unavailable(
					"the transport closed while the page was being asked",
					reasonOf(error),
					error,
				)
			: BrowseError.navigation("the page could not answer", reasonOf(error), error);
	};

	const open = async (url: string, openOptions: OpenOptions): Promise<Target> => {
		const attachTo = openOptions.attachTo;
		const ownership: Ownership = attachTo === undefined ? "launched" : "attached";
		let transport: Transport;
		try {
			transport = await bounded(
				ownership === "launched" ? "open (launch)" : "open (attach)",
				() =>
					ownership === "launched"
						? launcher.launch(url, openOptions)
						: launcher.attach(attachTo as string, url, openOptions),
				openOptions,
			);
		} catch (error) {
			throw classifyOpen(error);
		}
		if (transport.closed) {
			try {
				await withBudget(() => release(transport, ownership), disposeBudget);
			} catch (error) {
				reportQuietly({
					target: {
						url,
						ws: transport.endpoint,
						ownership,
					},
					error,
				});
			}
			throw BrowseError.unavailable(
				"the transport was already closed when it arrived",
				`${url} on ${transport.endpoint}`,
			);
		}
		const target: Target = {
			url,
			ws: transport.endpoint,
			ownership,
		};
		held.set(target.ws, transport);
		released.delete(target.ws);
		return target;
	};

	const evaluate = async <T>(
		target: Target,
		expression: string,
		operationOptions: OperationOptions,
	): Promise<T> => {
		const transport = held.get(target.ws);
		if (transport === undefined)
			throw BrowseError.unavailable(
				"this session holds no transport for the target",
				`${target.ws} was never opened here, or it was already disposed`,
			);
		if (transport.closed)
			throw BrowseError.unavailable(
				"the transport is closed",
				`${target.ws} is no longer connected`,
			);
		try {
			return await bounded(
				"evaluate",
				() => transport.evaluate<T>(expression, operationOptions),
				operationOptions,
			);
		} catch (error) {
			throw classifyPage(error, transport);
		}
	};

	const dispose = async (target: Target): Promise<void> => {
		try {
			if (released.has(target.ws)) return;
			released.add(target.ws);
			const transport = held.get(target.ws);
			held.delete(target.ws);
			if (transport === undefined) return;
			await withBudget(() => release(transport, target.ownership), disposeBudget);
		} catch (error) {
			// Never throw: not on a failed release, and not on something that is
			// not a target at all. A dispose failure is reported, not propagated.
			reportQuietly({ target, error });
		}
	};

	return { open, evaluate, dispose };
};

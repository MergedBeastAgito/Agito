/**
 * The TypeScript side of the differential run for `agentic.browse`.
 *
 * A scenario is a fixture (one launcher, one transport, one session) and a list
 * of steps. The fake below is the only thing the session can reach, and it
 * records every interaction in `effects`, in order: that list is the protocol
 * the implementation spoke, and it is what makes an implementation that returns
 * the right value by the wrong route fail.
 *
 * The fake is deliberately the same shape as `Test/Browse.test.ts`'s, because a
 * scenario that could only be expressed against a different fake would be a
 * scenario the real suite does not cover.
 */
import { BrowseError } from "../../packages/browser-cdp/src/Port.ts";
import { createSession } from "../../packages/browser-cdp/src/Session.ts";

/** The error a launcher throws, as the scenario describes it. */
const failureOf = (failure) => {
	if (failure === null || failure === undefined) return new Error("the browser failed");
	// `unavailable` is thrown as a bare error on purpose: the TypeScript port
	// classifies an unclassified failure as `browser-unavailable`, and a scenario
	// that handed it a `BrowseError` would skip that rule. The Rust port has no
	// fourth code to classify, so it returns `Unavailable` directly - the same
	// observable, reached the only way each language allows.
	if (failure.code === "navigation")
		return BrowseError.navigation(failure.message, failure.detail);
	if (failure.code === "timeout") return BrowseError.timeout(failure.message, failure.detail);
	return new Error(failure.message);
};

/** A transport the scenario controls, recording what was done to it. */
const transportOf = (fixture, effects) => {
	const state = { ...fixture.transport };
	const transport = {
		endpoint: fixture.endpoint,
		get closed() {
			return state.closed;
		},
		set closed(value) {
			state.closed = value;
		},
		get behaviour() {
			return state.behaviour;
		},
		closes: 0,
		detaches: 0,
		asked: [],
		async evaluate(expression) {
			effects.push(`evaluate:${expression}`);
			transport.asked.push(expression);
			if (state.behaviour === "dead" || state.closed) {
				// The socket died under the call: the fake reports the state as well
				// as the error, so the session can classify it as a dead transport.
				state.closed = true;
				effects.push(`refused:evaluate:${fixture.endpoint}`);
				throw new Error("WebSocket is already in CLOSING or CLOSED state");
			}
			if (state.behaviour === "throw") throw new Error("ReferenceError: nothing is defined");
			if (state.behaviour === "hang") return new Promise(() => {});
			return state.answer;
		},
		async close() {
			effects.push(`close:${fixture.endpoint}`);
			transport.closes += 1;
			if (state.hangOnClose) return new Promise(() => {});
			state.closed = true;
			if (state.closeFailure !== null && state.closeFailure !== undefined) {
				effects.push(`close-failed:${fixture.endpoint}`);
				throw new Error(state.closeFailure);
			}
		},
		async detach() {
			effects.push(`detach:${fixture.endpoint}`);
			transport.detaches += 1;
			state.closed = true;
		},
		patch: (values) => Object.assign(state, values),
	};
	return transport;
};

/** A launcher the scenario controls. */
const launcherOf = (fixture, transport, effects) => {
	const state = { ...fixture.launcher };
	const answer = async () => {
		if (state.open === "hang") return new Promise(() => {});
		if (state.open === "throw") throw failureOf(state.failure);
		if (state.deadOnArrival) transport.closed = true;
		effects.push(`handed:${fixture.endpoint}`);
		return transport;
	};
	return {
		launches: 0,
		attaches: 0,
		async launch(url) {
			effects.push(`launch:${url}`);
			this.launches += 1;
			return await answer();
		},
		async attach(endpoint) {
			effects.push(`attach:${endpoint}`);
			this.attaches += 1;
			return await answer();
		},
		patch: (values) => Object.assign(state, values),
	};
};

/** The caller's signal, as the step describes it. */
const signalOf = (kind) => {
	if (kind === "absent") return undefined;
	const controller = new AbortController();
	if (kind === "aborted") controller.abort();
	if (typeof kind === "string" && kind.startsWith("after:"))
		setTimeout(() => controller.abort(), Number(kind.slice("after:".length)));
	return controller.signal;
};

/** One step, run against the session. */
const step = async (one, held, session, options) => {
	if (one.before !== undefined) {
		if (one.before.transport !== undefined) options.transport.patch(one.before.transport);
		if (one.before.launcher !== undefined) options.launcher.patch(one.before.launcher);
		if (one.before.session !== undefined) Object.assign(options.session, one.before.session);
	}
	const target =
		one.target === undefined || one.target === "last"
			? held.last
			: typeof one.target === "object"
				? one.target
				: undefined;
	if (one.op === "open") {
		const opened = await session.open(
			one.url,
			one.attachTo === undefined || one.attachTo === null
				? { timeoutMs: one.timeoutMs, signal: signalOf(one.signal ?? "fresh") }
				: {
						timeoutMs: one.timeoutMs,
						signal: signalOf(one.signal ?? "fresh"),
						attachTo: one.attachTo,
					},
		);
		held.last = opened;
		return { kind: "target", ownership: opened.ownership, url: opened.url, ws: opened.ws };
	}
	if (one.op === "evaluate") {
		const value = await session.evaluate(
			target,
			one.expression,
			one.signal === "absent"
				? { timeoutMs: one.timeoutMs }
				: { timeoutMs: one.timeoutMs, signal: signalOf(one.signal ?? "fresh") },
		);
		return { kind: "value", value };
	}
	if (one.op === "dispose") {
		for (let index = 0; index < (one.times ?? 1); index += 1) await session.dispose(target);
		return { kind: "void" };
	}
	throw new Error(`unknown browse step ${JSON.stringify(one.op)}`);
};

/** Run one browse case and answer what it did. */
export const runBrowse = async (entry) => {
	const effects = [];
	const fixture = entry.fixture;
	const transport = transportOf(fixture, effects);
	const launcher = launcherOf(fixture, transport, effects);
	const options = { ...fixture.session };
	let reports = 0;
	const session = createSession(launcher, {
		disposeTimeoutMs: options.disposeTimeoutMs,
		report: (failure) => {
			effects.push(`report:${failure.target?.ws ?? "nothing"}`);
			reports += 1;
			if (options.report === "throw") throw new Error("the reporter is broken too");
		},
	});
	const held = { last: undefined };

	let result;
	for (const [index, one] of entry.steps.entries()) {
		try {
			result = await step(one, held, session, { transport, launcher, session: options });
		} catch (error) {
			if (!(error instanceof BrowseError))
				throw new Error(
					`${entry.id} step ${index} rejected with something that is not a BrowseError: ${String(error)}`,
				);
			result = {
				kind: "error",
				code: error.code,
				message: error.message,
				detail: error.detail,
			};
			break;
		}
	}
	return { result, effects, reports };
};

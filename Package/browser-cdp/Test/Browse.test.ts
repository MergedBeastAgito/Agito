/**
 * The contract of `agentic.browse`, proved against a fake.
 *
 * No Chromium, no network, no dependency: `node --test Test/*.test.ts` in this
 * package. The fake distinguishes a dead transport from a slow one by state and
 * by silence - `closed` is a fact the session can read, while a hang is simply a
 * promise that never settles - so the two cases cannot be confused by wording.
 */
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { describe, test } from "node:test";

import { BrowseError, ErrorCode, type OperationOptions, type Target } from "../Source/Port.ts";
import { createSession } from "../Source/Session.ts";
import type { Launcher, Transport } from "../Source/Transport.ts";

/** What the fake page does when it is asked. */
type Behaviour = "answer" | "hang" | "throw" | "dead";

class FakeTransport implements Transport {
	readonly endpoint: string;
	closed = false;
	behaviour: Behaviour;
	answer: unknown;
	closes = 0;
	detaches = 0;
	asked: string[] = [];
	closeFailure: Error | undefined;
	hangOnClose = false;

	constructor(endpoint: string, behaviour: Behaviour = "answer", answer: unknown = undefined) {
		this.endpoint = endpoint;
		this.behaviour = behaviour;
		this.answer = answer;
	}

	async evaluate<T>(expression: string, _options: OperationOptions): Promise<T> {
		this.asked.push(expression);
		if (this.behaviour === "dead" || this.closed) {
			// The socket died under the call: the fake reports the state as well
			// as the error, so the session can classify it as a dead transport.
			this.closed = true;
			throw new Error("WebSocket is already in CLOSING or CLOSED state");
		}
		if (this.behaviour === "throw") throw new Error("ReferenceError: nothing is defined");
		if (this.behaviour === "hang") return new Promise<T>(() => {});
		return this.answer as T;
	}

	async close(): Promise<void> {
		this.closes += 1;
		if (this.hangOnClose) return new Promise<void>(() => {});
		this.closed = true;
		if (this.closeFailure !== undefined) throw this.closeFailure;
	}

	async detach(): Promise<void> {
		this.detaches += 1;
		this.closed = true;
	}
}

class FakeLauncher implements Launcher {
	readonly transport: FakeTransport;
	launches = 0;
	attaches = 0;
	attachedTo: string[] = [];
	launchedUrls: string[] = [];
	open: "answer" | "hang" | "throw" = "answer";
	failure: Error = new Error("connect ECONNREFUSED 127.0.0.1:9222");
	deadOnArrival = false;

	constructor(transport: FakeTransport) {
		this.transport = transport;
	}

	async attach(endpoint: string, _url: string, _options: OperationOptions): Promise<Transport> {
		this.attaches += 1;
		this.attachedTo.push(endpoint);
		return await this.answer();
	}

	async launch(url: string, _options: OperationOptions): Promise<Transport> {
		this.launches += 1;
		this.launchedUrls.push(url);
		return await this.answer();
	}

	private async answer(): Promise<Transport> {
		if (this.open === "hang") return new Promise<Transport>(() => {});
		if (this.open === "throw") throw this.failure;
		if (this.deadOnArrival) this.transport.closed = true;
		return this.transport;
	}
}

const Page = "ws://127.0.0.1:9222/devtools/page/1";
const Url = "https://example.test/";

const session = (launcher: Launcher, disposeTimeoutMs = 40) => {
	const reported: { target: Target; error: unknown }[] = [];
	const browse = createSession(launcher, {
		disposeTimeoutMs,
		report: (failure) => reported.push(failure),
	});
	return { browse, reported };
};

const options = (timeoutMs = 40, signal = new AbortController().signal): OperationOptions => ({
	timeoutMs,
	signal,
});

/** Await a rejection and insist it is the port's error, never a string. */
const rejection = async (work: Promise<unknown>): Promise<BrowseError> => {
	try {
		await work;
	} catch (error) {
		assert.ok(error instanceof BrowseError, `expected a BrowseError, got ${String(error)}`);
		return error;
	}
	assert.fail("expected the operation to reject");
};

describe("agentic.browse", () => {
	test("the port declares exactly the errors the manifest promises", () => {
		const manifest = JSON.parse(
			readFileSync(new URL("../module.json", import.meta.url), "utf8"),
		) as { provides: { errors: string[] } };
		assert.deepEqual(Object.values(ErrorCode).sort(), [...manifest.provides.errors].sort());
	});

	describe("open", () => {
		test("launches a browser when the caller names no endpoint", async () => {
			const launcher = new FakeLauncher(new FakeTransport(Page));
			const { browse } = session(launcher);
			const target = await browse.open(Url, options());
			assert.equal(target.ownership, "launched");
			assert.equal(target.url, Url);
			assert.equal(target.ws, Page);
			assert.equal(launcher.launches, 1);
			assert.equal(launcher.attaches, 0);
			assert.deepEqual(launcher.launchedUrls, [Url]);
		});

		test("attaches to a browser the caller already owns", async () => {
			const launcher = new FakeLauncher(new FakeTransport(Page));
			const { browse } = session(launcher);
			const target = await browse.open(Url, {
				...options(),
				attachTo: "http://127.0.0.1:9222",
			});
			assert.equal(target.ownership, "attached");
			assert.equal(target.ws, Page);
			assert.equal(launcher.attaches, 1);
			assert.equal(launcher.launches, 0);
			assert.deepEqual(launcher.attachedTo, ["http://127.0.0.1:9222"]);
		});

		test("rejects with navigation-failed when the page does not load", async () => {
			const launcher = new FakeLauncher(new FakeTransport(Page));
			launcher.open = "throw";
			launcher.failure = BrowseError.navigation(
				"the page did not load",
				"net::ERR_NAME_NOT_RESOLVED",
			);
			const { browse } = session(launcher);
			const error = await rejection(browse.open(Url, options()));
			assert.equal(error.code, ErrorCode.Navigation);
			assert.equal(error.name, "BrowseError");
		});

		test("rejects with browser-unavailable when the browser cannot be reached", async () => {
			const launcher = new FakeLauncher(new FakeTransport(Page));
			launcher.open = "throw";
			const { browse } = session(launcher);
			const error = await rejection(browse.open(Url, options()));
			assert.equal(error.code, ErrorCode.Unavailable);
			assert.notEqual(error.code, ErrorCode.Timeout);
		});

		test("rejects with timeout when the launcher never answers", async () => {
			const launcher = new FakeLauncher(new FakeTransport(Page));
			launcher.open = "hang";
			const { browse } = session(launcher);
			const error = await rejection(browse.open(Url, options(40)));
			assert.equal(error.code, ErrorCode.Timeout);
			assert.match(error.message, /deadline/);
		});

		test("rejects with timeout when the caller's signal fires", async () => {
			const launcher = new FakeLauncher(new FakeTransport(Page));
			launcher.open = "hang";
			const { browse } = session(launcher);
			const controller = new AbortController();
			setTimeout(() => controller.abort(), 5);
			const error = await rejection(browse.open(Url, options(5_000, controller.signal)));
			assert.equal(error.code, ErrorCode.Timeout);
			assert.match(error.message, /cancelled/);
		});

		test("releases a transport that arrives already dead", async () => {
			const launcher = new FakeLauncher(new FakeTransport(Page));
			launcher.deadOnArrival = true;
			const { browse } = session(launcher);
			const error = await rejection(browse.open(Url, options()));
			assert.equal(error.code, ErrorCode.Unavailable);
			assert.equal(launcher.transport.closes, 1);
		});
	});

	describe("evaluate", () => {
		test("returns the value the page produced", async () => {
			const transport = new FakeTransport(Page, "answer", 42);
			const launcher = new FakeLauncher(transport);
			const { browse } = session(launcher);
			const target = await browse.open(Url, options());
			const value = await browse.evaluate<number>(target, "1 + 41", options());
			assert.equal(value, 42);
			assert.deepEqual(transport.asked, ["1 + 41"]);
		});

		test("rejects with timeout when the page never answers", async () => {
			const transport = new FakeTransport(Page, "hang");
			const launcher = new FakeLauncher(transport);
			const { browse } = session(launcher);
			const target = await browse.open(Url, options());
			const error = await rejection(browse.evaluate(target, "document.title", options(40)));
			assert.equal(error.code, ErrorCode.Timeout);
			assert.equal(typeof error.message, "string");
			assert.match(error.message, /deadline/);
			// The card's headline case: a BrowseError, not a string, not a bare Error.
			assert.equal(typeof error, "object");
			assert.notEqual(error.constructor, Error);
			assert.equal(error.name, "BrowseError");
			// The page was asked and stayed silent; the transport is still alive.
			assert.equal(transport.asked.length, 1);
			assert.equal(transport.closed, false);
		});

		test("rejects with timeout when the signal fires mid-flight", async () => {
			const transport = new FakeTransport(Page, "hang");
			const launcher = new FakeLauncher(transport);
			const { browse } = session(launcher);
			const target = await browse.open(Url, options());
			const controller = new AbortController();
			setTimeout(() => controller.abort(), 5);
			const error = await rejection(
				browse.evaluate(target, "document.title", options(5_000, controller.signal)),
			);
			assert.equal(error.code, ErrorCode.Timeout);
			assert.match(error.message, /cancelled/);
		});

		test("rejects with timeout before asking when the signal is already aborted", async () => {
			const transport = new FakeTransport(Page);
			const launcher = new FakeLauncher(transport);
			const { browse } = session(launcher);
			const target = await browse.open(Url, options());
			const controller = new AbortController();
			controller.abort();
			const error = await rejection(
				browse.evaluate(target, "document.title", options(5_000, controller.signal)),
			);
			assert.equal(error.code, ErrorCode.Timeout);
			assert.deepEqual(transport.asked, []);
		});

		test("still honours the deadline when the caller passes no signal", async () => {
			const transport = new FakeTransport(Page, "hang");
			const launcher = new FakeLauncher(transport);
			const { browse } = session(launcher);
			const target = await browse.open(Url, options());
			// A caller that forgot its signal gets its deadline, not a TypeError.
			const withoutSignal = { timeoutMs: 20 } as OperationOptions;
			const error = await rejection(browse.evaluate(target, "document.title", withoutSignal));
			assert.equal(error.code, ErrorCode.Timeout);
			assert.match(error.message, /deadline/);
		});

		test("rejects with browser-unavailable when the transport closed during the call", async () => {
			const transport = new FakeTransport(Page, "dead");
			const launcher = new FakeLauncher(transport);
			const { browse } = session(launcher);
			const target = await browse.open(Url, options());
			const error = await rejection(browse.evaluate(target, "document.title", options()));
			assert.equal(error.code, ErrorCode.Unavailable);
			assert.notEqual(error.code, ErrorCode.Timeout);
			assert.equal(transport.asked.length, 1);
		});

		test("never asks a transport it already knows is closed", async () => {
			const transport = new FakeTransport(Page);
			const launcher = new FakeLauncher(transport);
			const { browse } = session(launcher);
			const target = await browse.open(Url, options());
			transport.closed = true;
			const error = await rejection(browse.evaluate(target, "document.title", options()));
			assert.equal(error.code, ErrorCode.Unavailable);
			assert.deepEqual(transport.asked, []);
		});

		test("rejects with navigation-failed when the page throws", async () => {
			const transport = new FakeTransport(Page, "throw");
			const launcher = new FakeLauncher(transport);
			const { browse } = session(launcher);
			const target = await browse.open(Url, options());
			const error = await rejection(browse.evaluate(target, "nope.nope", options()));
			assert.equal(error.code, ErrorCode.Navigation);
			assert.equal(transport.closed, false);
		});

		test("rejects with browser-unavailable on a target this session does not hold", async () => {
			const launcher = new FakeLauncher(new FakeTransport(Page));
			const { browse } = session(launcher);
			const error = await rejection(
				browse.evaluate(
					{
						url: Url,
						ws: "ws://127.0.0.1:9222/devtools/page/9",
						ownership: "attached",
					},
					"document.title",
					options(),
				),
			);
			assert.equal(error.code, ErrorCode.Unavailable);
			assert.equal(launcher.transport.asked.length, 0);
		});
	});

	describe("dispose", () => {
		test("closes a launched target", async () => {
			const transport = new FakeTransport(Page);
			const launcher = new FakeLauncher(transport);
			const { browse } = session(launcher);
			const target = await browse.open(Url, options());
			await browse.dispose(target);
			assert.equal(transport.closes, 1);
			assert.equal(transport.detaches, 0);
		});

		test("detaches an attached target without closing it", async () => {
			const transport = new FakeTransport(Page);
			const launcher = new FakeLauncher(transport);
			const { browse } = session(launcher);
			const target = await browse.open(Url, {
				...options(),
				attachTo: "http://127.0.0.1:9222",
			});
			await browse.dispose(target);
			assert.equal(transport.detaches, 1);
			assert.equal(transport.closes, 0);
		});

		test("is silent when called twice", async () => {
			const transport = new FakeTransport(Page);
			const launcher = new FakeLauncher(transport);
			const { browse, reported } = session(launcher);
			const target = await browse.open(Url, options());
			await browse.dispose(target);
			await browse.dispose(target);
			assert.equal(transport.closes, 1);
			assert.deepEqual(reported, []);
		});

		test("reports a failing release instead of throwing it", async () => {
			const transport = new FakeTransport(Page);
			const launcher = new FakeLauncher(transport);
			const { browse, reported } = session(launcher);
			const target = await browse.open(Url, options());
			const failure = new Error("the browser refused to close");
			transport.closeFailure = failure;
			await browse.dispose(target);
			assert.equal(reported.length, 1);
			assert.equal(reported[0].error, failure);
			assert.equal(reported[0].target.ws, Page);
		});

		test("does not throw when its own reporter throws", async () => {
			const transport = new FakeTransport(Page);
			const launcher = new FakeLauncher(transport);
			const browse = createSession(launcher, {
				disposeTimeoutMs: 40,
				report: () => {
					throw new Error("the reporter is broken too");
				},
			});
			const target = await browse.open(Url, options());
			transport.closeFailure = new Error("the browser refused to close");
			await browse.dispose(target);
			assert.equal(transport.closes, 1);
		});

		test("reports a release that hangs instead of waiting forever", async () => {
			const transport = new FakeTransport(Page);
			transport.hangOnClose = true;
			const launcher = new FakeLauncher(transport);
			const { browse, reported } = session(launcher, 20);
			const target = await browse.open(Url, options());
			await browse.dispose(target);
			assert.equal(reported.length, 1);
			const error = reported[0].error;
			assert.ok(error instanceof BrowseError);
			assert.equal(error.code, ErrorCode.Timeout);
		});

		test("resolves for a target this session never opened", async () => {
			const launcher = new FakeLauncher(new FakeTransport(Page));
			const { browse, reported } = session(launcher);
			await browse.dispose({
				url: Url,
				ws: "ws://127.0.0.1:9222/devtools/page/9",
				ownership: "launched",
			});
			assert.deepEqual(reported, []);
		});

		test("resolves even when handed something that is not a target", async () => {
			const launcher = new FakeLauncher(new FakeTransport(Page));
			const { browse, reported } = session(launcher);
			await browse.dispose(undefined as unknown as Target);
			assert.equal(reported.length, 1);
		});

		test("leaves the target unusable afterwards", async () => {
			const transport = new FakeTransport(Page);
			const launcher = new FakeLauncher(transport);
			const { browse } = session(launcher);
			const target = await browse.open(Url, options());
			await browse.dispose(target);
			const error = await rejection(browse.evaluate(target, "document.title", options()));
			assert.equal(error.code, ErrorCode.Unavailable);
			assert.deepEqual(transport.asked, []);
		});
	});
});

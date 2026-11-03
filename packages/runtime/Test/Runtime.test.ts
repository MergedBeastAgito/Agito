/**
 * The contract test for `agentic.runtime`, proved against a supplied clock.
 *
 * No browser, no network, no dependency, no sleeping: the clock is a number this
 * file moves, so "the lease expired while its worker was still running" is a
 * deterministic event rather than a race against the machine.
 *
 * The fake a write lands in is a list, and the list is the evidence. Every
 * refusal below also asserts that the list did not grow, because a refusal that
 * still lands the effect is the one failure this package exists to prevent - a
 * refused call that wrote anyway is worse than an accepted one, since the caller
 * believes nothing happened.
 *
 *   node --test Test/*.test.ts
 */
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { describe, test } from "node:test";

import {
	Create,
	ErrorCode,
	RuntimeError,
	type Budget,
	type Clock,
	type Lease,
	type Policy,
	type StateClass,
} from "../src/index.ts";

const Start = 1_000;
const Generation = 7;

/** A clock the test moves by hand. */
const FakeClock = (start = Start): { clock: Clock; advance: (ms: number) => void } => {
	let now = start;
	return {
		clock: { now: () => now },
		advance(ms: number): void {
			now += ms;
		},
	};
};

/** A promise whose settlement the test decides. */
const Deferred = () => {
	let settle: (value: string) => void = () => undefined;
	const promise = new Promise<string>((resolve) => {
		settle = resolve;
	});
	return { promise, settle: (value: string) => settle(value) };
};

/** One runtime over one supplied clock, and the list its writes land in. */
const Harness = () => {
	const { clock, advance } = FakeClock();
	const writes: string[] = [];
	const runtime = Create({
		generation: Generation,
		capabilities: [
			{ name: "browse", state: "holds-transport" },
			{ name: "chat", state: "holds-identity" },
		],
		clock,
	});
	const write = async (text: string): Promise<string> => {
		writes.push(text);
		return text;
	};
	return { runtime, advance, writes, write };
};

/** The refusal code a call rejects with, asserting that it rejected at all. */
const Refuses = async (
	call: Promise<unknown>,
	code: (typeof ErrorCode)[keyof typeof ErrorCode],
): Promise<RuntimeError> => {
	try {
		await call;
	} catch (error) {
		assert.ok(error instanceof RuntimeError, `expected a RuntimeError, got ${String(error)}`);
		assert.equal(error.code, code);
		return error;
	}
	return assert.fail(`expected the call to reject with ${code}`);
};

/** The reason `admit` refuses a request with, asserting that it refused. */
const Refusal = async (
	runtime: Policy,
	request: { capability: string; state: StateClass; generation: number },
): Promise<string> => {
	const result = await runtime.admit(request);
	if (result.ok) return assert.fail(`expected ${request.capability} to be refused`);
	return result.reason;
};

describe("agentic.runtime", () => {
	test("the port declares exactly the errors the manifest promises", () => {
		const manifest = JSON.parse(
			readFileSync(new URL("../module.json", import.meta.url), "utf8"),
		) as { provides: { errors: string[] } };
		assert.deepEqual(Object.values(ErrorCode).sort(), [...manifest.provides.errors].sort());
	});

	test("reads the clock it was given, not the wall clock", () => {
		const { runtime, advance } = Harness();
		assert.equal(runtime.now(), Start);
		advance(250);
		assert.equal(runtime.now(), Start + 250);
	});
});

describe("admission", () => {
	test("admits a registered capability at the generation it is bound to", async () => {
		const { runtime } = Harness();
		assert.deepEqual(
			await runtime.admit({
				capability: "browse",
				state: "holds-transport",
				generation: Generation,
			}),
			{ ok: true },
		);
	});

	test("refuses a generation it was not bound to", async () => {
		const { runtime } = Harness();
		const reason = await Refusal(runtime, {
			capability: "browse",
			state: "holds-transport",
			generation: Generation + 1,
		});
		assert.match(reason, /^admission-refused: generation 8 is not bound/);
	});

	test("refuses a capability it does not register", async () => {
		const { runtime } = Harness();
		const reason = await Refusal(runtime, {
			capability: "design-verification",
			state: "stateless",
			generation: Generation,
		});
		assert.match(reason, /is not registered/);
	});

	test("refuses a state class the registration does not declare", async () => {
		const { runtime } = Harness();
		const reason = await Refusal(runtime, {
			capability: "browse",
			state: "holds-workspace",
			generation: Generation,
		});
		assert.match(reason, /declares holds-transport, not holds-workspace/);
	});

	test("the manifest it reports is the manifest it enforces", async () => {
		const { runtime } = Harness();
		const binding = runtime.reflect();
		assert.equal(binding.generation, Generation);
		assert.deepEqual(
			binding.capabilities.map((capability) => capability.name),
			["browse", "chat"],
		);
		for (const capability of binding.capabilities) {
			assert.deepEqual(
				await runtime.admit({
					capability: capability.name,
					state: capability.state,
					generation: binding.generation,
				}),
				{ ok: true },
			);
		}
		// The copy is a copy: editing what was reported changes nothing.
		const reported = binding.capabilities as {
			name: string;
			state: StateClass;
		}[];
		reported.push({ name: "ghost", state: "stateless" });
		assert.match(
			await Refusal(runtime, {
				capability: "ghost",
				state: "stateless",
				generation: Generation,
			}),
			/is not registered/,
		);
	});
});

describe("leases", () => {
	test("a second claim is refused while the first is live", async () => {
		const { runtime } = Harness();
		const first = await runtime.claim("browse", 1_000);
		const refusal = await Refuses(runtime.claim("browse", 1_000), ErrorCode.LeaseHeld);
		assert.match(refusal.message, new RegExp(`fence ${first.fence}`));
	});

	test("a claim takes over an expired lease, and the fence advances", async () => {
		const { runtime, advance } = Harness();
		const first = await runtime.claim("browse", 1_000);
		advance(1_000);
		const second = await runtime.claim("browse", 1_000);
		assert.equal(second.fence, first.fence + 1);
		assert.notEqual(second.id, first.id);
	});

	test("a stale fence is refused and the write never lands", async () => {
		const { runtime, advance, writes, write } = Harness();
		const first = await runtime.claim("browse", 1_000);
		advance(1_000);
		const second = await runtime.claim("browse", 1_000);
		await Refuses(
			runtime.withLease(first, () => write("late")),
			ErrorCode.LeaseHeld,
		);
		assert.deepEqual(writes, []);
		assert.equal(await runtime.withLease(second, () => write("now")), "now");
		assert.deepEqual(writes, ["now"]);
	});

	test("a lease that expires mid-run refuses the next write", async () => {
		const { runtime, advance, writes, write } = Harness();
		const lease = await runtime.claim("browse", 1_000);
		assert.equal(await runtime.withLease(lease, () => write("early")), "early");
		advance(1_000);
		const refusal = await Refuses(
			runtime.withLease(lease, () => write("late")),
			ErrorCode.AdmissionRefused,
		);
		assert.match(refusal.message, /expired at/);
		assert.deepEqual(writes, ["early"]);
	});

	test("a renewal extends the lease for the current holder", async () => {
		const { runtime, advance, write } = Harness();
		const lease = await runtime.claim("browse", 1_000);
		advance(900);
		const renewed = await runtime.renew(lease, 1_000);
		assert.equal(renewed.expiresAt, Start + 1_900);
		assert.equal(renewed.fence, lease.fence);
		advance(500);
		assert.equal(await runtime.withLease(renewed, () => write("kept")), "kept");
	});

	test("a renewal that does not extend the lease is refused", async () => {
		const { runtime, advance } = Harness();
		const lease = await runtime.claim("browse", 1_000);
		advance(500);
		const refusal = await Refuses(runtime.renew(lease, 1), ErrorCode.AdmissionRefused);
		assert.match(refusal.message, /must extend/);
	});

	test("a renewal after expiry is refused rather than resurrected", async () => {
		const { runtime, advance } = Harness();
		const lease = await runtime.claim("browse", 1_000);
		advance(1_000);
		const refusal = await Refuses(runtime.renew(lease, 1_000), ErrorCode.AdmissionRefused);
		assert.match(refusal.message, /expired/);
	});

	test("a renewal from a superseded lease is refused", async () => {
		const { runtime, advance } = Harness();
		const first = await runtime.claim("browse", 1_000);
		advance(1_000);
		await runtime.claim("browse", 1_000);
		await Refuses(runtime.renew(first, 1_000), ErrorCode.LeaseHeld);
	});

	test("a lease from another runtime is not this runtime's lease", async () => {
		const { runtime, writes, write } = Harness();
		const { clock } = FakeClock();
		const sibling = Create({
			generation: Generation,
			capabilities: [{ name: "browse", state: "holds-transport" }],
			clock,
		});
		const mine = await runtime.claim("browse", 1_000);
		const theirs = await sibling.claim("browse", 1_000);
		assert.equal(theirs.fence, mine.fence);
		await Refuses(
			runtime.withLease(theirs, () => write("foreign")),
			ErrorCode.AdmissionRefused,
		);
		assert.deepEqual(writes, []);
	});

	test("a claim during an effect on an expired lease is refused", async () => {
		const { runtime, advance, write } = Harness();
		const lease = await runtime.claim("browse", 1_000);
		const gate = Deferred();
		const started = Deferred();
		const inFlight = runtime.withLease(lease, async () => {
			started.settle("writing");
			await gate.promise;
			return write("settled");
		});
		await started.promise;
		advance(1_000);
		await Refuses(runtime.claim("browse", 1_000), ErrorCode.LeaseHeld);
		gate.settle("done");
		assert.equal(await inFlight, "settled");
		const taken = await runtime.claim("browse", 1_000);
		assert.equal(taken.fence, lease.fence + 1);
	});

	test("a malformed claim is refused, not thrown as a TypeError", async () => {
		const { runtime } = Harness();
		await Refuses(runtime.claim("browse", 0), ErrorCode.AdmissionRefused);
		await Refuses(runtime.claim("", 1_000), ErrorCode.AdmissionRefused);
		await Refuses(
			runtime.claim("browse", 1_000, { deadline: NaN, operations: 1 }),
			ErrorCode.AdmissionRefused,
		);
	});
});

describe("release", () => {
	test("release is safe twice, and the next claim takes a new fence", async () => {
		const { runtime, writes, write } = Harness();
		const lease = await runtime.claim("browse", 1_000);
		await runtime.release(lease);
		await runtime.release(lease);
		await Refuses(
			runtime.withLease(lease, () => write("after")),
			ErrorCode.AdmissionRefused,
		);
		assert.deepEqual(writes, []);
		const next = await runtime.claim("browse", 1_000);
		assert.equal(next.fence, lease.fence + 1);
	});

	test("releasing a superseded lease does not free the new holder's resource", async () => {
		const { runtime, advance, write } = Harness();
		const first = await runtime.claim("browse", 1_000);
		advance(1_000);
		const second = await runtime.claim("browse", 1_000);
		await runtime.release(first);
		assert.equal(await runtime.withLease(second, () => write("held")), "held");
	});

	test("release during an effect frees the resource once it settles", async () => {
		const { runtime, write } = Harness();
		const lease = await runtime.claim("browse", 1_000);
		const gate = Deferred();
		const started = Deferred();
		const inFlight = runtime.withLease(lease, async () => {
			started.settle("writing");
			await gate.promise;
			return write("settled");
		});
		await started.promise;
		await runtime.release(lease);
		await Refuses(runtime.claim("browse", 1_000), ErrorCode.LeaseHeld);
		gate.settle("done");
		await inFlight;
		const taken = await runtime.claim("browse", 1_000);
		assert.equal(taken.fence, lease.fence + 1);
	});

	test("release never throws, whatever it is handed", async () => {
		const { runtime } = Harness();
		await runtime.release(undefined as unknown as Lease);
		await runtime.release({} as Lease);
		await runtime.release({
			id: "never-issued",
			resource: "browse",
			fence: 1,
			expiresAt: Start,
		});
	});
});

describe("budgets", () => {
	const Allowance = (operations: number, deadline = Infinity): Budget => ({
		deadline,
		operations,
	});

	test("a budget is not inherited: a new run starts from the caller's count", async () => {
		const { runtime, advance, writes, write } = Harness();
		const first = await runtime.claim("browse", 1_000, Allowance(2));
		await runtime.withLease(first, () => write("a"));
		await runtime.withLease(first, () => write("b"));
		await Refuses(
			runtime.withLease(first, () => write("c")),
			ErrorCode.BudgetExhausted,
		);
		assert.deepEqual(writes, ["a", "b"]);
		advance(1_000);
		const second = await runtime.claim("browse", 1_000, Allowance(2));
		assert.equal(await runtime.withLease(second, () => write("d")), "d");
		assert.equal(await runtime.withLease(second, () => write("e")), "e");
		assert.deepEqual(writes, ["a", "b", "d", "e"]);
	});

	test("a budget's deadline refuses effects past it", async () => {
		const { runtime, advance, writes, write } = Harness();
		const lease = await runtime.claim("browse", 5_000, Allowance(10, Start + 1_000));
		await runtime.withLease(lease, () => write("in time"));
		advance(1_000);
		const refusal = await Refuses(
			runtime.withLease(lease, () => write("too late")),
			ErrorCode.BudgetExhausted,
		);
		assert.match(refusal.message, /deadline passed/);
		assert.deepEqual(writes, ["in time"]);
	});

	test("an effect that throws still spends an operation", async () => {
		const { runtime } = Harness();
		const lease = await runtime.claim("browse", 1_000, Allowance(1));
		await assert.rejects(
			runtime.withLease(lease, async () => {
				throw new Error("the resource refused");
			}),
			/the resource refused/,
		);
		await Refuses(
			runtime.withLease(lease, async () => "second"),
			ErrorCode.BudgetExhausted,
		);
	});

	test("a run with no budget is unbounded", async () => {
		const { runtime, writes, write } = Harness();
		const lease = await runtime.claim("browse", 60_000);
		for (const text of ["a", "b", "c", "d", "e"])
			await runtime.withLease(lease, () => write(text));
		assert.equal(writes.length, 5);
	});

	test("a budget of no operations refuses every effect", async () => {
		const { runtime, writes, write } = Harness();
		const lease = await runtime.claim("browse", 1_000, Allowance(0));
		await Refuses(
			runtime.withLease(lease, () => write("none")),
			ErrorCode.BudgetExhausted,
		);
		assert.deepEqual(writes, []);
	});

	test("one run's budget is not another resource's", async () => {
		const { runtime, writes, write } = Harness();
		const browse = await runtime.claim("browse", 1_000, Allowance(1));
		const chat = await runtime.claim("chat", 1_000, Allowance(1));
		await runtime.withLease(browse, () => write("browse"));
		assert.equal(await runtime.withLease(chat, () => write("chat")), "chat");
		await Refuses(
			runtime.withLease(browse, () => write("spent")),
			ErrorCode.BudgetExhausted,
		);
		assert.deepEqual(writes, ["browse", "chat"]);
	});
});

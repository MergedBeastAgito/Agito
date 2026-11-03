/**
 * The four invariants, each proved by a refusal that wrote nothing.
 *
 * The fake context records every write, so "refused" and "refused without
 * writing" are different assertions here: a rule that returned a refusal after
 * mutating a row would fail the second one. It also refuses to rewrite an
 * append-only table and checks every row against the catalog, so a reducer
 * cannot quietly break either promise and still pass.
 *
 *   node --test Test/*.test.ts
 */
import assert from "node:assert/strict";
import { test } from "node:test";

import {
	bind,
	claim,
	probe,
	recordTrace,
	renew,
	type Claim,
	type Probe,
	type Renew,
	type Trace,
} from "../src/index.ts";
import { Memory, type Write } from "./Fake.ts";

const Browse = "agentic.browse@0.1.0";
const Chat = "agentic.chat@0.1.0";
const Resource = "browser:default";

/** A database whose catalog already declares two capabilities. */
const Catalog = (sender = "agent:a", now = 0): Memory => {
	const ctx = new Memory(sender, now);
	for (const id of [Browse, Chat]) {
		const at = id.indexOf("@");
		ctx.insert("capability", {
			id,
			name: id.slice(0, at),
			version: id.slice(at + 1),
			state: "stateless",
			declaredAt: ctx.now,
			declaredBy: "module",
		});
	}
	return ctx;
};

/** The writes made since `mark`, which is how "wrote nothing" is asserted. */
const Written = (ctx: Memory, mark: number): Write[] => ctx.writes.slice(mark);

/** Bind a generation and answer its id. */
const Bound = (
	ctx: Memory,
	previous: string | null = null,
	digest = "sha256:first",
): string => {
	assert.deepEqual(
		bind(ctx, {
			previous,
			manifest: { digest, capabilities: [Browse] },
		}),
		{ ok: true },
	);
	return previous === null ? "1" : String(Number(previous) + 1);
};

const Claimed = (ctx: Memory, overrides: Partial<Claim> = {}) =>
	claim(ctx, {
		resource: Resource,
		generation: "1",
		task: "task:1",
		ttl: 1_000,
		...overrides,
	});

const Renewed = (ctx: Memory, overrides: Partial<Renew> = {}) =>
	renew(ctx, {
		resource: Resource,
		generation: "1",
		fence: 1,
		ttl: 1_000,
		...overrides,
	});

const Probed = (ctx: Memory, overrides: Partial<Probe> = {}) =>
	probe(ctx, {
		generation: "1",
		resource: Resource,
		fence: 1,
		property: "agentic.browse.reliability",
		version: 3,
		observed: "high",
		...overrides,
	});

const Traced = (ctx: Memory, overrides: Partial<Trace> = {}) =>
	recordTrace(ctx, {
		generation: "1",
		kind: "lease.claimed",
		subject: Resource,
		payload: '{"fence":1}',
		...overrides,
	});

/** A generation bound, a lease claimed and held by `agent:a`. */
const Leased = (ctx: Memory): void => {
	Bound(ctx);
	Claimed(ctx);
};

test("bind appends the first generation at the sender's time, and writes exactly one row", () => {
	const ctx = Catalog("agent:a", 1_000);
	const mark = ctx.writes.length;
	assert.deepEqual(
		bind(ctx, {
			previous: null,
			manifest: { digest: "sha256:first", capabilities: [Browse] },
		}),
		{ ok: true },
	);
	assert.deepEqual(ctx.rows("generation"), [
		{
			id: "1",
			sequence: 1,
			parent: null,
			manifestDigest: "sha256:first",
			capabilities: JSON.stringify([Browse]),
			boundAt: 1_000,
			boundBy: "agent:a",
		},
	]);
	assert.deepEqual(
		Written(ctx, mark).map((write) => [write.kind, write.table]),
		[["insert", "generation"]],
	);
});

test("bind refuses a manifest that names a capability the catalog does not declare, and writes nothing", () => {
	const ctx = Catalog();
	const mark = ctx.writes.length;
	assert.deepEqual(
		bind(ctx, {
			previous: null,
			manifest: {
				digest: "sha256:first",
				capabilities: [Browse, "agentic.vote@1.0.0"],
			},
		}),
		{ ok: false, reason: "admission-refused" },
	);
	assert.deepEqual(Written(ctx, mark), []);
	assert.equal(ctx.find("generation", "1"), null);
});

test("bind appends a successor and leaves the previous generation untouched", () => {
	const ctx = Catalog("agent:a", 1_000);
	Bound(ctx);
	const first = ctx.find("generation", "1");
	assert.ok(first);
	ctx.now = 2_000;
	const mark = ctx.writes.length;
	assert.deepEqual(
		bind(ctx, {
			previous: "1",
			manifest: { digest: "sha256:second", capabilities: [Browse, Chat] },
		}),
		{ ok: true },
	);
	assert.deepEqual(ctx.find("generation", "1"), first);
	assert.deepEqual(
		ctx.rows("generation").map((row) => row["id"]),
		["1", "2"],
	);
	assert.deepEqual(
		Written(ctx, mark).map((write) => [write.kind, write.table]),
		[["insert", "generation"]],
	);
	assert.equal(ctx.find("generation", "2")?.["parent"], "1");
	assert.equal(ctx.find("generation", "2")?.["boundAt"], 2_000);
	assert.equal(
		ctx.find("generation", "2")?.["capabilities"],
		JSON.stringify([Browse, Chat].sort()),
	);
});

test("bind refuses a predecessor that is already superseded, or that was never bound", () => {
	const ctx = Catalog();
	Bound(ctx);
	Bound(ctx, "1", "sha256:second");
	const mark = ctx.writes.length;
	assert.deepEqual(
		bind(ctx, {
			previous: "1",
			manifest: { digest: "sha256:x", capabilities: [Browse] },
		}),
		{ ok: false, reason: "admission-refused" },
	);
	assert.deepEqual(
		bind(ctx, {
			previous: "9",
			manifest: { digest: "sha256:x", capabilities: [Browse] },
		}),
		{ ok: false, reason: "admission-refused" },
	);
	assert.deepEqual(Written(ctx, mark), []);
	assert.deepEqual(
		ctx.rows("generation").map((row) => row["id"]),
		["1", "2"],
	);
});

test("a claim while a lease is live is lease-held, and writes nothing", () => {
	const ctx = Catalog("agent:a", 0);
	Leased(ctx);
	ctx.now = 999;
	const mark = ctx.writes.length;
	assert.deepEqual(Claimed(ctx, { task: "task:2" }), {
		ok: false,
		reason: "lease-held",
	});
	assert.deepEqual(Written(ctx, mark), []);
	assert.equal(ctx.find("lease", Resource)?.["task"], "task:1");
	assert.equal(ctx.find("lease", Resource)?.["fence"], 1);
});

test("the fence: a claim past expiry increments it, and the first fence can no longer renew", () => {
	const ctx = Catalog("agent:a", 0);
	Bound(ctx);
	assert.deepEqual(Claimed(ctx, { ttl: 1_000 }), { ok: true });
	assert.equal(ctx.find("lease", Resource)?.["fence"], 1);

	ctx.now = 1_001;
	assert.deepEqual(Claimed(ctx, { task: "task:2", ttl: 1_000 }), {
		ok: true,
	});
	assert.equal(ctx.find("lease", Resource)?.["fence"], 2);

	const mark = ctx.writes.length;
	assert.deepEqual(Renewed(ctx, { fence: 1 }), {
		ok: false,
		reason: "admission-refused",
	});
	assert.deepEqual(Written(ctx, mark), []);
	assert.equal(ctx.find("lease", Resource)?.["fence"], 2);
	assert.equal(ctx.find("lease", Resource)?.["expiresAt"], 2_001);
});

test("renew extends a live lease at the sender's time", () => {
	const ctx = Catalog("agent:a", 0);
	Leased(ctx);
	ctx.now = 500;
	assert.deepEqual(Renewed(ctx, { ttl: 2_000 }), { ok: true });
	assert.equal(ctx.find("lease", Resource)?.["renewedAt"], 500);
	assert.equal(ctx.find("lease", Resource)?.["expiresAt"], 2_500);
	assert.equal(ctx.find("lease", Resource)?.["acquiredAt"], 0);
	assert.equal(ctx.find("lease", Resource)?.["fence"], 1);
});

test("an expired lease cannot be renewed back to life", () => {
	const ctx = Catalog("agent:a", 0);
	Leased(ctx);
	ctx.now = 1_000;
	const mark = ctx.writes.length;
	assert.deepEqual(Renewed(ctx), { ok: false, reason: "admission-refused" });
	assert.deepEqual(Written(ctx, mark), []);
	assert.equal(ctx.find("lease", Resource)?.["expiresAt"], 1_000);
});

test("a lease is renewed only by the sender that holds it", () => {
	const ctx = Catalog("agent:a", 0);
	Leased(ctx);
	ctx.as("agent:b");
	const mark = ctx.writes.length;
	assert.deepEqual(Renewed(ctx), { ok: false, reason: "admission-refused" });
	assert.deepEqual(Written(ctx, mark), []);
});

test("a probe writes exactly one row, carrying the audience and the property version, not the private value", () => {
	const ctx = Catalog("agent:a", 0);
	Leased(ctx);
	ctx.now = 10;
	// The private truth the owner computed the disclosure from. It is deliberately
	// never passed to the reducer: a secret the database was never told cannot be
	// selected from it, and this is the canary that says so.
	const secret = "internal-reliability-0.97";
	const mark = ctx.writes.length;
	const result = Probed(ctx, { observed: "high" });
	const observation = {
		id: "agentic.browse.reliability:3:agent:a",
		property: "agentic.browse.reliability",
		version: 3,
		audience: "agent:a",
		observed: "high",
		at: 10,
		generation: "1",
	};
	assert.deepEqual(result, { ok: true, observation });
	assert.deepEqual(ctx.rows("probe"), [observation]);
	assert.deepEqual(
		Written(ctx, mark).map((write) => [write.kind, write.table]),
		[["insert", "probe"]],
	);
	const everything = ["generation", "lease", "probe", "trace"]
		.flatMap((table) => ctx.rows(table))
		.map((row) => JSON.stringify(row))
		.join("\n");
	assert.ok(!everything.includes(secret));
});

test("a second probe by the same audience at the same property version is budget-exhausted, and writes nothing", () => {
	const ctx = Catalog("agent:a", 0);
	Leased(ctx);
	ctx.now = 10;
	assert.equal(Probed(ctx).ok, true);
	const mark = ctx.writes.length;
	ctx.now = 20;
	assert.deepEqual(Probed(ctx, { observed: "low" }), {
		ok: false,
		reason: "budget-exhausted",
	});
	assert.deepEqual(Written(ctx, mark), []);
	assert.equal(ctx.rows("probe").length, 1);
	assert.equal(ctx.rows("probe")[0]?.["observed"], "high");
});

test("the allowance is per audience, so another holder gets its own observation", () => {
	const ctx = Catalog("agent:a", 0);
	Leased(ctx);
	ctx.now = 10;
	assert.equal(Probed(ctx).ok, true);

	ctx.as("agent:b").now = 20;
	assert.deepEqual(
		Claimed(ctx, { resource: "browser:second", task: "task:2" }),
		{ ok: true },
	);
	const result = Probed(ctx, { resource: "browser:second" });
	assert.equal(result.ok, true);
	assert.deepEqual(
		ctx.rows("probe").map((row) => row["audience"]),
		["agent:a", "agent:b"],
	);
});

test("a probe carrying a stale fence is refused, and writes nothing", () => {
	const ctx = Catalog("agent:a", 0);
	Leased(ctx);
	ctx.now = 1_001;
	assert.deepEqual(Claimed(ctx, { task: "task:2" }), { ok: true });
	const mark = ctx.writes.length;
	assert.deepEqual(Probed(ctx, { fence: 1 }), {
		ok: false,
		reason: "admission-refused",
	});
	assert.deepEqual(Written(ctx, mark), []);
	assert.deepEqual(ctx.rows("probe"), []);
});

test("a claim or a probe under a superseded generation is refused, and writes nothing", () => {
	const ctx = Catalog("agent:a", 0);
	Leased(ctx);
	Bound(ctx, "1", "sha256:second");
	ctx.now = 10;
	const mark = ctx.writes.length;
	assert.deepEqual(Claimed(ctx, { resource: "browser:second" }), {
		ok: false,
		reason: "admission-refused",
	});
	assert.deepEqual(Probed(ctx), { ok: false, reason: "admission-refused" });
	assert.deepEqual(Written(ctx, mark), []);
	assert.deepEqual(ctx.rows("probe"), []);
});

test("recordTrace appends once per instant, kind and subject, and refuses different content under the same key", () => {
	const ctx = Catalog("agent:a", 1_000);
	Bound(ctx);
	const mark = ctx.writes.length;
	assert.deepEqual(Traced(ctx), { ok: true });
	assert.deepEqual(Traced(ctx), { ok: true });
	assert.equal(Written(ctx, mark).length, 1);
	assert.equal(
		ctx.rows("trace")[0]?.["id"],
		`1000:lease.claimed:${Resource}`,
	);
	assert.equal(ctx.rows("trace")[0]?.["sender"], "agent:a");

	const second = ctx.writes.length;
	assert.deepEqual(Traced(ctx, { payload: '{"fence":2}' }), {
		ok: false,
		reason: "admission-refused",
	});
	assert.deepEqual(Written(ctx, second), []);
});

test("recordTrace refuses a generation that is not the latest bound", () => {
	const ctx = Catalog();
	Bound(ctx);
	Bound(ctx, "1", "sha256:second");
	const mark = ctx.writes.length;
	assert.deepEqual(Traced(ctx), { ok: false, reason: "admission-refused" });
	assert.deepEqual(Written(ctx, mark), []);
});

test("every timestamp a row carries is ctx.now, so a fake clock decides the whole transcript", () => {
	const ctx = Catalog("agent:a", 1_234_567);
	const mark = ctx.writes.length;
	Bound(ctx);
	Claimed(ctx, { ttl: 100 });
	Probed(ctx);
	Traced(ctx);
	const times = Written(ctx, mark).flatMap((write) =>
		Object.entries(write.row ?? {}).filter(
			([column, value]) =>
				/(^at$|At$)/.test(column) &&
				column !== "expiresAt" &&
				typeof value === "number",
		),
	);
	assert.ok(
		times.length >= 4,
		`only ${times.length} timestamps were written`,
	);
	for (const [column, value] of times)
		assert.equal(value, 1_234_567, `${column} did not come from ctx.now`);
});

test("every rule answers a malformed argument with a refusal, and throws nothing", () => {
	const ctx = Catalog();
	Bound(ctx);
	const mark = ctx.writes.length;
	const calls: (() => { ok: boolean })[] = [
		() =>
			bind(ctx, {
				previous: null,
				manifest: { digest: "", capabilities: [Browse] },
			}),
		() =>
			bind(ctx, {
				previous: null,
				manifest: {
					digest: "sha256:x",
					capabilities: [Browse, Browse],
				},
			}),
		() => Claimed(ctx, { resource: "" }),
		() => Claimed(ctx, { generation: "nowhere" }),
		() => Claimed(ctx, { ttl: 0 }),
		() => Renewed(ctx, { ttl: -1 }),
		() => Renewed(ctx, { generation: "nowhere" }),
		() => Probed(ctx, { property: "" }),
		() => Probed(ctx, { version: 0 }),
		() => Probed(ctx, { fence: 99 }),
		() => Traced(ctx, { kind: "" }),
		() => Traced(ctx, { generation: "nowhere" }),
	];
	for (const call of calls) assert.equal(call().ok, false);
	assert.deepEqual(Written(ctx, mark), []);
});

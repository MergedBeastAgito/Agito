/**
 * The destination's rule, as the reducer makes it - and the mirror of the Rust
 * late-arrival test.
 *
 * `rust/Crate/Runtime/Test/Runtime.rs` runs the same transcript against the local
 * stand-in:
 *
 *   claim a resource
 *   admit an effect
 *   let the lease expire while the effect is in flight
 *   let the effect perform its external action
 *   present the record
 *
 * and asserts the same thing this file asserts: the record is refused, the effect
 * is never counted as a success, the refusal is written as an ambiguity, and the
 * resource is not reassigned until a resolver says what happened. Two
 * implementations, one answer. If they ever disagree, one of them is wrong and the
 * other is a second authority.
 *
 * The vocabulary matches too: this side refuses with `"admission-refused"`, which
 * is what `Code::AdmissionRefused` names on the Rust side, and both manifests
 * declare that code.
 *
 *   node --test Test/*.test.ts
 */
import assert from "node:assert/strict";
import { test } from "node:test";

import { bind, claim, record, resolve, type Claim, type Effect } from "../src/index.ts";
import { Memory, type Write } from "./Fake.ts";

const Browse = "agentic.browse@0.1.0";
const Resource = "browser:default";

/** A database with one capability declared and generation 1 bound. */
const Catalog = (sender = "agent:a", now = 0): Memory => {
	const ctx = new Memory(sender, now);
	const at = Browse.indexOf("@");
	ctx.insert("capability", {
		id: Browse,
		name: Browse.slice(0, at),
		version: Browse.slice(at + 1),
		state: "stateless",
		declaredAt: ctx.now,
		declaredBy: "module",
	});
	assert.deepEqual(
		bind(ctx, { previous: null, manifest: { digest: "sha256:first", capabilities: [Browse] } }),
		{ ok: true },
	);
	return ctx;
};

const Claimed = (ctx: Memory, overrides: Partial<Claim> = {}) =>
	claim(ctx, { resource: Resource, generation: "1", task: "task:1", ttl: 1_000, ...overrides });

const Recorded = (ctx: Memory, overrides: Partial<Effect> = {}) =>
	record(ctx, { generation: "1", resource: Resource, fence: 1, name: "write", ...overrides });

/** The writes made since `mark`, which is how "wrote nothing" is asserted. */
const Written = (ctx: Memory, mark: number): Write[] => ctx.writes.slice(mark);

const Kinds = (ctx: Memory): string[] => ctx.rows("trace").map((row) => String(row["kind"]));

test("an effect that outlives its lease is refused where it lands and recorded ambiguous", () => {
	// The Rust transcript, step for step. The external action is unobservable from
	// here - that is the whole point of an effect - so what this side can prove is
	// the other half: the record was refused, the effect was never counted, and the
	// refusal is not silence.
	const ctx = Catalog("agent:a", 0);
	assert.deepEqual(Claimed(ctx), { ok: true });
	ctx.now = 1_001;
	const mark = ctx.writes.length;
	assert.deepEqual(Recorded(ctx), { ok: false, reason: "admission-refused" });

	// Two writes, and neither of them is the effect: the ambiguity, then the
	// quarantine. A refused effect leaves nothing in the effect's own history.
	const writes = Written(ctx, mark);
	assert.deepEqual(
		writes.map((write) => [write.kind, write.table]),
		[
			["insert", "trace"],
			["update", "lease"],
		],
	);
	assert.equal(writes[0]?.row?.["kind"], "effect-ambiguous");
	assert.equal(writes[1]?.row?.["state"], "quarantined");
	assert.deepEqual(JSON.parse(String(writes[0]?.row?.["payload"])), {
		name: "write",
		fence: 1,
		reason: "outlived its authority",
	});

	// The resource is not reassigned while the outcome is unknown.
	assert.deepEqual(Claimed(ctx), { ok: false, reason: "lease-held" });
	assert.equal(ctx.find("lease", Resource)?.["state"], "quarantined");

	// A resolver states what happened, and only then is reassignment possible.
	assert.deepEqual(resolve(ctx, { resource: Resource, generation: "1", outcome: "landed" }), {
		ok: true,
	});
	assert.deepEqual(
		JSON.parse(
			String(ctx.rows("trace").find((row) => row["kind"] === "effect-resolved")?.["payload"]),
		),
		{ outcome: "landed", fence: 1 },
	);
	// The ambiguity stays. A resolution that erased the question would be the
	// retraction the handover rule forbids.
	assert.equal(Kinds(ctx).filter((kind) => kind === "effect-ambiguous").length, 1);

	assert.deepEqual(Claimed(ctx), { ok: true });
	assert.equal(ctx.find("lease", Resource)?.["fence"], 2);
	// Across the whole transcript, the effect was never recorded as a success.
	assert.equal(Kinds(ctx).filter((kind) => kind === "effect").length, 0);
});

test("a caller that reproduces the row's numbers is still not the holder", () => {
	// The review's first gap. Every number this caller presents is correct - the
	// generation, the resource, the fence - and it is refused, because the row says
	// who holds it and the transport says who is asking.
	const ctx = Catalog("agent:a", 0);
	assert.deepEqual(Claimed(ctx), { ok: true });
	ctx.as("agent:b");
	const mark = ctx.writes.length;
	assert.deepEqual(Recorded(ctx), { ok: false, reason: "admission-refused" });
	// Nothing was admitted for this caller, so there is no external action to be
	// ambiguous about, and the refusal writes nothing at all.
	assert.deepEqual(Written(ctx, mark), []);
	assert.equal(ctx.find("lease", Resource)?.["state"], "held");
	assert.deepEqual(
		Kinds(ctx).filter((kind) => kind === "effect-ambiguous"),
		[],
	);
});

test("an effect from a superseded fence is refused without an ambiguity", () => {
	// A superseded epoch is where the takeover, not the destination, does the
	// accounting: the row no longer describes that epoch, so the rule refuses and
	// says nothing about an effect it cannot attribute.
	const ctx = Catalog("agent:a", 0);
	assert.deepEqual(Claimed(ctx), { ok: true });
	ctx.now = 1_001;
	assert.deepEqual(Claimed(ctx), { ok: true });
	assert.equal(ctx.find("lease", Resource)?.["fence"], 2);
	const mark = ctx.writes.length;
	assert.deepEqual(Recorded(ctx), { ok: false, reason: "admission-refused" });
	assert.deepEqual(Written(ctx, mark), []);
	assert.equal(ctx.find("lease", Resource)?.["state"], "held");
});

test("an effect whose lease was released while it ran is refused and recorded ambiguous", () => {
	// Release is the other way authority ends. The catalog has no `release`
	// reducer, so the row is moved here the way `resolve` moves it: the rule reads
	// the state, not the call that set it.
	const ctx = Catalog("agent:a", 0);
	assert.deepEqual(Claimed(ctx), { ok: true });
	ctx.update("lease", { ...ctx.find("lease", Resource)!, state: "released" });
	const mark = ctx.writes.length;
	assert.deepEqual(Recorded(ctx), { ok: false, reason: "admission-refused" });
	const writes = Written(ctx, mark);
	assert.equal(writes[0]?.row?.["kind"], "effect-ambiguous");
	assert.deepEqual(JSON.parse(String(writes[0]?.row?.["payload"])), {
		name: "write",
		fence: 1,
		reason: "released while the effect was in flight",
	});
	assert.equal(ctx.find("lease", Resource)?.["state"], "quarantined");
});

test("an effect presented while its lease is live is recorded, not quarantined", () => {
	const ctx = Catalog("agent:a", 0);
	assert.deepEqual(Claimed(ctx), { ok: true });
	const mark = ctx.writes.length;
	assert.deepEqual(Recorded(ctx), { ok: true });
	const writes = Written(ctx, mark);
	assert.deepEqual(
		writes.map((write) => [write.kind, write.table]),
		[["insert", "trace"]],
	);
	assert.equal(writes[0]?.row?.["kind"], "effect");
	assert.equal(
		ctx.find("lease", Resource)?.["state"],
		"held",
		"a recorded effect changes no lease",
	);
	assert.deepEqual(
		Kinds(ctx).filter((kind) => kind === "effect-ambiguous"),
		[],
	);
});

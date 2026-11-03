/**
 * The contract test for `agentic.verify.design`.
 *
 * No browser, no build, no served page: every measurement below is supplied by a
 * transport written in this file, so what each rule is about is visible here
 * rather than hidden behind a running site. The selectors and lengths in this
 * file are the test's own design - the package holds none, which the source test
 * near the end proves by reading it.
 *
 *   node --test Test/*.test.ts
 */
import assert from "node:assert/strict";
import { readdir, readFile } from "node:fs/promises";
import { test } from "node:test";

import {
	Create,
	ErrorCode,
	VerifyError,
	type Evidence,
	type Invariant,
	type Measurement,
	type Transport,
} from "../src/index.ts";

/** A deadline no test waits for, and a signal nobody fires. */
const Options = (): { timeoutMs: number; signal: AbortSignal } => ({
	timeoutMs: 1_000,
	signal: new AbortController().signal,
});

/** One reading a fixture transport holds for a selector. */
type Reading = {
	present: boolean;
	value?: number | string | null;
};

/** A transport that answers from a table, and records what it was asked. */
type Fixture = Transport & { asked: string[][] };

const Answering = (readings: Record<string, Reading>): Fixture => {
	const asked: string[][] = [];
	return {
		asked,
		async measure(selectors) {
			asked.push([...selectors]);
			return selectors.map((selector) => {
				const reading = readings[selector];
				return {
					selector,
					present: reading?.present === true,
					value: reading?.present === true ? (reading.value ?? null) : null,
				};
			});
		},
	};
};

/** The capability under test, over a transport that answers nothing. */
const UnderTest = () => Create(Answering({}));

const Present = (selector: string, value: number | string | null): Measurement => ({
	selector,
	present: true,
	value,
});

const Absent = (selector: string): Measurement => ({
	selector,
	present: false,
	value: null,
});

const InvariantOf = (
	label: string,
	read: (measurements: Measurement[]) => unknown,
	want: unknown,
): Invariant => ({ label, read, want });

test("measure asks for every selector once and answers in the order it was given", async () => {
	const fixture = Answering({
		".rail": { present: true, value: 224 },
		".drawer": { present: true, value: 360 },
	});
	const measurements = await Create(fixture).measure([".rail", ".drawer"], Options());
	assert.deepEqual(fixture.asked, [[".rail", ".drawer"]]);
	assert.deepEqual(
		measurements.map((measurement) => measurement.selector),
		[".rail", ".drawer"],
	);
	assert.deepEqual(
		measurements.map((measurement) => measurement.value),
		[224, 360],
	);
});

test("an answer that arrives in another order is matched by selector, not by position", async () => {
	const transport: Transport = {
		async measure(selectors) {
			return [...selectors].reverse().map((selector) => ({
				selector,
				present: true,
				value: `value of ${selector}`,
			}));
		},
	};
	const measurements = await Create(transport).measure([".rail", ".drawer"], Options());
	assert.deepEqual(measurements, [
		{ selector: ".rail", present: true, value: "value of .rail" },
		{ selector: ".drawer", present: true, value: "value of .drawer" },
	]);
});

test("an element the page does not have is a measurement, never a throw", async () => {
	const fixture = Answering({ ".rail": { present: false } });
	const measurements = await Create(fixture).measure([".rail"], Options());
	assert.deepEqual(measurements, [Absent(".rail")]);
});

test("an absent element cannot carry a value out of the transport", async () => {
	const transport: Transport = {
		async measure(selectors) {
			return selectors.map((selector) => ({
				selector,
				present: false,
				value: 224,
			}));
		},
	};
	const [rail] = await Create(transport).measure([".rail"], Options());
	assert.deepEqual(rail, Absent(".rail"));
});

test("a dead transport is not-ready, which is not the answer an absent element gives", async () => {
	const dead: Transport = {
		async measure() {
			throw new Error("socket closed");
		},
	};
	await assert.rejects(
		Create(dead).measure([".rail"], Options()),
		(error: unknown) =>
			error instanceof VerifyError &&
			error.code === ErrorCode.NotReady &&
			/socket closed/.test(error.detail),
	);
});

test("a transport that never answers is not-ready at the deadline, not a hang", async () => {
	const silent: Transport = {
		measure: () => new Promise<Measurement[]>(() => {}),
	};
	const started = Date.now();
	await assert.rejects(
		Create(silent).measure([".rail"], {
			timeoutMs: 25,
			signal: new AbortController().signal,
		}),
		(error: unknown) =>
			error instanceof VerifyError &&
			error.code === ErrorCode.NotReady &&
			/deadline/.test(error.message),
	);
	assert.ok(Date.now() - started < 1_000);
});

test("a signal that had already fired rejects before the page is asked", async () => {
	let asked = 0;
	const transport: Transport = {
		async measure(selectors) {
			asked += 1;
			return selectors.map((selector) => Absent(selector));
		},
	};
	await assert.rejects(
		Create(transport).measure([".rail"], {
			timeoutMs: 1_000,
			signal: AbortSignal.abort(),
		}),
		(error: unknown) =>
			error instanceof VerifyError &&
			error.code === ErrorCode.NotReady &&
			/before it started/.test(error.message),
	);
	assert.equal(asked, 0);
});

test("a signal that fires while the page is being read is not-ready, named as a cancellation", async () => {
	const controller = new AbortController();
	const silent: Transport = {
		measure: () => new Promise<Measurement[]>(() => {}),
	};
	const pending = Create(silent).measure([".rail"], {
		timeoutMs: 5_000,
		signal: controller.signal,
	});
	controller.abort();
	await assert.rejects(
		pending,
		(error: unknown) =>
			error instanceof VerifyError &&
			error.code === ErrorCode.NotReady &&
			/cancel/.test(error.message) &&
			/fired/.test(error.detail),
	);
});

test("a selector the transport did not answer for is a broken transport, not an absent element", async () => {
	const transport: Transport = {
		async measure() {
			return [{ selector: ".rail", present: true, value: 224 }];
		},
	};
	await assert.rejects(
		Create(transport).measure([".rail", ".drawer"], Options()),
		(error: unknown) =>
			error instanceof VerifyError &&
			error.code === ErrorCode.NotReady &&
			/\.drawer/.test(error.detail),
	);
});

test("a measurement that does not say whether the element was there is refused", async () => {
	const transport: Transport = {
		async measure(selectors) {
			return selectors.map((selector) => ({
				selector,
			})) as unknown as Measurement[];
		},
	};
	await assert.rejects(
		Create(transport).measure([".rail"], Options()),
		(error: unknown) =>
			error instanceof VerifyError &&
			error.code === ErrorCode.NotReady &&
			/as present/.test(error.detail),
	);
});

test("a value that is not a number, a string or null is refused rather than flattened", async () => {
	const transport: Transport = {
		async measure(selectors) {
			return selectors.map((selector) => ({
				selector,
				present: true,
				value: { width: 224 },
			})) as unknown as Measurement[];
		},
	};
	await assert.rejects(
		Create(transport).measure([".rail"], Options()),
		(error: unknown) =>
			error instanceof VerifyError &&
			error.code === ErrorCode.NotReady &&
			/number, a string or null/.test(error.detail),
	);
});

test("a transport that answers with something other than a list is refused", async () => {
	const transport: Transport = {
		measure: async () => null as unknown as Measurement[],
	};
	await assert.rejects(
		Create(transport).measure([".rail"], Options()),
		(error: unknown) =>
			error instanceof VerifyError &&
			error.code === ErrorCode.NotReady &&
			/not a measurement list/.test(error.message),
	);
});

test("a measurement of nothing asks the transport for nothing", async () => {
	const fixture = Answering({});
	assert.deepEqual(await Create(fixture).measure([], Options()), []);
	assert.deepEqual(fixture.asked, []);
});

test("a missing transport fails the call, not the construction", async () => {
	const verify = Create(undefined as unknown as Transport);
	await assert.rejects(
		verify.measure([".rail"], Options()),
		(error: unknown) =>
			error instanceof VerifyError &&
			error.code === ErrorCode.NotReady &&
			/no transport/.test(error.message),
	);
});

test("a verdict holds when nothing is absent and every invariant holds", () => {
	const verdict = UnderTest().check(
		[
			InvariantOf(
				"the rail is its designed width",
				(measurements) => measurements[0]?.value,
				224,
			),
			InvariantOf(
				"the drawer is its designed height",
				(measurements) => measurements[1]?.value,
				360,
			),
		],
		[Present(".rail", 224), Present(".drawer", 360)],
	);
	assert.equal(verdict.held, true);
	assert.deepEqual(verdict.absent, []);
	assert.deepEqual(
		verdict.evidence.map((row) => [row.label, row.held]),
		[
			["the rail is its designed width", true],
			["the drawer is its designed height", true],
		],
	);
});

test("an absent element fails the verdict even when every invariant reads a number", () => {
	const verdict = UnderTest().check(
		[InvariantOf("the rail is its designed width", () => 224, 224)],
		[Absent(".rail")],
	);
	assert.equal(verdict.held, false);
	assert.deepEqual(verdict.absent, [".rail"]);
	assert.equal(verdict.evidence[0]?.held, true);
});

test("a broken invariant names itself, and only itself, among the failures", () => {
	const verdict = UnderTest().check(
		[
			InvariantOf(
				"the rail is its designed width",
				(measurements) => measurements[0]?.value,
				224,
			),
			InvariantOf(
				"the drawer is its designed height",
				(measurements) => measurements[0]?.value,
				240,
			),
		],
		[Present(".rail", 240)],
	);
	assert.equal(verdict.held, false);
	const failed = verdict.evidence.filter((row) => !row.held);
	assert.deepEqual(
		failed.map((row) => row.label),
		["the rail is its designed width"],
	);
	assert.equal(failed[0]?.got, 240);
	assert.equal(failed[0]?.want, 224);
});

test("a read that throws is a failed invariant, not an exception out of check", () => {
	const verdict = UnderTest().check(
		[
			InvariantOf(
				"the rail's bottom edge",
				(measurements) => {
					const rail = measurements[0] as unknown as {
						box: { top: number };
					};
					return rail.box.top;
				},
				224,
			),
		],
		[Absent(".rail")],
	);
	assert.equal(verdict.held, false);
	const row = verdict.evidence[0] as Evidence;
	assert.equal(row.held, false);
	const got = row.got as { threw?: unknown };
	assert.equal(typeof got.threw, "string");
});

test("an expectation that is a function of the measurements is derived, not hard-coded", () => {
	const verdict = UnderTest().check(
		[
			{
				label: "the drawer starts where the rail ends",
				read: (measurements) => measurements[1]?.value,
				want: (measurements: Measurement[]) => measurements[0]?.value,
			},
		],
		[Present(".rail", 224), Present(".drawer", 224)],
	);
	assert.equal(verdict.held, true);
	assert.equal(verdict.evidence[0]?.want, 224);
});

test("a comparator makes a relational invariant expressible", () => {
	const relational: Invariant = {
		label: "the lane air exceeds the canvas gap",
		read: (measurements) => measurements[0]?.value,
		want: (measurements: Measurement[]) => measurements[1]?.value,
		test: (got, want) => Number(got) > Number(want),
	};
	const measurements = [Present(".lane", 24), Present(".canvas", 20)];
	assert.equal(UnderTest().check([relational], measurements).held, true);
	const inverted: Invariant = { ...relational, read: () => 12 };
	assert.equal(UnderTest().check([inverted], measurements).held, false);
});

test("a present element that read null is a value, not an absence", () => {
	const verdict = UnderTest().check(
		[InvariantOf("the seam states no colour", (m) => m[0]?.value, null)],
		[Present(".seam", null)],
	);
	assert.equal(verdict.held, true);
	assert.deepEqual(verdict.absent, []);
});

test("the absent list names each selector once, in the order it was measured", () => {
	const verdict = UnderTest().check(
		[],
		[Absent(".seam"), Present(".rail", 224), Absent(".drawer"), Absent(".seam")],
	);
	assert.deepEqual(verdict.absent, [".seam", ".drawer"]);
	assert.equal(verdict.held, false);
	assert.deepEqual(verdict.evidence, []);
});

test("a run with no invariants holds when nothing is absent, and fails when something is", () => {
	const verify = UnderTest();
	assert.equal(verify.check([], [Present(".rail", 224)]).held, true);
	assert.equal(verify.check([], [Absent(".rail")]).held, false);
});

test("check is a pure function of its arguments: the same measurements answer the same verdict", () => {
	const invariants = [
		InvariantOf(
			"the rail is its designed width",
			(measurements) => measurements[0]?.value,
			224,
		),
	];
	const measurements = [Present(".rail", 224), Absent(".drawer")];
	const verify = UnderTest();
	assert.deepEqual(
		verify.check(invariants, measurements),
		verify.check(invariants, measurements),
	);
});

test("the package's source carries no selector, length or colour of any design", async () => {
	const directory = new URL("../src/", import.meta.url);
	const files = (await readdir(directory)).filter((name) => name.endsWith(".ts"));
	assert.ok(files.length > 0);
	const design = /[0-9]+px|rgb\(|#[0-9a-f]{3,6}/;
	for (const name of files) {
		const text = await readFile(new URL(name, directory), "utf8");
		const hit = design.exec(text);
		assert.equal(hit, null, `${name} carries ${hit?.[0]}`);
	}
});

test("the error codes are the manifest's errors, and there are no others", async () => {
	const manifest = JSON.parse(
		await readFile(new URL("../module.json", import.meta.url), "utf8"),
	) as { provides: { port: string; errors: string[] } };
	assert.equal(manifest.provides.port, "agentic.verify.design");
	assert.deepEqual([...Object.values(ErrorCode)].sort(), [...manifest.provides.errors].sort());
});

test("a verdict becomes the exception the manifest names, absence before a broken invariant", () => {
	const verify = UnderTest();
	const held = verify.check(
		[InvariantOf("the rail is its designed width", () => 224, 224)],
		[Present(".rail", 224)],
	);
	assert.equal(VerifyError.from(held), null);
	const absent = VerifyError.from(verify.check([], [Absent(".rail")]));
	assert.equal(absent?.code, ErrorCode.Absent);
	assert.match(absent?.detail ?? "", /\.rail/);
	const broke = VerifyError.from(
		verify.check(
			[InvariantOf("the rail is its designed width", () => 240, 224)],
			[Present(".rail", 240)],
		),
	);
	assert.equal(broke?.code, ErrorCode.Broke);
	assert.match(broke?.detail ?? "", /rail is its designed width/);
});

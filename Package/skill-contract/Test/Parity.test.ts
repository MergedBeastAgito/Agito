/**
 * The TypeScript side of the differential run for `agentic.skill.validate`.
 *
 * These cases are not written here: they are read from
 * `scenarios/skill-contract.json`, the same file the Rust port's `Test/Parity.rs`
 * reads. What this file adds is only the assertion - the corpus, the steps, the
 * expected result and the expected ordered effect list all come from the
 * scenario file, so the two implementations cannot drift by being tested against
 * two hand-written suites.
 *
 * The corpus is a real tree: `scenarios/lib/skill.mjs` builds it in a fresh
 * temporary directory from the fixture, and the Rust side builds the same tree
 * from the same description. `effects` is what the port reported, in order, so a
 * port that found the same findings in a different order - or discovered twice
 * where the scenario says once - fails here.
 *
 * `node --test Test/*.test.ts` runs this beside `SkillContract.test.ts`. The
 * suite that existed before is kept as it is: it is the contract, this is the
 * comparison.
 */
import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import { test } from "node:test";

import { casesOf, compare, load } from "../../../scenarios/lib/load.mjs";
import { runSkill } from "../../../scenarios/lib/skill.mjs";

const Scenarios = new URL("../../../scenarios/skill-contract.json", import.meta.url);
const Contract = new URL("./SkillContract.test.ts", import.meta.url);

const File = load(Scenarios);

for (const entry of casesOf(File)) {
	test(`scenario ${entry.id}`, async () => {
		const observed = await runSkill(entry);
		const wrong = compare(observed, entry.expect);
		assert.deepEqual(
			wrong,
			[],
			`${entry.id} disagrees with the scenario file:\n  ${wrong.join("\n  ")}`,
		);
	});
}

test("every scenario names a case the TypeScript suite has", async () => {
	const source = await readFile(Contract, "utf8");
	for (const entry of File.cases) {
		assert.equal(typeof entry.typescript, "string", `${entry.id} names no TypeScript case`);
		assert.ok(
			source.includes(`test("${entry.typescript}"`),
			`${entry.id} names a TypeScript case that does not exist: ${entry.typescript}`,
		);
	}
});

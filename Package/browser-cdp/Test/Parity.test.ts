/**
 * The TypeScript side of the differential run for `agentic.browse`.
 *
 * These cases are not written here: they are read from `scenarios/browser-cdp.json`,
 * the same file the Rust port's `Test/Parity.rs` reads. What this file adds is
 * only the assertion - the fixture, the steps, the expected result and the
 * expected ordered effect list all come from the scenario file, so the two
 * implementations cannot drift by being tested against two hand-written suites.
 *
 * `node --test Test/*.test.ts` runs this beside `Browse.test.ts`. The suite that
 * existed before is kept as it is: it is the contract, this is the comparison.
 */
import assert from "node:assert/strict";
import { test } from "node:test";

import { runBrowse } from "../../../scenarios/lib/browse.mjs";
import { casesOf, compare, load } from "../../../scenarios/lib/load.mjs";

const Scenarios = new URL("../../../scenarios/browser-cdp.json", import.meta.url);
const Generated = new URL("../../../scenarios/generated/browser-cdp.timing.json", import.meta.url);

for (const file of [load(Scenarios), load(Generated)])
	for (const entry of casesOf(file)) {
		test(`scenario ${entry.id}`, async () => {
			const observed = await runBrowse(entry);
			const wrong = compare(observed, entry.expect);
			assert.deepEqual(
				wrong,
				[],
				`${entry.id} disagrees with the scenario file:\n  ${wrong.join("\n  ")}`,
			);
		});
	}

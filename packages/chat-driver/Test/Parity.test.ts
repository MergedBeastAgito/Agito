/**
 * The TypeScript side of the differential run for `agentic.chat.ask`.
 *
 * These cases are not written here: they are read from `scenarios/chat-driver.json`
 * and `scenarios/generated/chat-driver.timing.json`, the same files the Rust
 * port's `tests/parity.rs` reads. What this file adds is only the assertion - the
 * fixture, the steps, the expected result and the expected ordered effect list
 * all come from the scenario files, so the two implementations cannot drift by
 * being tested against two hand-written suites.
 *
 * `node --test Test/*.test.ts` runs this beside `Chat.test.ts`. The suite that
 * existed before is kept as it is: it is the contract, this is the comparison.
 */
import assert from "node:assert/strict";
import { test } from "node:test";

import { runChat } from "../../../scenarios/lib/chat.mjs";
import { casesOf, compare, load } from "../../../scenarios/lib/load.mjs";

const Scenarios = new URL("../../../scenarios/chat-driver.json", import.meta.url);
const Generated = new URL("../../../scenarios/generated/chat-driver.timing.json", import.meta.url);

for (const file of [load(Scenarios), load(Generated)])
	for (const entry of casesOf(file)) {
		if ((entry.skip ?? []).includes("typescript")) continue;
		test(`scenario ${entry.id}`, async () => {
			const observed = await runChat(entry);
			const wrong = compare(observed, entry.expect);
			assert.deepEqual(
				wrong,
				[],
				`${entry.id} disagrees with the scenario file:\n  ${wrong.join("\n  ")}`,
			);
		});
	}

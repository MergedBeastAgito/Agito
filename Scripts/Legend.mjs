#!/usr/bin/env node
/**
 * Every diagram carries the shared legend.
 *
 * The legend was injected once, by a script that no longer exists; this is the
 * invariant that outlives it. A page added without a legend is a page whose terms
 * are undefined for whoever reads it next, so the check fails rather than warning.
 *
 *   node Scripts/Legend.mjs
 */
import { readdirSync, readFileSync } from "node:fs";
import { join } from "node:path";

const Directory = new URL("../docs/diagram", import.meta.url).pathname;
const Marker = "legend:";
const Problems = [];
let Blocks = 0;

for (const Entry of readdirSync(Directory).sort()) {
	if (!Entry.endsWith(".md") || Entry === "Index.md") continue;
	const Text = readFileSync(join(Directory, Entry), "utf8");

	for (const Match of Text.matchAll(/```mermaid\n([\s\S]*?)```/g)) {
		Blocks++;
		if (!Match[1].includes(Marker)) Problems.push(`${Entry}: a diagram without the legend`);
	}
}

if (Blocks === 0) Problems.push("docs/diagram holds no mermaid block at all");

for (const Problem of Problems) console.log(`FAIL  ${Problem}`);
console.log(
	Problems.length === 0
		? `RESULT: ${Blocks} diagram(s), every one carries the legend`
		: `RESULT: ${Problems.length} problem(s)`,
);
process.exit(Problems.length === 0 ? 0 : 1);

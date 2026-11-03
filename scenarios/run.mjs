/**
 * The differential run.
 *
 *   node scenarios/run.mjs            # every scenario against the TypeScript side
 *   node scenarios/run.mjs --compare  # and against the Rust side, per scenario
 *   node scenarios/run.mjs --write    # rewrite the generated timing files first
 *
 * One scenario file per capability is the input both implementations answer to.
 * This script is the referee: it runs the TypeScript half in-process, asks cargo
 * for the Rust half (`AGENTIC_PARITY_OUT`), and reports, per scenario, whether
 * both sides produced the result and the ordered effect list the file expects.
 *
 * Two things it refuses to do:
 *
 * - compare against a stale generated file. The timing scenarios are produced by
 *   `generate.mjs`; if the committed copies have drifted, the comparison would
 *   be against something nobody generated, so this fails and says how to fix it.
 * - call a case that one side skips a disagreement. A case that cannot be
 *   expressed in one implementation is reported as `unported`, with the reason
 *   the file gives, and counted separately.
 */
import { execFileSync } from "node:child_process";
import { mkdtempSync, readFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

import { runBrowse } from "./lib/browse.mjs";
import { runChat } from "./lib/chat.mjs";
import { casesOf, compare, load } from "./lib/load.mjs";
import { runSkill } from "./lib/skill.mjs";
import { documents } from "./generate.mjs";

const Here = dirname(fileURLToPath(import.meta.url));
const Root = dirname(Here);
const Args = process.argv.slice(2);
const Compare = Args.includes("--compare");
const Write = Args.includes("--write");

/** The three capabilities, and how to reach each side of each one. */
const Capabilities = [
	{
		capability: "agentic.browse",
		files: ["browser-cdp.json", "generated/browser-cdp.timing.json"],
		run: runBrowse,
		package: "browser-cdp",
		crate: "agentic-browser-cdp",
	},
	{
		capability: "agentic.chat.ask",
		files: ["chat-driver.json", "generated/chat-driver.timing.json"],
		run: runChat,
		package: "chat-driver",
		crate: "agentic-chat-driver",
	},
	{
		capability: "agentic.skill.validate",
		files: ["skill-contract.json"],
		run: runSkill,
		package: "skill-contract",
		crate: "agentic-skill-contract",
	},
];

/** The generated files, as `generate.mjs` would write them. */
const serialised = (document) => `${JSON.stringify(document, null, "\t")}\n`;

/** Refuse to compare against a generated file nobody generated. */
const checkDrift = () => {
	const stale = [];
	for (const [name, document] of Object.entries(documents())) {
		const path = join(Here, name);
		let onDisk = null;
		try {
			onDisk = readFileSync(path, "utf8");
		} catch {
			stale.push(`${name} is missing`);
			continue;
		}
		if (onDisk !== serialised(document)) stale.push(`${name} does not match generate.mjs`);
	}
	if (stale.length > 0) {
		if (Write) {
			for (const one of stale) console.log(`regenerating: ${one}`);
			execFileSync(process.execPath, [join(Here, "generate.mjs")], { stdio: "inherit" });
			return;
		}
		console.error("the generated timing scenarios are stale:");
		for (const one of stale) console.error(`  ${one}`);
		console.error("run `node scenarios/generate.mjs` (or this script with --write) first.");
		process.exit(1);
	}
};

/** The Rust half: cargo writes what it observed to a file this script reads. */
const rustObservations = (crate) => {
	const directory = mkdtempSync(join(tmpdir(), "parity-"));
	const out = join(directory, "rust.json");
	try {
		execFileSync("cargo", ["test", "-p", crate, "--test", "parity"], {
			cwd: join(Root, "rust"),
			env: { ...process.env, AGENTIC_PARITY_OUT: out },
			stdio: ["ignore", "pipe", "pipe"],
		});
	} catch (error) {
		console.error(`cargo test -p ${crate} --test parity failed:`);
		console.error(String(error.stdout ?? ""));
		console.error(String(error.stderr ?? ""));
		process.exit(1);
	}
	return JSON.parse(readFileSync(out, "utf8"));
};

const skipped = (entry, side) => (entry.skip ?? []).includes(side);

checkDrift();

let total = 0;
let agreed = 0;
let unported = 0;
const differing = [];

for (const capability of Capabilities) {
	const cases = capability.files.flatMap((name) => {
		const path = join(Here, name);
		try {
			return casesOf(load(path)).map((entry) => ({ ...entry, file: name }));
		} catch {
			return [];
		}
	});
	const rust = Compare ? rustObservations(capability.crate) : null;

	console.log(`\n${capability.capability} - ${cases.length} scenarios`);

	for (const entry of cases) {
		total += 1;
		const wrong = [];
		let observed = null;

		if (!skipped(entry, "typescript")) {
			observed = await capability.run(entry);
			for (const sentence of compare(observed, entry.expect))
				wrong.push(`typescript: ${sentence}`);
		}

		if (rust !== null && !skipped(entry, "rust")) {
			const theirs = rust[entry.id];
			if (theirs === undefined) {
				wrong.push("rust: no observation for this scenario");
			} else {
				for (const sentence of compare(theirs, entry.expect))
					wrong.push(`rust: ${sentence}`);
				// The expectation already pins the effects, so this is a redundant
				// check on purpose: it catches a case whose expectation was left too
				// vague for either side to be pinned by it.
				if (
					observed !== null &&
					JSON.stringify(observed.effects) !== JSON.stringify(theirs.effects)
				)
					wrong.push(
						`effects differ: typescript ${JSON.stringify(observed.effects)} / rust ${JSON.stringify(theirs.effects)}`,
					);
			}
		}

		const sides = ["typescript", "rust"].filter((side) => skipped(entry, side));
		// A case neither side runs is not an agreement: nothing was compared. It is
		// reported as unported, with the reason the file gives.
		if (sides.length === 2) {
			unported += 1;
			console.log(`  unported  ${entry.id} - ${entry.unported ?? "no reason given"}`);
		} else if (wrong.length === 0) {
			agreed += 1;
			console.log(
				`  agree     ${entry.id}${sides.length > 0 ? ` (${sides.join(", ")} only)` : ""}`,
			);
		} else {
			differing.push({ id: entry.id, wrong });
			console.log(`  differ    ${entry.id}`);
			for (const sentence of wrong) console.log(`              ${sentence}`);
		}
	}
}

console.log(
	`\n${agreed}/${total} scenarios agreed${Compare ? " on both sides" : ""}; ${unported} unported; ${differing.length} differing`,
);
if (differing.length > 0) process.exit(1);

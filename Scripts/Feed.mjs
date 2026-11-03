#!/usr/bin/env node
/**
 * The feed tools, tested where they can be: `bundle.mjs` is pure filesystem work,
 * so it belongs in the gate. `attach.mjs` needs a browser and stays an ad-hoc run.
 *
 * A bundler that is not deterministic is a bundler whose batches cannot be
 * compared between runs, which is the property this exists to hold.
 */
import { mkdtempSync, mkdirSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import { execFileSync } from "node:child_process";
import { tmpdir } from "node:os";
import { join } from "node:path";

const Bundle = new URL("../.hermes/tools/bundle.mjs", import.meta.url).pathname;
const Work = mkdtempSync(join(tmpdir(), "agentic-feed-"));
const Problems = [];

const Check = (Label, Got, Want) => {
	if (Got !== Want) Problems.push(`${Label}: got ${Got}, want ${Want}`);
};
const Read = (Path) => readFileSync(join(Work, Path), "utf8");
const Manifest = (Name) => JSON.parse(Read(`${Name}/manifest.json`));

try {
	mkdirSync(join(Work, "tree/Source/Fn"), { recursive: true });
	mkdirSync(join(Work, "tree/target"), { recursive: true });
	writeFileSync(join(Work, "tree/Source/Fn/A.rs"), "alpha\n");
	writeFileSync(join(Work, "tree/Source/Fn/B.rs"), "beta\n");
	writeFileSync(join(Work, "tree/target/Skip.rs"), "skip\n");
	writeFileSync(join(Work, "tree/Source/Big.rs"), "x".repeat(1500));

	const Run = (Out) =>
		execFileSync("node", [Bundle, join(Work, "tree"), "--out", join(Work, Out), "--max-files", "1", "--max-bytes", "1000", "--include", ".rs", "--exclude", "target"], { stdio: "pipe" });

	Run("first");
	Run("second");

	const One = Manifest("first");
	Check("files kept", One.totalFiles, 2);
	Check("excluded directory", Read("first/tree.txt").includes("target/Skip.rs"), false);
	Check("oversize recorded", One.leftOut[0]?.path, "Source/Big.rs");
	Check("one file per batch", One.batches.every((Batch) => Batch.files.length === 1), true);
	Check("file delimiter", Read("first/batch-01.txt").startsWith("--- FILE: Source/Fn/A.rs ---"), true);
	Check("deterministic order", Read("first/batch-01.txt") === Read("second/batch-01.txt"), true);
} finally {
	rmSync(Work, { recursive: true, force: true });
}

for (const Problem of Problems) console.log(`FAIL  ${Problem}`);
console.log(Problems.length === 0 ? "RESULT: the bundler is deterministic and splits as asked" : `RESULT: ${Problems.length} problem(s)`);
process.exit(Problems.length === 0 ? 0 : 1);

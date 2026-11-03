#!/usr/bin/env node
/**
 * Fail when the arrows reverse.
 *
 * A capability may import its own files or another capability; an adapter may
 * import the capability whose port it implements; the runtime may import any.
 * This walks every package's source and refuses an import that crosses a
 * boundary the manifest does not declare.
 *
 *   node Scripts/Boundaries.mjs
 */
import { readdirSync, readFileSync, statSync } from "node:fs";
import { join } from "node:path";

const Root = process.cwd();
const Problems = [];

const Walk = (Directory) => {
	const Found = [];
	for (const Entry of readdirSync(Directory)) {
		const Path = join(Directory, Entry);
		if (Entry === "node_modules" || Entry === "Target") continue;
		if (statSync(Path).isDirectory()) Found.push(...Walk(Path));
		else if (/\.(ts|mjs|js)$/.test(Entry)) Found.push(Path);
	}
	return Found;
};

const Kinds = new Map();
for (const Entry of readdirSync(join(Root, "packages"))) {
	try {
		const Manifest = JSON.parse(
			readFileSync(join(Root, "packages", Entry, "module.json"), "utf8"),
		);
		Kinds.set(Manifest.name, Manifest.kind);
	} catch {
		Problems.push("packages/" + Entry + ": no readable module.json");
	}
}

for (const [Name, Kind] of Kinds) {
	const Directory = join(Root, "packages", Name.replace("@agentic/", ""));
	for (const File of Walk(Directory)) {
		const Text = readFileSync(File, "utf8");
		for (const Match of Text.matchAll(/from\s+["'](@agentic\/[a-z-]+)["']/g)) {
			const Target = Match[1];
			const TargetKind = Kinds.get(Target);
			if (Target === Name) continue;
			const Allowed =
				Kind === "runtime" ||
				(Kind === "capability" && TargetKind === "capability") ||
				(Kind === "adapter" && TargetKind === "capability");
			if (!Allowed)
				Problems.push(
					File.replace(Root + "/", "") +
						": " +
						Kind +
						" imports " +
						Target +
						" (" +
						TargetKind +
						")",
				);
		}
	}
}

for (const Problem of Problems) console.log("FAIL  " + Problem);
console.log(
	Problems.length === 0
		? "RESULT: every import points inward"
		: "RESULT: " + Problems.length + " boundary violation(s)",
);
process.exit(Problems.length === 0 ? 0 : 1);

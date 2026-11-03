#!/usr/bin/env node
/**
 * Every diagram is a typed source file with a rendered sibling.
 *
 * The convention this enforces: a `.mmd` under `docs/diagram/` opens with frontmatter, names the
 * `line9:` theme, annotates at least one node with the shared vocabulary, and has an `.svg` sibling
 * that was actually rendered. A page may keep a fenced block only where line9 cannot export it, and
 * says so. A prose legend inside a drawing is the thing the annotations replaced, so it fails here.
 *
 *   node Scripts/Legend.mjs
 */
import { readdirSync, readFileSync, statSync } from "node:fs";
import { join, relative } from "node:path";
import { fileURLToPath } from "node:url";

const Root = fileURLToPath(new URL("../docs/diagram", import.meta.url));
const Classes = [":::host", ":::hook", ":::store", ":::fail"];
const ProseLegend = "legend:";
const Unexportable = "cannot be exported";
const MinimumSvgBytes = 1000;

const Problems = [];
let Diagrams = 0;
let Pages = 0;

function Files(Directory) {
	const Found = [];
	for (const Entry of readdirSync(Directory, { withFileTypes: true })) {
		const Path = join(Directory, Entry.name);
		if (Entry.isDirectory()) Found.push(...Files(Path));
		else Found.push(Path);
	}
	return Found.sort();
}

function Rendered(Path) {
	return Path.replace(/\.mmd$/, ".svg");
}

function CheckDiagram(Name, Path, Text) {
	if (!/^---\r?\n/.test(Text)) {
		Problems.push(`${Name}: does not open with frontmatter`);
		return;
	}
	const End = Text.indexOf("\n---", 3);
	const Frontmatter = End === -1 ? "" : Text.slice(0, End);
	if (!/^line9:\s*$/m.test(Frontmatter))
		Problems.push(`${Name}: frontmatter names no line9 block`);
	if (!/^\s+theme:\s*\S+/m.test(Frontmatter))
		Problems.push(`${Name}: frontmatter names no theme`);
	if (!Classes.some((Class) => Text.includes(Class))) {
		Problems.push(`${Name}: carries no class annotation`);
	}
	if (/\bclassDef\b/.test(Text))
		Problems.push(`${Name}: declares a classDef; the theme is the CLI's`);
	if (/\bnote\s+(left|right|over)\b/.test(Text)) {
		Problems.push(`${Name}: a note inside a drawing is a prose legend`);
	}
	if (/subgraph\s+Legend\b/i.test(Text))
		Problems.push(`${Name}: a Legend subgraph is a prose legend`);

	const Svg = Rendered(Path);
	try {
		const Size = statSync(Svg).size;
		if (Size <= MinimumSvgBytes)
			Problems.push(`${Name}: its .svg is ${Size} bytes, not rendered`);
	} catch {
		Problems.push(`${Name}: no .svg sibling`);
	}
}

function CheckRender(Name, Path) {
	try {
		statSync(Path.replace(/\.svg$/, ".mmd"));
	} catch {
		Problems.push(`${Name}: an .svg with no .mmd beside it`);
		return;
	}
	const Size = statSync(Path).size;
	if (Size <= MinimumSvgBytes) Problems.push(`${Name}: ${Size} bytes, which is not a render`);
}

function CheckPage(Name, Text) {
	if (Text.includes(ProseLegend)) Problems.push(`${Name}: carries a prose legend`);
	if (/```mermaid/.test(Text) && !Text.includes(Unexportable)) {
		Problems.push(`${Name}: keeps a fenced block without saying why it cannot be exported`);
	}
}

for (const Path of Files(Root)) {
	const Name = relative(Root, Path).split("\\").join("/");

	if (Path.endsWith(".svg")) {
		CheckRender(Name, Path);
		continue;
	}

	const Text = readFileSync(Path, "utf8");
	if (Path.endsWith(".mmd")) {
		Diagrams++;
		CheckDiagram(Name, Path, Text);
	} else if (Path.endsWith(".md")) {
		Pages++;
		CheckPage(Name, Text);
	}
}

if (Diagrams === 0) Problems.push("docs/diagram holds no .mmd at all");

for (const Problem of Problems) console.log(`FAIL  ${Problem}`);
console.log(
	Problems.length === 0
		? `RESULT: ${Diagrams} diagram(s) and ${Pages} page(s), every diagram typed and rendered`
		: `RESULT: ${Problems.length} problem(s)`,
);
process.exit(Problems.length === 0 ? 0 : 1);

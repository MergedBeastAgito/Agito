/**
 * Package a tree of files into batches a chat product will accept.
 *
 * A chat composer takes files through an `<input type="file" multiple>` that is
 * usually hidden, and caps how many it will take at once. So the feed has two
 * halves: this bundler, which turns a directory into deterministic batches, and
 * an attacher that sets them on the input.
 *
 *   node bundle.mjs <root> --out <dir> [--max-files 30] [--max-bytes 200000]
 *                              [--include "*.rs,*.md"] [--exclude "target"]
 *
 * Writes into <dir>: tree.txt (the shape), batch-NN.txt (the payload, each file
 * preceded by a `--- FILE: <relative path> ---` line), manifest.json (what went
 * where, and what was left out and why).
 *
 * Everything it writes is relative to a repository. The manifest records the
 * root as a path from the repository root, and the payload is scrubbed of the
 * two absolute roots a document is written against - the repository and the
 * home directory - because both the manifest and the batches are committed.
 * A machine path that survives the scrub is reported and fails the run.
 */
import {
	existsSync,
	mkdirSync,
	readFileSync,
	readdirSync,
	statSync,
	writeFileSync,
} from "node:fs";
import { homedir } from "node:os";
import { dirname, join, relative, resolve, sep } from "node:path";

const Args = process.argv.slice(2);
const Root = Args[0];
const At = (Name, Fallback) => {
	const Index = Args.indexOf(Name);
	return Index === -1 ? Fallback : Args[Index + 1];
};

if (!Root) {
	console.error(
		"usage: node bundle.mjs <root> --out <dir> [--max-files N] [--max-bytes N] [--include globs] [--exclude names]",
	);
	process.exit(2);
}

const Out = At("--out", join(process.cwd(), "bundle"));
const MaxFiles = Number(At("--max-files", 30));
const MaxBytes = Number(At("--max-bytes", 200000));
const Include = At("--include", ".rs,.ts,.mjs,.md,.json,.toml")
	.split(",")
	.map((Entry) => Entry.trim())
	.filter(Boolean);
const Exclude = At("--exclude", "target,node_modules,.git,dist,bundle")
	.split(",")
	.map((Entry) => Entry.trim())
	.filter(Boolean);

// The nearest directory holding a `.git`, walking up from `Path`. A bundle of a
// tree outside any repository is still named relative to the working directory,
// so the answer `null` only changes the base, never the fact of a relative path.
const Up = (Path) => {
	let Directory = resolve(Path);
	for (;;) {
		if (existsSync(join(Directory, ".git"))) return Directory;
		const Parent = dirname(Directory);
		if (Parent === Directory) return null;
		Directory = Parent;
	}
};

const Repository = Up(Root);
const RootName = (() => {
	const Named = relative(Repository ?? process.cwd(), resolve(Root));
	return Named === "" ? "." : Named.split(sep).join("/");
})();

// Longest first, so a root nested inside another root is replaced by its own
// name rather than by the outer root's relative form. Duplicates collapse
// because the repository root and the bundle root are often the same directory.
const Roots = [
	[resolve(Root), ""],
	[Repository, ""],
	[homedir(), "~"],
]
	.filter(([From], Index, All) => From && All.findIndex(([Other]) => Other === From) === Index)
	.sort(([Left], [Right]) => Right.length - Left.length);

// A document quotes the files it was written against, and a card that spells a
// checkout path publishes the machine through the payload instead of through the
// manifest. Both roots are replaced with their relative form: `Source/Library.rs`
// for a repository path, `~/…` for a home path.
const Scrub = (Text) =>
	Roots.reduce(
		(Clean, [From, To]) => Clean.split(From + sep).join(To === "" ? "" : To + "/"),
		Text,
	);

// The shapes that name a machine, assembled from pieces so this file cannot be
// the first hit its own check reports. A survivor is a source file that must be
// fixed, not a batch that must be edited.
const Machine = new RegExp(
	["/" + "Volumes/", "/" + "Users/[a-z]", "/" + "private/(?:tmp|var)/"].join("|"),
);
const Leaks = [];

const Wanted = (Name) => Include.some((Extension) => Name.endsWith(Extension));
// Case-insensitive: a Rust workspace puts its build in `Target/` and the exclude
// list is written by hand as `target`, and on a case-sensitive volume those are
// two different directories. Three files of build scratch ended up in a batch
// because of exactly that.
const Skipped = (Path) => {
	const Parts = Path.split(sep).map((Part) => Part.toLowerCase());
	return Exclude.some((Name) => Parts.includes(Name.toLowerCase()));
};

const Files = [];
const Left = [];

const Walk = (Directory) => {
	for (const Entry of readdirSync(Directory, { withFileTypes: true }).sort((Left, Right) =>
		Left.name.localeCompare(Right.name),
	)) {
		const Path = join(Directory, Entry.name);
		if (Entry.isDirectory()) {
			if (Skipped(Path)) continue;
			Walk(Path);
			continue;
		}
		if (!Entry.isFile()) continue;
		if (!Wanted(Entry.name)) continue;
		const Size = statSync(Path).size;
		if (Size > MaxBytes) {
			Left.push({
				path: relative(Root, Path),
				bytes: Size,
				reason: "larger than --max-bytes",
			});
			continue;
		}
		Files.push({ path: Path, relative: relative(Root, Path), bytes: Size });
	}
};

Walk(resolve(Root));

// Deterministic order: by path, so two runs over the same tree produce the same
// batches and a reader can diff batch 3 between runs.
Files.sort((Left, Right) => Left.relative.localeCompare(Right.relative));

const Batches = [];
let Current = { files: [], bytes: 0 };

for (const File of Files) {
	const Would = Current.bytes + File.bytes;
	if (Current.files.length >= MaxFiles || (Current.files.length > 0 && Would > MaxBytes)) {
		Batches.push(Current);
		Current = { files: [], bytes: 0 };
	}
	Current.files.push(File);
	Current.bytes += File.bytes;
}
if (Current.files.length > 0) Batches.push(Current);

mkdirSync(Out, { recursive: true });

const Tree = Files.map((File) => `${File.relative}  (${File.bytes} bytes)`).join("\n");
writeFileSync(join(Out, "tree.txt"), Tree + "\n");

const Written = Batches.map((Batch, Index) => {
	const Name = `batch-${String(Index + 1).padStart(2, "0")}.txt`;
	const Body = Batch.files
		.map((File) => {
			const Text = Scrub(readFileSync(File.path, "utf8").replace(/\n+$/, ""));
			for (const Hit of Text.match(Machine) ?? [])
				Leaks.push({ batch: Name, file: File.relative, path: Hit });
			return `--- FILE: ${File.relative} ---\n${Text}\n`;
		})
		.join("\n");
	writeFileSync(join(Out, Name), Body);
	return { name: Name, files: Batch.files.map((File) => File.relative), bytes: Body.length };
});

const Manifest = {
	root: RootName,
	generated: new Date().toISOString(),
	maxFiles: MaxFiles,
	maxBytes: MaxBytes,
	include: Include,
	exclude: Exclude,
	totalFiles: Files.length,
	totalBytes: Files.reduce((Sum, File) => Sum + File.bytes, 0),
	batches: Written,
	leftOut: Left,
};

writeFileSync(join(Out, "manifest.json"), JSON.stringify(Manifest, null, "\t") + "\n");

console.log(
	`files: ${Manifest.totalFiles}  bytes: ${Manifest.totalBytes}  batches: ${Written.length}`,
);
for (const Batch of Written)
	console.log(`  ${Batch.name}  ${Batch.files.length} file(s)  ${Batch.bytes} bytes`);
if (Left.length > 0) console.log(`left out: ${Left.length} (see manifest.json)`);
if (Leaks.length > 0) {
	console.error(
		`${Leaks.length} absolute machine path(s) survived the scrub - fix the source file, not the batch:`,
	);
	for (const Leak of Leaks) console.error(`  ${Leak.batch}  ${Leak.file}  ${Leak.path}`);
	process.exitCode = 1;
}

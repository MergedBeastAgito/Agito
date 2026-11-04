/**
 * The mutation suite: five deliberate defects, each of which both implementations
 * must reject.
 *
 *   node scenarios/mutate.mjs --check
 *   node scenarios/mutate.mjs --check --only threshold
 *
 * A green suite proves that the assertions ran, not that they bite. So this
 * applies one defect at a time to the real source - the TypeScript file and the
 * Rust file, in the same mutation - runs both suites, and insists that each one
 * fails. A mutation that either side survives is a hole in that side's tests, and
 * this script exits non-zero naming it.
 *
 * Every file is snapshotted by content hash before it is touched and restored
 * from that snapshot afterwards, in a `finally`, so an interrupted run cannot
 * leave a mutated tree behind; the restoration is verified by hash and reported
 * if it did not take.
 */
import { execFileSync } from "node:child_process";
import { createHash } from "node:crypto";
import { readFileSync, readdirSync, writeFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const Here = dirname(fileURLToPath(import.meta.url));
const Root = dirname(Here);
const Args = process.argv.slice(2);
const Only = Args.includes("--only") ? Args[Args.indexOf("--only") + 1] : null;

const hash = (text) => createHash("sha256").update(text, "utf8").digest("hex");
const at = (path) => join(Root, path);

/**
 * Line endings are not part of a mutation.
 *
 * This repository's checkouts do not agree on them - `core.autocrlf` is on, some
 * files are written back with CRLF by tooling that sweeps the tree, and a
 * mutation that matched nothing because of one byte per line would look like a
 * test that passed. So a file's text is compared with LF, and written back in
 * whichever ending it arrived with. The restore is still byte-exact: it writes
 * the snapshot, not a normalised version of it.
 */
const toLf = (text) => text.replace(/\r\n/g, "\n");
const endingOf = (text) => (text.includes("\r\n") ? "\r\n" : "\n");
const inEnding = (text, ending) => (ending === "\r\n" ? text.replace(/\n/g, "\r\n") : text);

/**
 * The five mutations.
 *
 * Each one is the defect named in the card, written the smallest way it can be
 * written in each language. `find` must occur exactly once in the file: a
 * mutation that matched nothing, or matched twice, is a stale mutation rather
 * than a passing test, so it is refused rather than skipped.
 */
const Mutations = [
	{
		name: "attached-page-closed",
		what: "closing an attached page on dispose",
		package: "browser-cdp",
		crate: "agentic-browser-cdp",
		edits: [
			{
				file: "Package/browser-cdp/Source/Session.ts",
				find: '\t\tif (ownership === "launched") await transport.close();\n\t\telse await transport.detach();',
				replace: "\t\tawait transport.close();",
			},
			{
				file: "Crate/BrowserCDP/Source/Struct/Session.rs",
				find: "\t\tlet Work = match Ownership {\n\t\t\tOwnership::Launched => Transport.Close(),\n\t\t\tOwnership::Attached => Transport.Detach(),\n\t\t};",
				replace: "\t\tlet Work = Transport.Close();",
			},
		],
	},
	{
		name: "model-after-send",
		what: "checking the model after the send instead of before",
		package: "chat-driver",
		crate: "agentic-chat-driver",
		edits: [
			{
				file: "Package/chat-driver/Source/Driver.ts",
				find: '			// The model is a precondition, and it is compared before the send.\n			const label = await probe<unknown>("model", ReadModel(selectors), target);\n\t\t\tif (normalise(label) !== normalise(question.model))\n\t\t\t\tthrow ChatError.wrongModel(\n\t\t\t\t\t"the page is set to a different model",\n\t\t\t\t\t`wanted ${JSON.stringify(question.model)}, the page reads ${describe(label)}`,\n\t\t\t\t);\n\n',
				replace: "",
			},
			{
				file: "Package/chat-driver/Source/Driver.ts",
				find: "\t\t\t// Completion is stable text: unchanged for three consecutive reads.",
				replace:
					'			const label = await probe<unknown>("model", ReadModel(selectors), target);\n\t\t\tif (normalise(label) !== normalise(question.model))\n\t\t\t\tthrow ChatError.wrongModel(\n\t\t\t\t\t"the page is set to a different model",\n\t\t\t\t\t`wanted ${JSON.stringify(question.model)}, the page reads ${describe(label)}`,\n\t\t\t\t);\n\n\t\t\t// Completion is stable text: unchanged for three consecutive reads.',
			},
			{
				file: "Crate/ChatDriver/Source/Struct/Driver.rs",
				find: '\t\t// The model is a precondition, and it is compared before the send.\n\t\tlet Label = self.Probe("model", ReadModel(Selectors), Target, Call, Budget).await?;\n\t\tlet Wanted = Value::String(Question.Model.clone());\n\t\tif Normalise(Some(&Label)) != Normalise(Some(&Wanted)) {\n\t\t\treturn Err(Error::WrongModel(\n\t\t\t\t"the page is set to a different model",\n\t\t\t\tformat!(\n\t\t\t\t\t"wanted {}, the page reads {}",\n\t\t\t\t\tserde_json::to_string(&Question.Model).unwrap_or_else(|_| format!("{:?}", Question.Model)),\n\t\t\t\t\tDescribe(Some(&Label))\n\t\t\t\t),\n\t\t\t));\n\t\t}\n\n',
				replace: "",
			},
			{
				file: "Crate/ChatDriver/Source/Struct/Driver.rs",
				find: "\t\t// Completion is stable text: unchanged for three consecutive reads.",
				replace:
					'\t\tlet Label = self.Probe("model", ReadModel(Selectors), Target, Call, Budget).await?;\n\t\tlet Wanted = Value::String(Question.Model.clone());\n\t\tif Normalise(Some(&Label)) != Normalise(Some(&Wanted)) {\n\t\t\treturn Err(Error::WrongModel(\n\t\t\t\t"the page is set to a different model",\n\t\t\t\tformat!(\n\t\t\t\t\t"wanted {}, the page reads {}",\n\t\t\t\t\tserde_json::to_string(&Question.Model).unwrap_or_else(|_| format!("{:?}", Question.Model)),\n\t\t\t\t\tDescribe(Some(&Label))\n\t\t\t\t),\n\t\t\t));\n\t\t}\n\n\t\t// Completion is stable text: unchanged for three consecutive reads.',
			},
		],
	},
	{
		name: "partial-text-on-deadline",
		what: "returning partial text on deadline expiry",
		package: "chat-driver",
		crate: "agentic-chat-driver",
		edits: [
			{
				file: "Package/chat-driver/Source/Driver.ts",
				find: "\t\t\t\tif (remaining() <= 0)\n\t\t\t\t\tthrow ChatError.notReady(\n\t\t\t\t\t\t`the answer did not settle within the ${budget}ms deadline`,\n\t\t\t\t\t\t`the last read was ${describe(previous)}`,\n\t\t\t\t\t);",
				replace:
					'\t\t\t\tif (remaining() <= 0) {\n\t\t\t\t\ttext = previous ?? "";\n\t\t\t\t\tbreak;\n\t\t\t\t}',
			},
			{
				file: "Crate/ChatDriver/Source/Struct/Driver.rs",
				find: '\t\t\tif Call.Expired(self.Pace.as_ref()) {\n\t\t\t\treturn Err(Error::NotReady(\n\t\t\t\t\tformat!("the answer did not settle within the {}ms deadline", Budget.as_millis()),\n\t\t\t\t\tformat!("the last read was {}", DescribeText(Progress.Text())),\n\t\t\t\t));\n\t\t\t}',
				replace:
					"\t\t\tif Call.Expired(self.Pace.as_ref()) {\n\t\t\t\tbreak Progress.Text().unwrap_or_default().to_string();\n\t\t\t}",
			},
		],
	},
	{
		name: "eof-as-timeout",
		what: "classifying a transport EOF as a timeout",
		package: "browser-cdp",
		crate: "agentic-browser-cdp",
		edits: [
			{
				file: "Package/browser-cdp/Source/Session.ts",
				find: '\t\t\t? BrowseError.unavailable(\n\t\t\t\t\t"the transport closed while the page was being asked",',
				replace:
					'\t\t\t? BrowseError.timeout(\n\t\t\t\t\t"the transport closed while the page was being asked",',
			},
			{
				file: "Crate/BrowserCDP/Source/Fn/Classify.rs",
				find: '		_ if Closed => {\n			Error::Unavailable("the transport closed while the page was being asked", Why.Detail().to_string())\n		},',
				replace:
					'		_ if Closed => {\n			Error::Timeout("the transport closed while the page was being asked", Why.Detail().to_string())\n		},',
			},
		],
	},
	{
		name: "threshold-one-read",
		what: "changing the stability threshold from three reads to one",
		package: "chat-driver",
		crate: "agentic-chat-driver",
		edits: [
			{
				file: "Package/chat-driver/Source/Driver.ts",
				find: "const StableReads = 3;",
				replace: "const StableReads = 1;",
			},
			{
				file: "Crate/ChatDriver/Source/Struct/Settlement.rs",
				find: "pub const StableReads:u8 = 3;",
				replace: "pub const StableReads:u8 = 1;",
			},
		],
	},
];

/** The TypeScript suite of one package: every `Test/*.test.ts`, no shell glob. */
const typescriptSuite = (name) => {
	const directory = at(join("Package", name));
	const files = readdirSync(join(directory, "Test"))
		.filter((entry) => entry.endsWith(".test.ts"))
		.map((entry) => join("Test", entry));
	return { command: process.execPath, args: ["--test", ...files], cwd: directory };
};

/** The Rust suite of one crate: its own test target and its parity target. */
const rustSuite = (crate) => ({ command: "cargo", args: ["test", "-p", crate], cwd: Root });

/** Run one suite, and answer whether it failed - and whether it failed to build. */
const failed = (suite) => {
	try {
		execFileSync(suite.command, suite.args, {
			cwd: suite.cwd,
			stdio: ["ignore", "pipe", "pipe"],
			env: { ...process.env, RUST_BACKTRACE: "0" },
		});
		return { rejected: false, detail: "", compiled: true };
	} catch (error) {
		const output = `${error.stdout ?? ""}${error.stderr ?? ""}`;
		// A mutation that does not compile proves nothing about the assertions: it
		// is reported separately and counted as a hole rather than as a catch.
		const compiled =
			!/could not compile|error\[E\d+\]|SyntaxError|Unexpected token|ERR_MODULE_NOT_FOUND/.test(
				output,
			);
		const line = (pattern) =>
			output
				.split("\n")
				.map((one) => one.trim())
				.find((one) => pattern.test(one));
		const detail =
			line(/^not ok \d+ - /) ??
			line(/^---- .* stdout ----$/) ??
			line(/test result: FAILED/) ??
			line(/AssertionError|panicked at/);
		return { rejected: true, detail: detail ?? "", compiled };
	}
};

/** Apply one mutation, run both suites, restore, and answer what happened. */
const check = (mutation) => {
	// `originals` is what the file was; `current` is what it will be written as.
	// Keeping them apart is the whole safety of this harness: a snapshot taken
	// after the edit is a snapshot of the mutation, and restoring it verifies a
	// mutated tree against itself.
	const originals = new Map();
	const current = new Map();
	for (const edit of mutation.edits) {
		const path = at(edit.file);
		if (!originals.has(path)) {
			const text = readFileSync(path, "utf8");
			originals.set(path, text);
			current.set(path, text);
		}
		const before = current.get(path);
		const occurrences = toLf(before).split(edit.find).length - 1;
		if (occurrences !== 1)
			throw new Error(
				`${mutation.name}: ${edit.file} contains ${occurrences} copies of the text to mutate, expected exactly 1`,
			);
		current.set(
			path,
			inEnding(toLf(before).replace(edit.find, edit.replace), endingOf(before)),
		);
	}

	try {
		for (const [path, text] of current) writeFileSync(path, text, "utf8");
		const typescript = failed(typescriptSuite(mutation.package));
		const rust = failed(rustSuite(mutation.crate));
		return { typescript, rust };
	} finally {
		for (const [path, text] of originals) {
			writeFileSync(path, text, "utf8");
			if (hash(readFileSync(path, "utf8")) !== hash(text))
				throw new Error(`${mutation.name}: ${path} was not restored`);
		}
	}
};

const chosen = Mutations.filter((mutation) => Only === null || mutation.name === Only);
if (chosen.length === 0) {
	console.error(`no mutation named ${Only}`);
	process.exit(1);
}

let survived = 0;
// A run-level audit, independent of the per-mutation restore: every file this run
// touched is hashed before the first mutation and again at the end.
const Audit = new Map();
const remember = (mutation) => {
	for (const edit of mutation.edits) {
		const path = at(edit.file);
		if (!Audit.has(path)) Audit.set(path, hash(readFileSync(path, "utf8")));
	}
};
console.log("mutation                     typescript  rust");
console.log("-".repeat(60));
for (const mutation of chosen) {
	remember(mutation);
	const { typescript, rust } = check(mutation);
	const mark = (result) =>
		!result.rejected ? "SURVIVED" : result.compiled ? "rejected" : "NO COMPILE";
	if (!typescript.rejected || !rust.rejected || !typescript.compiled || !rust.compiled)
		survived += 1;
	console.log(`${mutation.name.padEnd(28)} ${mark(typescript).padEnd(11)} ${mark(rust)}`);
	console.log(`  ${mutation.what}`);
	if (typescript.rejected) console.log(`  typescript: ${typescript.detail}`);
	if (rust.rejected) console.log(`  rust:       ${rust.detail}`);
}
console.log("-".repeat(60));
const unrestored = [...Audit].filter(
	([path, digest]) => hash(readFileSync(path, "utf8")) !== digest,
);
for (const [path] of unrestored) console.log(`UNRESTORED ${path}`);
console.log(
	`${chosen.length - survived}/${chosen.length} mutations rejected by both suites` +
		(survived > 0 ? `; ${survived} survived` : "") +
		`; ${Audit.size - unrestored.length}/${Audit.size} files restored byte-for-byte`,
);
if (survived > 0 || unrestored.length > 0) process.exit(1);

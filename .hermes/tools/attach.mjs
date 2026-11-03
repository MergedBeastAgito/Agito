/**
 * Attach files to a chat composer through its own file input.
 *
 * The input is hidden and the page's handler is a React `onChange`, so a click
 * cannot reach it: `DOM.setFileInputFiles` is the only path that sets the files
 * *and* lets the page react to them. It takes node ids, which is why this walks
 * the document rather than evaluating JavaScript.
 *
 *   node attach.mjs <ws-url> <file> [<file> ...]
 *   node attach.mjs <ws-url> --batch <bundle-dir>/batch-03.txt
 *
 * Exit 0 when the composer shows at least as many attachments as files given;
 * exit 1 when it accepted fewer; exit 2 when it could not run.
 */
import { readFileSync } from "node:fs";
import { dirname, join, resolve } from "node:path";

const Args = process.argv.slice(2);
const WsUrl = Args[0];

if (!WsUrl) {
	console.error("usage: node attach.mjs <ws-url> <file...> | --batch <batch.txt>");
	process.exit(2);
}

// A batch file carries several files as one text blob, so attaching it is one
// file, not many. `--batch` names the batch and attaches it as itself.
let Files;
if (Args[1] === "--batch") {
	const Batch = resolve(Args[2]);
	Files = [Batch];
} else {
	Files = Args.slice(1).map((Path) => resolve(Path));
}

if (Files.length === 0) {
	console.error("blocker: no files named");
	process.exit(2);
}

process.on("unhandledRejection", (Error) => {
	console.error(`blocker: ${Error?.message ?? Error}`);
	process.exit(2);
});

const sleep = (Ms) => new Promise((Done) => setTimeout(Done, Ms));
let Id = 0;
const Pending = new Map();
const Socket = new WebSocket(WsUrl);

try {
	await new Promise((Done, Fail) => {
		Socket.onopen = Done;
		Socket.onerror = (Event) => Fail(new Error(`cannot connect: ${Event.message ?? "error"}`));
	});
} catch (Error) {
	console.error(`blocker: ${Error.message}`);
	process.exit(2);
}

Socket.onmessage = (Event) => {
	const Message = JSON.parse(Event.data);
	const Slot = Pending.get(Message.id);
	if (Slot === undefined) return;
	Pending.delete(Message.id);
	if (Message.error) Slot.Fail(new Error(`${Message.error.message} (${Message.error.code})`));
	else Slot.Done(Message.result);
};

const Send = (Method, Params = {}) =>
	new Promise((Done, Fail) => {
		const Key = ++Id;
		Pending.set(Key, { Done, Fail });
		Socket.send(JSON.stringify({ id: Key, method: Method, params: Params }));
	});

await Send("DOM.enable");

const FindInput = async () => {
	const Document = await Send("DOM.getDocument", { depth: -1 });
	const Found = await Send("DOM.querySelector", {
		nodeId: Document.root.nodeId,
		selector: 'input[type="file"]',
	});
	return Found.nodeId || 0;
};

let InputNode = await FindInput();

// Some composers create the input only when their attach menu is opened. The menu
// is not the mechanism - it is a wrapper that ends in `input.click()` - but a
// product that builds the input lazily needs the wrapper opened first.
if (InputNode === 0) {
	const Opened = await Send("Runtime.evaluate", {
		expression: `(() => {
			const Add = [...document.querySelectorAll("button")].find((B) => (B.getAttribute("aria-label") || "") === "Add");
			if (Add !== undefined) { Add.click(); return "add"; }
			return "none";
		})()`,
		returnByValue: true,
	});
	if (Opened.result?.value === "add") {
		await sleep(600);
		await Send("Runtime.evaluate", {
			expression: `(() => {
				const Option = document.querySelector('[id*="add-files"], [id*="file-attachments"]');
				if (Option !== null) { Option.click(); return "option"; }
				return "none";
			})()`,
			returnByValue: true,
		});
		await sleep(600);
		InputNode = await FindInput();
	}
}

if (InputNode === 0) {
	console.error("blocker: no file input in this page, even after opening the attach menu");
	process.exit(2);
}

const Attributes = await Send("DOM.getAttributes", { nodeId: InputNode });
const Pairs = Attributes.attributes ?? [];
const Accept = Pairs[Pairs.indexOf("accept") + 1] ?? "";
const Multiple = Pairs.includes("multiple");

if (!Multiple && Files.length > 1) {
	console.error("blocker: the input accepts one file; attach them one at a time");
	process.exit(2);
}

const Refused = Files.filter((Path) => {
	const Extension = Path.slice(Path.lastIndexOf("."));
	return Accept.length > 0 && !Accept.split(",").includes(Extension);
});
if (Refused.length > 0) {
	console.error(
		`blocker: the input does not accept ${Refused.join(", ")}; allowed: ${Accept.split(",").slice(0, 12).join(",")}…`,
	);
	process.exit(2);
}

await Send("DOM.setFileInputFiles", { nodeId: InputNode, files: Files });
await sleep(1500);

// What the composer shows is the only proof: the page may reject a file silently.
const Count = await Send("Runtime.evaluate", {
	expression: `(() => {
		const Markers = [...document.querySelectorAll("[class*=attachment], [data-testid*=attachment], [class*=file-chip], [class*=fileChip]")];
		const Named = ${JSON.stringify(Files.map((Path) => Path.split("/").pop()))};
		const Text = document.body.innerText;
		return JSON.stringify({
			markers: Markers.length,
			named: Named.filter((Name) => Text.includes(Name)).length,
		});
	})()`,
	returnByValue: true,
});

const Seen = JSON.parse(Count.result.value);
console.log(`attached: ${Files.length}  markers: ${Seen.markers}  named in page: ${Seen.named}`);
console.log(`input: multiple=${Multiple} accept=${Accept.split(",").length} extensions`);

Socket.close();
process.exit(Seen.named >= Files.length ? 0 : 1);

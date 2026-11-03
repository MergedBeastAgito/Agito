/**
 * Extract a chat product's step trace as a *tree* of JSON lines.
 *
 * A research answer renders its work as nested collapsible steps - "Searching
 * the web" holding the queries it ran, "Used tools" holding the calls it made -
 * and that nesting is the evidence for how the answer was reached. It is not in
 * the answer text, so a reader who keeps only `innerText` loses it.
 *
 * The trace used to be flat: one line per step with a `depth` number, which a
 * renderer could only draw as a table. Depth is a *consequence* of nesting, so
 * counting it and rebuilding the tree from it invents a structure the page
 * never claimed. This driver reads the nesting itself - a step's children are
 * the steps inside its own children container - and writes `id`/`parent` on
 * every line. `depth` survives for a reader's convenience and is never used to
 * rebuild anything.
 *
 * The extraction runs in the page, because the markup is hashed-class HTML that
 * changes between releases while the structure - a header button, a title, a
 * duration, a children container - is stable.
 *
 *   node cdp-trace.mjs <ws-url> [--thread <id>] [--expand]
 *   node cdp-trace.mjs --fixture <file.json>
 *   node cdp-trace.mjs --selftest
 *
 * `--fixture` reads the same shape the page returns, so the tree serialisation
 * can be exercised without a browser:
 *
 *   { "steps": [ { "title": "…", "children": [ … ] } ], "sources": [] }
 *
 * One JSON line per step and per source on stdout; a count on stderr. Append it
 * beside the answer so the trace survives the session that read it.
 *
 * Exit 0 with lines, 2 when it could not run.
 */
import { readFileSync } from "node:fs";

const Args = process.argv.slice(2);
const At = (Name, Fallback) => {
	const Index = Args.indexOf(Name);
	return Index === -1 ? Fallback : Args[Index + 1];
};
const Flag = (Name) => Args.includes(Name);
const WsUrl = Args[0] !== undefined && !Args[0].startsWith("--") ? Args[0] : null;
const Thread = At("--thread", null);
const Fixture = At("--fixture", null);
const Expand = Flag("--expand");
const Passes = Number(At("--expand-passes", 4));
const WaitMs = Number(At("--expand-wait", 600));
const SelfTest = Flag("--selftest");

if (WsUrl === null && Fixture === null && !SelfTest) {
	console.error(
		"usage: node cdp-trace.mjs <ws-url> [--thread <id>] [--expand] [--expand-passes N] [--expand-wait MS]\n" +
			"       node cdp-trace.mjs --fixture <file.json>\n" +
			"       node cdp-trace.mjs --selftest",
	);
	process.exit(2);
}

if (WsUrl === null && Expand) {
	console.error("blocker: --expand needs a page to click; a fixture has no DOM");
	process.exit(2);
}

process.on("unhandledRejection", (Error) => {
	console.error(`blocker: ${Error?.message ?? Error}`);
	process.exit(2);
});

/**
 * The page-side reader. It returns a nested tree, not a flat list: the nesting
 * is what the DOM already says, so resolving it here is a read, and flattening
 * it into ids happens outside, where it can be tested.
 */
const Extract = `(() => {
	const Ui = /^(read (more|less)|show (more|less)|copy|copy code|share|helpful|not helpful|add|model|wrap lines)$/i;
	const Duration = /^[0-9]+(\\.[0-9]+)?\\s*(ms|s|m|h)$/;
	const Candidates = ['div[class*="group/step-header"]', 'div[class*="step-header"]'];

	let Selector = null;
	let Label = null;
	let Headers = [];
	for (const Candidate of Candidates) {
		const Found = [...document.querySelectorAll(Candidate)];
		if (Found.length > 0) {
			Selector = Candidate;
			Label = Candidate;
			Headers = Found;
			break;
		}
	}
	// Last resort: the header button itself. The step is still the element that
	// contains both the row and the row's sibling, so the tree survives a class
	// rename - only the wrapper it hangs from changes.
	if (Headers.length === 0) {
		const Buttons = [...document.querySelectorAll("button[aria-expanded]")].filter(
			(Button) => Button.querySelector("div[title]") !== null,
		);
		if (Buttons.length > 0) {
			Selector = "button[aria-expanded]";
			Label = Selector + " (fallback: the header class moved)";
			Headers = Buttons;
		}
	}

	const Entries = [];
	const Seen = new Set();
	for (const Header of Headers) {
		// The header row is the step-header div, or - when the class has moved -
		// the button's parent. In both readings the step is the row's parent, and
		// the row's next sibling is the container its children live in.
		const Row = Header.tagName === "BUTTON" ? Header.parentElement : Header;
		if (Row === null) continue;
		const Node = Row.parentElement;
		if (Node === null || Seen.has(Node)) continue;
		const Button = Header.tagName === "BUTTON" ? Header : Header.querySelector("button");
		const Labelled = Row.querySelector("div[title]");
		const IconLabel = Row.querySelector("span[role=img][aria-label]");
		// The step's name is not the button's text: it lives in a title attribute
		// on the label div. innerText also picks up the duration and child rows.
		const Title = (
			Labelled?.getAttribute("title") ??
			IconLabel?.getAttribute("aria-label") ??
			(Button?.textContent ?? "")
		).trim();
		if (Title.length === 0 || Ui.test(Title)) continue;

		const Clock = Row.querySelector(".tabular-nums");
		const Raw = Clock === null ? null : (Clock.textContent || "").trim();
		const Use = Row.querySelector("svg use");
		const Href = Use ? Use.getAttribute("href") || Use.getAttribute("xlink:href") || "" : "";
		// aria-expanded is the product's own statement. The chevron is a second
		// witness, used only when the button is missing.
		const Expanded =
			Button === null
				? /chevron-down/.test(Href)
				: Button.getAttribute("aria-expanded") === "true";
		// It is not found by looking for padding-left: that is how the container
		// is drawn, not where it is.
		const Container = Row.nextElementSibling;

		Seen.add(Node);
		Entries.push({
			node: Node,
			line: {
				title: Title.slice(0, 120),
				duration: Raw !== null && Duration.test(Raw) ? Raw : null,
				icon: Href.replace(/^#pplx-icon-/, ""),
				expanded: Expanded,
				links: [...Row.querySelectorAll("a[href]")].map((Anchor) => Anchor.href).slice(0, 5),
			},
			container: Container,
		});
	}

	const ByNode = new Map(Entries.map((Entry) => [Entry.node, Entry]));
	const Nested = new Map(Entries.map((Entry) => [Entry, { ...Entry.line, children: [] }]));
	const Roots = [];

	for (const Entry of Entries) {
		// Nearest ancestor that is itself a step node: that is the container, and
		// nothing between the two is a step. This is the whole tree.
		let Parent = null;
		for (let Walk = Entry.node.parentElement; Walk !== null; Walk = Walk.parentElement) {
			const Candidate = ByNode.get(Walk);
			if (Candidate !== undefined) {
				Parent = Nested.get(Candidate);
				break;
			}
		}

		// Cross-check only: the padding wrapper count should equal the structural
		// depth. A mismatch is a markup change worth reporting, never a reason to
		// believe the padding instead.
		let Padded = 0;
		for (let Walk = Entry.node.parentElement; Walk !== null; Walk = Walk.parentElement) {
			const Pad = Walk.style ? Walk.style.paddingLeft : "";
			if (Pad !== "" && Pad !== "0px") Padded++;
		}

		const Line = Nested.get(Entry);
		Line.padded = Padded;
		if (Parent === null) Roots.push(Line);
		else Parent.children.push(Line);
	}

	const Sources = [...document.querySelectorAll("a[href^='http']")]
		.map((Anchor) => ({ href: Anchor.href, label: (Anchor.innerText || "").trim().slice(0, 100) }))
		.filter((Entry) => Entry.label.length > 0)
		.slice(0, 60);

	// A container that still holds headers while its step recorded no children
	// means something was filtered out (a UI label read as a step name). That is
	// a hole in the tree, and a hole is worth reporting rather than drawing.
	let Unread = 0;
	for (const Entry of Entries) {
		const Line = Nested.get(Entry);
		if (
			Line.children.length === 0 &&
			Entry.container !== null &&
			Selector !== null &&
			Entry.container.querySelectorAll(Selector).length > 0
		) {
			Unread++;
		}
	}

	return JSON.stringify({
		selector: Label,
		headers: Headers.length,
		unread: Unread,
		steps: Roots,
		sources: Sources,
	});
})()`;

/**
 * Click every collapsed step header, repeatedly: opening a node renders new
 * headers that were not in the DOM a moment ago, so one pass is not a tree.
 */
const Open = `(() => {
	const Collapsed = [...document.querySelectorAll('div[class*="step-header"] button[aria-expanded="false"]')];
	for (const Button of Collapsed) Button.click();
	return Collapsed.length;
})()`;

/** How many headers still refuse to be open - the only honest progress signal. */
const Count = `(() => document.querySelectorAll('div[class*="step-header"] button[aria-expanded="false"]').length)()`;

// The page-side scripts are strings: a broken escape inside them is a SyntaxError
// only the browser would report, so parse them here before a page is asked to
// run them.
if (SelfTest) {
	try {
		new Function(Extract);
		new Function(Open);
	} catch (Error) {
		console.error(`blocker: a page-side script does not parse: ${Error.message}`);
		process.exit(2);
	}
	console.error("page scripts parse: extract, expand");
	process.exit(0);
}

/**
 * Assign ids and parents. Pre-order, so a child's parent is always a line that
 * was already written - the invariant a reader (and the renderer) relies on.
 */
const Flatten = (Roots) => {
	const Lines = [];
	const Visit = (Node, Parent, Depth) => {
		const Id = "s" + (Lines.length + 1);
		const Children = Array.isArray(Node.children) ? Node.children : [];
		Lines.push({
			id: Id,
			parent: Parent,
			depth: Depth,
			title: Node.title ?? "",
			duration: Node.duration ?? null,
			icon: Node.icon ?? "",
			// `expanded: false` with nothing under it is not a leaf that did
			// nothing: it is a node whose contents were never rendered.
			expanded: Node.expanded === true,
			unexpanded: Node.expanded !== true && Children.length === 0,
			links: Array.isArray(Node.links) ? Node.links : [],
			padded: typeof Node.padded === "number" ? Node.padded : null,
		});
		for (const Child of Children) Visit(Child, Id, Depth + 1);
	};
	for (const Root of Roots) Visit(Root, null, 0);
	return Lines;
};

const Emit = (Payload) => {
	const At = new Date().toISOString();
	const Steps = Flatten(Payload.steps ?? []);
	const Sources = Array.isArray(Payload.sources) ? Payload.sources : [];

	for (const Step of Steps) {
		console.log(JSON.stringify({ at: At, thread: Thread, kind: "step", ...Step }));
	}
	for (const Source of Sources) {
		console.log(
			JSON.stringify({
				at: At,
				thread: Thread,
				kind: "source",
				...Source,
			}),
		);
	}

	const Deepest = Steps.reduce((Most, Step) => Math.max(Most, Step.depth), 0);
	const Collapsed = Steps.filter((Step) => Step.unexpanded).length;
	const Mismatched = Steps.filter(
		(Step) => Step.padded !== null && Step.padded !== Step.depth,
	).length;

	console.error(
		`steps: ${Steps.length}  sources: ${Sources.length}  deepest: ${Deepest}  collapsed: ${Collapsed}`,
	);
	if (Payload.selector) console.error(`headers: ${Payload.headers} via ${Payload.selector}`);
	if (Number(Payload.unread) > 0) {
		console.error(
			`warning: ${Payload.unread} step(s) have headers in their children container that were not read; the tree has a hole`,
		);
	}
	if (Steps.length === 0) {
		console.error(
			"warning: no step tree found; the answer may still be streaming, or the markup moved",
		);
	}
	if (Mismatched > 0) {
		console.error(
			`warning: ${Mismatched} step(s) whose structural depth differs from its padding wrapper count; the tree follows the DOM`,
		);
	}
	return Steps.length;
};

if (Fixture !== null) {
	let Payload;
	try {
		Payload = JSON.parse(readFileSync(Fixture, "utf8"));
	} catch (Error) {
		console.error(`blocker: cannot read fixture ${Fixture}: ${Error.message}`);
		process.exit(2);
	}
	Emit(Payload);
	process.exit(0);
}

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

// A collapsed node has no rendered children, so its subtree does not exist to
// be read. Expanding first is the difference between a trace of what was open
// and a trace of what the answer did.
if (Expand) {
	const Evaluate = async (Expression) => {
		const Result = await Send("Runtime.evaluate", {
			expression: Expression,
			returnByValue: true,
		});
		if (Result.exceptionDetails) {
			console.error(`blocker: expanding failed: ${Result.exceptionDetails.text}`);
			process.exit(2);
		}
		return Number(Result.result?.value ?? 0);
	};
	let Opened = 0;
	let Stuck = 0;
	for (let Pass = 0; Pass < Passes; Pass++) {
		const Before = await Evaluate(Count);
		if (Before === 0) break;
		await Evaluate(Open);
		await new Promise((Done) => setTimeout(Done, WaitMs));
		// A click is not an opening. Counting what is still collapsed after the
		// page has had time to render is the only progress signal there is, and
		// it stops the loop from spending every pass on a header that refuses.
		const After = await Evaluate(Count);
		if (After >= Before) {
			Stuck = After;
			break;
		}
		Opened += Before - After;
	}
	console.error(`expanded: ${Opened} step header(s) over at most ${Passes} pass(es)`);
	if (Stuck > 0) {
		console.error(
			`warning: ${Stuck} step header(s) stayed collapsed after a click; their subtrees are not in the trace`,
		);
	}
}

const Result = await Send("Runtime.evaluate", {
	expression: Extract,
	returnByValue: true,
});
if (Result.exceptionDetails) {
	console.error(`blocker: ${Result.exceptionDetails.text}`);
	process.exit(2);
}

let Parsed;
try {
	Parsed = JSON.parse(Result.result.value);
} catch {
	console.error("blocker: the page returned no measurement");
	process.exit(2);
}

Emit(Parsed);
Socket.close();

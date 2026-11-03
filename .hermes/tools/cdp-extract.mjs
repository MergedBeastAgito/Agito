/**
 * Serialise a rendered answer to Markdown, with the maths kept as maths.
 *
 * `document.body.innerText` is the wrong reader for a rendered page: KaTeX draws
 * a formula as hundreds of positioned glyphs, so innerText returns them one per
 * line and the expression is destroyed. The original TeX is still in the DOM,
 * inside `<annotation encoding="application/x-tex">`, and that is what this
 * reads.
 *
 * Everything here is defensive, because the page is a moving target:
 *
 *   - the container is found by trying a chain of selectors, then the largest
 *     text block, and the choice is reported;
 *   - UI chrome inside the container (buttons, svg, the composer, suggestion
 *     rows) is skipped rather than rendered as prose;
 *   - a capture that is suspiciously small is a failure, not an answer;
 *   - a collapsed answer is reported, because its text is not in the DOM.
 *
 *   node cdp-extract.mjs <ws-url> [--selector <css>] [--out <file>] [--min-chars N]
 *
 * Exit 0 with Markdown on stdout, 1 when the capture is too small to trust,
 * 2 when it could not run.
 */
import { writeFileSync } from "node:fs";

const Args = process.argv.slice(2);
const WsUrl = Args[0];
const At = (Name, Fallback) => {
	const Index = Args.indexOf(Name);
	return Index === -1 ? Fallback : Args[Index + 1];
};
const Selector = At("--selector", null);
const Out = At("--out", null);
const MinChars = Number(At("--min-chars", 200));

if (!WsUrl) {
	console.error("usage: node cdp-extract.mjs <ws-url> [--selector <css>] [--out <file>] [--min-chars N]");
	process.exit(2);
}

process.on("unhandledRejection", (Error) => {
	console.error(`blocker: ${Error?.message ?? Error}`);
	process.exit(2);
});

const Extract = `(() => {
	const Chain = ${Selector === null ? '["main", "[role=main]", "article"]' : JSON.stringify([Selector])};
	const Wanted = ${Selector === null ? "null" : JSON.stringify(Selector)};
	let Root = null;
	let Chosen = null;
	for (const Candidate of Chain) {
		const Found = document.querySelector(Candidate);
		if (Found === null) continue;
		// An explicitly named container is honoured even when it is short: the
		// caller asked for that element, and a short one is a finding, not a
		// reason to widen the search behind their back.
		if (Wanted !== null || (Found.innerText || "").trim().length > 200) {
			Root = Found;
			Chosen = Candidate;
			break;
		}
	}
	// A named selector that does not exist is an error, not an invitation to
	// capture something else: the caller was specific, so silence would be a lie.
	if (Root === null && Wanted !== null) return JSON.stringify({ error: "no element for selector " + Wanted });
	if (Root === null) {
		const All = [...document.querySelectorAll("div, section, article, main")]
			.filter((Element) => Element.children.length > 0)
			.sort((Left, Right) => (Right.innerText || "").length - (Left.innerText || "").length);
		Root = All[0] ?? document.body;
		Chosen = "largest text block";
	}
	if (Root === null) return JSON.stringify({ error: "no container and no body" });

	const Skip = new Set(["BUTTON", "SVG", "NAV", "FORM", "INPUT", "TEXTAREA", "SELECT", "STYLE", "SCRIPT", "NOSCRIPT", "TEMPLATE"]);
	const SkipClass = /(^|\\s)(composer|suggestion|toolbar|sidebar|prose-actions)(\\s|$)/i;
	const SkipLabel = /^(copy|copy code|share|helpful|not helpful|read more|read less|show more|show less|add|model|dictation|submit|wrap lines|search|computer)$/i;

	const TeX = (Element) => {
		const Annotation = Element.querySelector('annotation[encoding="application/x-tex"]');
		return (Annotation ? Annotation.textContent : Element.textContent).trim();
	};

	const shouldSkip = (Element) => {
		if (Skip.has(Element.tagName)) return true;
		const Label = (Element.getAttribute("aria-label") || "").trim();
		if (Label.length > 0 && SkipLabel.test(Label)) return true;
		const Class = typeof Element.className === "string" ? Element.className : "";
		return SkipClass.test(Class);
	};

	const Inline = (Element) => {
		let Text = "";
		for (const Node of Element.childNodes) {
			if (Node.nodeType === 3) {
				Text += Node.textContent.replace(/\\s+/g, " ");
				continue;
			}
			if (Node.nodeType !== 1) continue;
			if (shouldSkip(Node)) continue;
			const Tag = Node.tagName.toLowerCase();
			if (Node.classList.contains("katex") || Node.querySelector(":scope > .katex")) {
				const Display = Node.classList.contains("katex-display");
				Text += (Display ? "$$" : "$") + TeX(Node) + (Display ? "$$" : "$");
			} else if (Tag === "code") {
				Text += "\\u0060" + Node.textContent + "\\u0060";
			} else if (Tag === "strong" || Tag === "b") {
				Text += "**" + Inline(Node) + "**";
			} else if (Tag === "em" || Tag === "i") {
				Text += "*" + Inline(Node) + "*";
			} else if (Tag === "a") {
				const Href = Node.getAttribute("href") || "";
				const Label = Inline(Node).trim();
				Text += Href.startsWith("http") && Label.length > 0 ? "[" + Label + "](" + Href + ")" : Label;
			} else if (Tag === "img") {
				const Src = Node.getAttribute("src") || "";
				const Alt = (Node.getAttribute("alt") || "").trim();
				if (Src.startsWith("http")) Text += "![" + Alt + "](" + Src + ")";
			} else if (Tag === "br") {
				Text += "\\n";
			} else {
				Text += Inline(Node);
			}
		}
		return Text;
	};

	const Blocks = (Element) => {
		let Out = "";
		for (const Node of Element.children) {
			if (shouldSkip(Node)) continue;
			const Tag = Node.tagName.toLowerCase();
			const Class = typeof Node.className === "string" ? Node.className : "";

			if (Class.includes("katex-display")) {
				Out += "\\n\\n$$" + TeX(Node) + "$$\\n\\n";
			} else if (/^h[1-6]$/.test(Tag)) {
				Out += "\\n\\n" + "#".repeat(Number(Tag[1])) + " " + Inline(Node).trim() + "\\n\\n";
			} else if (Tag === "p") {
				const Text = Inline(Node).trim();
				if (Text.length > 0) Out += "\\n\\n" + Text + "\\n\\n";
			} else if (Tag === "ul" || Tag === "ol") {
				Out += "\\n";
				let Index = 0;
				for (const Item of Node.querySelectorAll(":scope > li")) {
					Index++;
					Out += (Tag === "ol" ? Index + ". " : "- ") + Inline(Item).trim().replace(/\\n+/g, " ") + "\\n";
					const Nested = Item.querySelector(":scope > ul, :scope > ol");
					if (Nested !== null) {
						Out += Blocks(Nested)
							.split("\\n")
							.filter((Line) => Line.trim().length > 0)
							.map((Line) => "  " + Line)
							.join("\\n") + "\\n";
					}
				}
				Out += "\\n";
			} else if (Tag === "pre") {
				const Code = Node.querySelector("code");
				const Language = Code && Code.className ? (Code.className.match(/language-([a-z0-9+]+)/i) || [])[1] || "" : "";
				Out += "\\n\\n\\u0060\\u0060\\u0060" + Language + "\\n" + (Code ? Code.textContent : Node.textContent).replace(/\\n+$/, "") + "\\n\\u0060\\u0060\\u0060\\n\\n";
			} else if (Tag === "blockquote") {
				Out += "\\n\\n> " + Inline(Node).trim() + "\\n\\n";
			} else if (Tag === "hr") {
				Out += "\\n\\n---\\n\\n";
			} else if (Tag === "table") {
				const Rows = [...Node.querySelectorAll("tr")].map((Row) =>
					[...Row.querySelectorAll("th, td")].map((Cell) => Inline(Cell).trim().replace(/\\|/g, "\\\\|")),
				);
				if (Rows.length > 0 && Rows[0].length > 0) {
					Out += "\\n\\n| " + Rows[0].join(" | ") + " |\\n|" + Rows[0].map(() => " --- ").join("|") + "|\\n";
					for (const Row of Rows.slice(1)) Out += "| " + Row.join(" | ") + " |\\n";
					Out += "\\n";
				}
			} else if (Tag === "img") {
				const Src = Node.getAttribute("src") || "";
				if (Src.startsWith("http")) Out += "\\n\\n![" + (Node.getAttribute("alt") || "").trim() + "](" + Src + ")\\n\\n";
			} else if (Node.children.length > 0) {
				Out += Blocks(Node);
			} else {
				const Text = Inline(Node).trim();
				if (Text.length > 0) Out += Text + "\\n";
			}
		}
		return Out;
	};

	const Markdown = Blocks(Root)
		.replace(/\\n{3,}/g, "\\n\\n")
		.replace(/[ \\t]+\\n/g, "\\n")
		.trim();

	return JSON.stringify({
		container: Chosen,
		chars: Markdown.length,
		katex: Root.querySelectorAll(".katex").length,
		code: Root.querySelectorAll("pre").length,
		tables: Root.querySelectorAll("table").length,
		images: Root.querySelectorAll("img").length,
		collapsed: /^(read more|show more)$/im.test(Root.innerText || ""),
		markdown: Markdown,
	});
})()`;

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

const Result = await Send("Runtime.evaluate", { expression: Extract, returnByValue: true });
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

if (Parsed.error) {
	console.error(`blocker: ${Parsed.error}`);
	process.exit(2);
}

const Summary = `container: ${Parsed.container}  chars: ${Parsed.chars}  formulas: ${Parsed.katex}  code: ${Parsed.code}  tables: ${Parsed.tables}  images: ${Parsed.images}${Parsed.collapsed ? "  COLLAPSED" : ""}`;

if (Parsed.chars < MinChars) {
	console.error(`blocker: the capture is ${Parsed.chars} characters, below --min-chars ${MinChars}; refusing a stub. ${Summary}`);
	process.exit(1);
}

if (Out !== null) {
	writeFileSync(Out, Parsed.markdown + "\n");
	console.error(`wrote ${Out}  ${Summary}`);
} else {
	process.stderr.write(Summary + "\n");
	process.stdout.write(Parsed.markdown + "\n");
}
Socket.close();

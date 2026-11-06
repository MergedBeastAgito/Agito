/**
 * The answer DOM, serialised as Markdown.
 *
 * `innerText` is the wrong reader for a rendered answer. It keeps a table's
 * cells as tab-separated lines with no pipes, drops the structure of a list,
 * renders a formula as hundreds of positioned glyphs one per line, and can lose
 * `pre > code` entirely. The text is not wrong - it is unformatted - but a note
 * built from it is not the answer the product gave.
 *
 * This file is the whole reader, and it is a file rather than a Rust format
 * string for one reason: it is JavaScript, and a string that is never compiled,
 * parsed or formatted is a string whose syntax errors are found in production.
 * `Fn/Expressions.rs` splices it into the expression that reads the answer, so
 * the observation stream, the completion rule and the turn correlation are
 * untouched - only what "the text of an answer" means has changed.
 *
 * Five rules, and the reason each one is a rule rather than a nicety:
 *
 * - **A formula is its source, never its glyphs.** KaTeX writes the expression
 *   twice: a MathML tree carrying `<annotation encoding="application/x-tex">`,
 *   and an `aria-hidden` tree of positioned glyphs. Reading the second is what
 *   turns `E = mc^2` into nothing at all, so the glyph tree is skipped and the
 *   annotation is read.
 * - **A table keeps its columns.** The header is `<thead>`'s first row, or the
 *   table's first row when it has no head; the separator is sized to the widest
 *   row so a row with more cells than the header cannot silently drop them; a
 *   `|` inside a cell is escaped, because an unescaped one is a column the
 *   reader never wrote.
 * - **A fence outgrows its body.** The fence is one backtick longer than the
 *   longest run in the code, so a block that itself contains a fence survives
 *   instead of ending early.
 * - **A nested table is flattened, not nested.** Markdown has no table inside a
 *   cell; a nested table's cells become text in the cell that holds them, and
 *   its rows never join the outer table's.
 * - **Prose is paragraphs.** Runs of inline text are separated by one blank
 *   line, whitespace inside a run is one space, and no line ends in a space.
 *
 * A value the page does not have is the empty string, never a guess: a formula
 * with no annotation contributes nothing rather than its glyphs, and a cell
 * that is not there is an empty cell rather than a shifted row.
 */
const Serialise = (Root: Element | null): string => {
	if (Root === null || Root === undefined) return "";

	/** Elements that are chrome, controls or duplicate trees, never prose. */
	const Skip = new Set([
		"BUTTON",
		"CANVAS",
		"FORM",
		"IFRAME",
		"INPUT",
		"NOSCRIPT",
		"OPTION",
		"SCRIPT",
		"SELECT",
		"STYLE",
		"SVG",
		"TEMPLATE",
		"TEXTAREA",
		"VIDEO",
		"AUDIO",
	]);

	/** Class names that mark a control strip rather than the answer. */
	const SkipClass = /(^|\s)(composer|suggestion|toolbar|sidebar|prose-actions)(\s|$)/i;

	/** Labels of a control that happens to be a `button` inside the answer. */
	const SkipLabel =
		/^(copy|copy code|share|helpful|not helpful|read more|read less|show more|show less|add|model|dictation|submit|wrap lines|search|computer)$/i;

	/** Whether an element contributes nothing to the answer's text. */
	const Skipped = (Element) => {
		if (Skip.has(Element.tagName)) return true;
		// The glyph half of a formula, and any other decorative duplicate: it is
		// in the DOM and not part of what the page said.
		if (Element.getAttribute("aria-hidden") === "true") return true;
		if (Element.hidden === true) return true;
		const Label = (Element.getAttribute("aria-label") || "").trim();
		if (Label.length > 0 && SkipLabel.test(Label)) return true;
		const Class = typeof Element.className === "string" ? Element.className : "";
		if (SkipClass.test(Class)) return true;
		// A hidden subtree - a closed dialog, a collapsed panel - is in the DOM
		// and not on the page. The computed style is read rather than the box:
		// `display: contents` has no box and does render its children.
		const Style = getComputedStyle(Element);
		return Style.display === "none" || Style.visibility === "hidden";
	};

	/** A formula's TeX, or the empty string when the page does not carry it. */
	const Tex = (Element) => {
		const Annotation = Element.querySelector('annotation[encoding="application/x-tex"]');
		return Annotation === null ? "" : (Annotation.textContent || "").trim();
	};

	/** Whether an element is a rendered formula. */
	const IsMath = (Element) =>
		Element.classList.contains("katex") ||
		Element.classList.contains("katex-display") ||
		Element.querySelector(":scope > .katex") !== null;

	/** Whether an element is a displayed formula, on its own line. */
	const IsDisplayMath = (Element) =>
		Element.classList.contains("katex-display") || Element.closest(".katex-display") !== null;

	/** The longest run of backticks in some text, so a fence can be longer. */
	const Run = (Text) => {
		let Longest = 0;
		for (const Match of Text.matchAll(/`+/g)) Longest = Math.max(Longest, Match[0].length);
		return Longest;
	};

	/** One run of whitespace, because a page's own line breaks are not text. */
	const Collapse = (Text) => Text.replace(/\s+/g, " ");

	/** A fenced block whose fence the body cannot close. */
	const Fence = (Text, Language) => {
		const Ticks = "`".repeat(Math.max(3, Run(Text) + 1));
		return Ticks + Language + "\n" + Text.replace(/\n+$/, "") + "\n" + Ticks;
	};

	/** The language a code block declares, as a word a fence line can hold. */
	const LanguageOf = (Element) => {
		if (Element === null) return "";
		const Declared = (Element.getAttribute("data-language") || "").trim();
		const Class = typeof Element.className === "string" ? Element.className : "";
		const Match = Class.match(/(?:^|\s)language-([^\s]+)/);
		const Found = Declared.length > 0 ? Declared : Match === null ? "" : Match[1];
		return Found.replace(/[^A-Za-z0-9_+#.-]/g, "");
	};

	/** Inline content: text, emphasis, links, maths and inline code. */
	const Inline = (Node) => {
		if (Node.nodeType === 3) return Collapse(Node.nodeValue || "");
		if (Node.nodeType !== 1) return "";
		if (Skipped(Node)) return "";
		if (IsMath(Node)) {
			const Source = Tex(Node);
			if (Source.length === 0) return "";
			return IsDisplayMath(Node) ? "$$" + Source + "$$" : "$" + Source + "$";
		}
		const Tag = Node.tagName;
		if (Tag === "BR") return " ";
		if (Tag === "CODE") {
			const Source = Node.textContent || "";
			const Ticks = "`".repeat(Math.max(1, Run(Source) + 1));
			// A body that starts or ends with a backtick is padded, or the fence
			// and the body would be one run of backticks.
			const Pad = Source.startsWith("`") || Source.endsWith("`") ? " " : "";
			return Ticks + Pad + Source + Pad + Ticks;
		}
		if (Tag === "STRONG" || Tag === "B") {
			const Text = InlineChildren(Node);
			return Text.trim().length === 0 ? Text : "**" + Text.trim() + "**";
		}
		if (Tag === "EM" || Tag === "I") {
			const Text = InlineChildren(Node);
			return Text.trim().length === 0 ? Text : "*" + Text.trim() + "*";
		}
		if (Tag === "DEL" || Tag === "S") {
			const Text = InlineChildren(Node);
			return Text.trim().length === 0 ? Text : "~~" + Text.trim() + "~~";
		}
		if (Tag === "A") {
			const Href = (Node.getAttribute("href") || "").trim();
			const Label = InlineChildren(Node).trim();
			// A citation marker is the product's footnote: a number standing for
			// a source chip, or an anchor that goes nowhere. It is kept as the
			// number the page showed rather than turned into a link the reader
			// would have to follow to learn what `1` meant.
			const Citation =
				/^\d{1,3}$/.test(Label) ||
				Href.length === 0 ||
				Href.startsWith("#") ||
				Href.startsWith("javascript:");
			if (Citation || Label.length === 0) return Label;
			return "[" + Label + "](" + Href + ")";
		}
		if (Tag === "IMG") {
			const Src = (Node.getAttribute("src") || "").trim();
			if (Src.length === 0) return "";
			return "![" + (Node.getAttribute("alt") || "").trim() + "](" + Src + ")";
		}
		// A cell reached outside a table is a flattened table inside a cell: its
		// cells stay text, separated so two of them do not become one word.
		if (Tag === "TD" || Tag === "TH" || Tag === "TR") return InlineChildren(Node) + " ";
		return InlineChildren(Node);
	};

	/** Every child of an element, read inline. */
	const InlineChildren = (Element) => {
		let Text = "";
		for (const Child of Element.childNodes) Text += Inline(Child);
		return Text;
	};

	/** Blocks that stand on their own line rather than inside a paragraph. */
	const Block = new Set([
		"P",
		"DIV",
		"SECTION",
		"ARTICLE",
		"HEADER",
		"FOOTER",
		"MAIN",
		"ASIDE",
		"FIGURE",
		"FIGCAPTION",
		"DETAILS",
		"SUMMARY",
		"UL",
		"OL",
		"LI",
		"DL",
		"DT",
		"DD",
		"H1",
		"H2",
		"H3",
		"H4",
		"H5",
		"H6",
		"BLOCKQUOTE",
		"PRE",
		"TABLE",
		"HR",
		"NAV",
		"FORM",
		"FIELDSET",
	]);

	/** A cell's text: one line, and no pipe that would read as a column. */
	const Cell = (Element) =>
		Element === undefined ? "" : Collapse(Inline(Element)).trim().replace(/\|/g, "\\|");

	/** `<table>` as a Markdown table, with nested tables flattened into text. */
	const Table = (Table) => {
		// Rows of *this* table. A nested table's rows belong to the cell that
		// holds them, and lifting them into the outer table would invent columns
		// the outer table does not have.
		const Rows = [...Table.querySelectorAll("tr")].filter(
			(Row) => Row.closest("table") === Table,
		);
		if (Rows.length === 0) return "";
		const Head = Table.querySelector("thead");
		const Named =
			Head === null
				? null
				: [...Head.querySelectorAll("tr")].find((Row) => Row.closest("table") === Table);
		const Header = Named === null || Named === undefined ? Rows[0] : Named;
		const Cells = (Row) =>
			[...Row.children].filter((Cell) => Cell.tagName === "TH" || Cell.tagName === "TD");
		const Grid = [Header, ...Rows.filter((Row) => Row !== Header)].map(Cells);
		const Width = Grid.reduce((Widest, Row) => Math.max(Widest, Row.length), 0);
		if (Width === 0) return "";
		const Line = (Row) =>
			"| " + Array.from({ length: Width }, (_, At) => Cell(Row[At])).join(" | ") + " |";
		const Lines = [
			Line(Grid[0]),
			"| " + Array.from({ length: Width }, () => "---").join(" | ") + " |",
		];
		for (const Row of Grid.slice(1)) Lines.push(Line(Row));
		return Lines.join("\n");
	};

	/** `pre > code` as a fenced block, its fence longer than anything inside. */
	const Pre = (Element) => {
		const Code = Element.querySelector("code");
		const Source = Code === null ? Element.textContent || "" : Code.textContent || "";
		return Fence(Source, LanguageOf(Code) || LanguageOf(Element));
	};

	/** A list, with its items' own blocks indented under the marker. */
	const List = (Element) => {
		const Ordered = Element.tagName === "OL";
		const Start = Number(Element.getAttribute("start") || 1);
		const Lines = [];
		const Items = [...Element.children].filter((Child) => Child.tagName === "LI");
		Items.forEach((Item, Index) => {
			const Marker = Ordered ? Start + Index + ". " : "- ";
			const Pad = " ".repeat(Marker.length);
			const Parts = Blocks(Item);
			if (Parts.length === 0) {
				Lines.push(Marker.trim());
				return;
			}
			const [First, ...Rest] = Parts;
			const FirstLines = First.split("\n");
			Lines.push(Marker + FirstLines[0]);
			for (const Line of FirstLines.slice(1)) Lines.push(Pad + Line);
			for (const Part of Rest) {
				for (const Line of Part.split("\n")) Lines.push(Pad + Line);
			}
		});
		return Lines.join("\n");
	};

	/** One block element, as the blocks it contributes. */
	const BlockOf = (Element) => {
		const Tag = Element.tagName;
		if (Tag === "TABLE") return [Table(Element)];
		if (Tag === "PRE") return [Pre(Element)];
		if (Tag === "HR") return ["---"];
		if (Tag === "UL" || Tag === "OL") return [List(Element)];
		if (Tag === "BLOCKQUOTE") {
			const Inner = Blocks(Element).join("\n\n").split("\n");
			return [Inner.map((Line) => (Line.length === 0 ? ">" : "> " + Line)).join("\n")];
		}
		if (/^H[1-6]$/.test(Tag)) {
			const Text = Collapse(InlineChildren(Element)).trim();
			return Text.length === 0 ? [] : ["#".repeat(Number(Tag[1])) + " " + Text];
		}
		return Blocks(Element);
	};

	/** Everything inside an element, as blocks. */
	const Blocks = (Element) => {
		const Out = [];
		let Run = "";
		const Flush = () => {
			const Text = Collapse(Run).trim();
			Run = "";
			if (Text.length > 0) Out.push(Text);
		};
		for (const Child of Element.childNodes) {
			if (Child.nodeType === 3) {
				Run += Child.nodeValue || "";
				continue;
			}
			if (Child.nodeType !== 1) continue;
			if (Skipped(Child)) continue;
			if (IsMath(Child) && IsDisplayMath(Child)) {
				Flush();
				const Source = Tex(Child);
				if (Source.length > 0) Out.push("$$" + Source + "$$");
				continue;
			}
			if (Block.has(Child.tagName)) {
				Flush();
				for (const Part of BlockOf(Child)) Out.push(Part);
				continue;
			}
			Run += Inline(Child);
		}
		Flush();
		return Out;
	};

	// A fence owns its own bytes: a trailing space inside a code block is part
	// of the code, and the no-trailing-space rule is a rule about prose.
	const Fenced = /^`{3,}/;
	return Blocks(Root)
		.map((Part) => (Fenced.test(Part) ? Part : Part.replace(/[ \t]+$/gm, "")))
		.join("\n\n")
		.trim();
};

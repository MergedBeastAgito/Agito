/**
 * The contract of `agentic.chat.ask`, proved against a fake page.
 *
 * No Chromium, no network, no dependency: `node --test Test/*.test.ts` in this
 * package.
 *
 * The fake is a page-shaped object rather than a table of canned answers: every
 * expression the driver sends is evaluated against it for real, with `new
 * Function` and a `document` that answers by selector. So what is under test is
 * the expression the driver actually sends and the rule it applies to what came
 * back - not a string this file happened to expect.
 *
 * A streaming answer and a stalled one are the same fake with a different `serve`
 * function. The streaming page serves its texts in order and repeats the last one
 * for ever, so three consecutive reads eventually agree and the answer settles.
 * The stalled page serves a different text on every read, so no two reads ever
 * agree, the deadline is the only thing that can end the wait, and the driver
 * reports `not-ready` rather than handing back the last thing it saw.
 */
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { describe, test } from "node:test";

import {
	ChatError,
	Create,
	DefaultSelectors,
	ErrorCode,
	type DriverOptions,
	type OperationOptions,
	type Page,
	type Question,
	type Target,
} from "../src/index.ts";

const Url = "https://chat.example.test/";
const S = DefaultSelectors;

type ElementOptions = {
	tagName?: string;
	innerText?: string;
	attributes?: Record<string, string>;
	disabled?: boolean;
	/** What the element's text is on its Nth read, counting from zero. */
	serve?: (read: number) => string;
	onClick?: () => void;
};

class FakeElement {
	readonly tagName: string;
	declare value: string | undefined;
	disabled = false;
	clicks = 0;
	events: string[] = [];
	attributes: Record<string, string>;
	serve: ((read: number) => string) | undefined;
	onClick: (() => void) | undefined;
	readonly children = new Map<string, FakeElement[]>();
	#text: string;
	#reads = 0;

	constructor(options: ElementOptions = {}) {
		this.tagName = options.tagName ?? "DIV";
		this.#text = options.innerText ?? "";
		this.attributes = options.attributes ?? {};
		this.disabled = options.disabled ?? false;
		this.serve = options.serve;
		this.onClick = options.onClick;
	}

	get innerText(): string {
		const read = this.#reads;
		this.#reads += 1;
		return this.serve === undefined ? this.#text : this.serve(read);
	}

	set innerText(text: string) {
		this.#text = text;
	}

	getAttribute(name: string): string | null {
		return Object.hasOwn(this.attributes, name)
			? this.attributes[name]
			: null;
	}

	setAttribute(name: string, value: string): void {
		this.attributes[name] = value;
	}

	click(): void {
		this.clicks += 1;
		this.onClick?.();
	}

	dispatchEvent(event: { type: string }): boolean {
		this.events.push(event.type);
		return true;
	}

	querySelector(selector: string): FakeElement | null {
		return this.querySelectorAll(selector)[0] ?? null;
	}

	querySelectorAll(selector: string): FakeElement[] {
		return this.children.get(selector) ?? [];
	}

	/** Place child elements under a selector, the way a page would answer. */
	place(selector: string, ...elements: FakeElement[]): FakeElement[] {
		this.children.set(selector, elements);
		return elements;
	}
}

class FakeDocument {
	readonly elements = new Map<string, FakeElement[]>();

	querySelector(selector: string): FakeElement | null {
		return this.querySelectorAll(selector)[0] ?? null;
	}

	querySelectorAll(selector: string): FakeElement[] {
		return this.elements.get(selector) ?? [];
	}

	place(selector: string, ...elements: FakeElement[]): FakeElement[] {
		this.elements.set(selector, elements);
		return elements;
	}
}

class FakePage implements Page {
	readonly document = new FakeDocument();
	opened: string[] = [];
	disposed: Target[] = [];
	asked: string[] = [];
	openFailure: Error | undefined;
	closed = false;
	hang = false;
	hangOnOpen = false;
	hangOnDispose = false;
	/**
	 * Rewrite what the page hands back, to simulate a provider that answers the
	 * driver's expression with a value of the wrong shape. The driver's rules must
	 * still refuse it with one of the manifest's four codes.
	 */
	rewrite: ((expression: string, value: unknown) => unknown) | undefined;

	async open(url: string, _options: OperationOptions): Promise<Target> {
		this.opened.push(url);
		if (this.hangOnOpen) return new Promise<Target>(() => {});
		if (this.openFailure !== undefined) throw this.openFailure;
		return { url };
	}

	async evaluate<T>(
		_target: Target,
		expression: string,
		_options: OperationOptions,
	): Promise<T> {
		this.asked.push(expression);
		if (this.closed) throw new Error("the page is gone");
		if (this.hang) return new Promise<T>(() => {});
		const run = new Function("document", `return (${expression});`);
		const value = run(this.document) as unknown;
		return (
			this.rewrite === undefined ? value : this.rewrite(expression, value)
		) as T;
	}

	async dispose(target: Target): Promise<void> {
		this.disposed.push(target);
		if (this.hangOnDispose) return new Promise<void>(() => {});
	}
}

type PageOptions = {
	/** The mode the composer is in, and what pressing the toggle does. */
	mode?: "search" | "computer" | "stuck";
	/** The model label the composer shows. */
	model?: string;
	/** The texts the answer serves in order; the last one repeats for ever. */
	stream?: string[];
	/** Serve a different text on every read, so nothing ever settles. */
	stall?: boolean;
	/** Put a `Read more` control over the answer. */
	collapsed?: boolean;
	/** The steps the new answer reports, placed inside it. */
	trace?: ElementOptions[];
	/** The sources the new answer cites, placed inside it. */
	sources?: ElementOptions[];
	/** The steps the answer already in the thread reports. */
	earlierTrace?: ElementOptions[];
	/** The sources the answer already in the thread cites. */
	earlierSources?: ElementOptions[];
	/** An answer that was already in the thread before the question. */
	earlier?: string;
	/** Leave the send control disabled, so it cannot be pressed. */
	disabledSubmit?: boolean;
};

/** One element, built from its description. */
const element = (options: ElementOptions): FakeElement =>
	new FakeElement(options);

/** A page that behaves like a chat product, and records what was done to it. */
const build = (options: PageOptions = {}) => {
	const page = new FakePage();
	const mode = options.mode ?? "search";

	const search = new FakeElement({
		tagName: "BUTTON",
		attributes: {
			"aria-label": "Search",
			"aria-pressed": mode === "search" ? "true" : "false",
		},
	});
	const computer = new FakeElement({
		tagName: "BUTTON",
		attributes: {
			"aria-label": "Computer",
			"aria-pressed": mode === "search" ? "false" : "true",
		},
	});
	const press = () => {
		// `stuck` is a toggle that is pressed and does not move: the page accepts
		// the click and stays in the mode it was in.
		if (mode === "stuck") return;
		search.setAttribute("aria-pressed", "true");
		computer.setAttribute("aria-pressed", "false");
	};
	search.onClick = press;
	computer.onClick = press;
	page.document.place(S.mode, search, computer);

	page.document.place(
		S.model,
		new FakeElement({
			tagName: "BUTTON",
			innerText: options.model ?? "GPT-6.1 Sol",
		}),
	);

	const composer = new FakeElement({ tagName: "TEXTAREA" });
	composer.value = "";
	page.document.place(S.composer, composer);

	if (options.earlier !== undefined) {
		const earlier = new FakeElement({
			tagName: "DIV",
			innerText: options.earlier,
		});
		earlier.place(S.trace, ...(options.earlierTrace ?? []).map(element));
		earlier.place(
			S.sources,
			...(options.earlierSources ?? []).map(element),
		);
		page.document.place(S.answer, earlier);
	}

	const submit = new FakeElement({
		tagName: "BUTTON",
		attributes: { "aria-label": "Submit" },
		disabled: options.disabledSubmit === true,
	});
	submit.onClick = () => {
		// A disabled control sends nothing, exactly as a real one would.
		if (submit.disabled || (composer.value ?? "").length === 0) return;
		const answer = new FakeElement({ tagName: "DIV" });
		if (options.stall === true)
			answer.serve = (read) => `still writing ${read}`;
		else {
			const texts = options.stream ?? ["The whole answer"];
			answer.serve = (read) => texts[Math.min(read, texts.length - 1)];
		}
		if (options.collapsed === true)
			answer.place(
				S.collapse,
				new FakeElement({ tagName: "BUTTON", innerText: "Read more" }),
			);
		answer.place(S.trace, ...(options.trace ?? []).map(element));
		answer.place(S.sources, ...(options.sources ?? []).map(element));
		page.document.place(
			S.answer,
			...page.document.querySelectorAll(S.answer),
			answer,
		);
	};
	page.document.place(S.submit, submit);

	return { page, search, computer, composer, submit };
};

const question = (overrides: Partial<Question> = {}): Question => ({
	text: "What is the smallest portable workflow for asking a web chat UI?",
	mode: "search",
	model: "GPT-6.1 Sol",
	...overrides,
});

const options = (
	timeoutMs = 400,
	signal = new AbortController().signal,
): OperationOptions => ({ timeoutMs, signal });

const driver = (page: FakePage, overrides: Partial<DriverOptions> = {}) =>
	Create(page, { url: Url, pollMs: 0, ...overrides });

/** Await a rejection and insist it is the port's error, never a string. */
const rejection = async (work: Promise<unknown>): Promise<ChatError> => {
	try {
		await work;
	} catch (error) {
		assert.ok(
			error instanceof ChatError,
			`expected a ChatError, got ${String(error)}`,
		);
		return error;
	}
	assert.fail("expected the call to reject");
};

describe("agentic.chat.ask", () => {
	test("the port declares exactly the errors the manifest promises", () => {
		const manifest = JSON.parse(
			readFileSync(new URL("../module.json", import.meta.url), "utf8"),
		) as { provides: { errors: string[] } };
		assert.deepEqual(
			Object.values(ErrorCode).sort(),
			[...manifest.provides.errors].sort(),
		);
	});

	test("returns the answer, its trace and its sources", async () => {
		const { page } = build({
			trace: [
				{
					tagName: "LI",
					innerText: "Searching",
					attributes: { "data-depth": "0", "data-detail": "12s" },
				},
				{
					tagName: "LI",
					innerText: "Reading",
					attributes: { "data-depth": "1" },
				},
			],
			sources: [
				{
					tagName: "A",
					innerText: "astro",
					attributes: {
						href: "https://docs.astro.build/en/concepts/islands/",
					},
				},
			],
		});

		const answer = await driver(page).ask(question(), options());

		assert.equal(answer.text, "The whole answer");
		assert.deepEqual(answer.trace, [
			{ depth: 0, label: "Searching", detail: "12s" },
			{ depth: 1, label: "Reading", detail: "" },
		]);
		assert.deepEqual(answer.sources, [
			{
				url: "https://docs.astro.build/en/concepts/islands/",
				label: "astro",
			},
		]);
		assert.equal(answer.model, "GPT-6.1 Sol");
		assert.equal(answer.mode, "search");
	});

	test("opens the configured page and disposes the target once", async () => {
		const { page } = build();
		await driver(page).ask(question(), options());
		assert.deepEqual(page.opened, [Url]);
		assert.equal(page.disposed.length, 1);
		assert.equal(page.disposed[0].url, Url);
	});
});

describe("the mode is a precondition", () => {
	test("presses the toggle when the composer is not in search mode", async () => {
		const { page, search, computer } = build({ mode: "computer" });

		const answer = await driver(page).ask(question(), options());

		assert.equal(answer.text, "The whole answer");
		assert.equal(search.clicks, 1);
		assert.equal(computer.clicks, 0);
		assert.equal(search.getAttribute("aria-pressed"), "true");
	});

	test("refuses with wrong-mode and sends nothing when the toggle never reads search", async () => {
		const { page, composer, submit } = build({ mode: "stuck" });

		const error = await rejection(driver(page).ask(question(), options()));

		assert.equal(error.code, ErrorCode.WrongMode);
		assert.match(error.detail, /computer/);
		assert.equal(submit.clicks, 0);
		assert.deepEqual(composer.events, []);
		assert.equal(page.disposed.length, 1);
	});

	test("refuses a question that asks for a mode this port does not offer", async () => {
		const { page } = build();
		const wanted = {
			...question(),
			mode: "computer",
		} as unknown as Question;

		const error = await rejection(driver(page).ask(wanted, options()));

		assert.equal(error.code, ErrorCode.WrongMode);
		assert.deepEqual(page.opened, []);
	});
});

describe("the model is a precondition", () => {
	test("refuses with wrong-model and sends nothing when the page is set to another model", async () => {
		const { page, composer, submit } = build({ model: "Gemini 3 Ultra" });

		const error = await rejection(driver(page).ask(question(), options()));

		assert.equal(error.code, ErrorCode.WrongModel);
		assert.match(error.detail, /Gemini 3 Ultra/);
		assert.equal(submit.clicks, 0);
		assert.deepEqual(composer.events, []);
		assert.equal(page.disposed.length, 1);
	});

	test("matches the label across case and whitespace, and reports what it read", async () => {
		const { page } = build({ model: "  gpt-6.1   sol " });

		const answer = await driver(page).ask(question(), options());

		assert.equal(answer.text, "The whole answer");
		assert.equal(answer.model, "gpt-6.1   sol");
	});
});

describe("completion is stable text, not a count", () => {
	test("returns the last text of an answer that stops changing", async () => {
		const { page } = build({
			stream: ["Half an answer", "Most of an answer", "The whole answer"],
		});

		const answer = await driver(page).ask(question(), options());

		assert.equal(answer.text, "The whole answer");
	});

	test("reads the answer this question caused, not the one already in the thread", async () => {
		const { page } = build({
			earlier: "An answer to an earlier question",
			stream: ["The new answer"],
		});

		const answer = await driver(page).ask(question(), options());

		assert.equal(answer.text, "The new answer");
	});

	test("reads the steps and sources of this turn, not the earlier turn's", async () => {
		const { page } = build({
			earlier: "An answer to an earlier question",
			earlierTrace: [
				{
					tagName: "LI",
					innerText: "Earlier step",
					attributes: { "data-depth": "0" },
				},
			],
			earlierSources: [
				{
					tagName: "A",
					innerText: "earlier",
					attributes: { href: "https://earlier.example.test/" },
				},
			],
			stream: ["The new answer"],
			trace: [
				{
					tagName: "LI",
					innerText: "This step",
					attributes: { "data-depth": "0" },
				},
			],
			sources: [
				{
					tagName: "A",
					innerText: "this",
					attributes: { href: "https://this.example.test/" },
				},
			],
		});

		const answer = await driver(page).ask(question(), options());

		assert.deepEqual(answer.trace, [
			{ depth: 0, label: "This step", detail: "" },
		]);
		assert.deepEqual(answer.sources, [
			{ url: "https://this.example.test/", label: "this" },
		]);
	});

	test("refuses with not-ready when the answer never stops changing", async () => {
		const { page } = build({ stall: true });

		const error = await rejection(
			driver(page).ask(question(), options(120)),
		);

		assert.equal(error.code, ErrorCode.NotReady);
		assert.match(error.message, /settle/);
		assert.equal(page.disposed.length, 1);
	});

	test("does not wait for ever on a page that ignores its deadline", async () => {
		const { page } = build();
		page.hang = true;

		const error = await rejection(
			driver(page).ask(question(), options(60)),
		);

		assert.equal(error.code, ErrorCode.NotReady);
		assert.match(error.message, /deadline/);
	});
});

describe("a collapsed answer is refused", () => {
	test("refuses with truncated when the answer hides behind a Read more control", async () => {
		const { page } = build({
			stream: ["The first paragraph"],
			collapsed: true,
		});

		const error = await rejection(driver(page).ask(question(), options()));

		assert.equal(error.code, ErrorCode.Truncated);
		assert.match(error.detail, /Read more/);
		assert.match(error.detail, /19 characters/);
		assert.equal(page.disposed.length, 1);
	});
});

describe("the page", () => {
	test("reports not-ready when the page cannot be opened", async () => {
		const { page } = build();
		page.openFailure = new Error("connect ECONNREFUSED 127.0.0.1:9222");

		const error = await rejection(driver(page).ask(question(), options()));

		assert.equal(error.code, ErrorCode.NotReady);
		assert.match(error.detail, /ECONNREFUSED/);
		assert.equal(page.disposed.length, 0);
	});

	test("does not wait for ever on a page that never opens", async () => {
		const { page } = build();
		page.hangOnOpen = true;

		const error = await rejection(
			driver(page).ask(question(), options(60)),
		);

		assert.equal(error.code, ErrorCode.NotReady);
		assert.match(error.message, /deadline/);
		assert.equal(page.disposed.length, 0);
	});

	test("reports not-ready when the page fails while it is being read", async () => {
		const { page } = build();
		page.closed = true;

		const error = await rejection(driver(page).ask(question(), options()));

		assert.equal(error.code, ErrorCode.NotReady);
		assert.match(error.detail, /gone/);
		assert.equal(page.disposed.length, 1);
	});

	test("refuses to press a send control that never enables", async () => {
		const { page, submit } = build({ disabledSubmit: true });

		const error = await rejection(driver(page).ask(question(), options()));

		assert.equal(error.code, ErrorCode.NotReady);
		assert.match(error.detail, /disabled/);
		assert.equal(submit.clicks, 0);
	});

	test("reports not-ready before opening anything when the signal already fired", async () => {
		const { page } = build();
		const controller = new AbortController();
		controller.abort();

		const error = await rejection(
			driver(page).ask(question(), options(400, controller.signal)),
		);

		assert.equal(error.code, ErrorCode.NotReady);
		assert.match(error.message, /cancelled/);
		assert.deepEqual(page.opened, []);
	});

	test("reports not-ready when the caller gives up while the answer is arriving", async () => {
		const { page } = build({ stall: true });
		const controller = new AbortController();
		setTimeout(() => controller.abort(), 10);

		const error = await rejection(
			driver(page).ask(question(), options(5_000, controller.signal)),
		);

		assert.equal(error.code, ErrorCode.NotReady);
		assert.match(error.message, /cancelled/);
		assert.equal(page.disposed.length, 1);
	});

	test("reports a release that hangs instead of waiting for it", async () => {
		const { page } = build();
		page.hangOnDispose = true;
		const reported: { target: Target; error: unknown }[] = [];

		const answer = await driver(page, {
			disposeTimeoutMs: 20,
			report: (failure) => reported.push(failure),
		}).ask(question(), options());

		assert.equal(answer.text, "The whole answer");
		assert.equal(reported.length, 1);
		assert.ok(reported[0].error instanceof ChatError);
		assert.equal(reported[0].error.code, ErrorCode.NotReady);
	});
});

describe("a provider that answers the wrong shape", () => {
	/**
	 * The manifest has four codes and no fifth, so a provider's own defect has to
	 * arrive as one of them rather than as a `TypeError` thrown out of the port -
	 * and a `trace` that is not a list must not be handed back as if it were one.
	 */
	test("refuses a mode that is not text with wrong-mode, and sends nothing", async () => {
		const { page, composer, submit } = build();
		page.rewrite = (expression, value) =>
			expression.includes("aria-pressed") ? 42 : value;

		const error = await rejection(driver(page).ask(question(), options()));

		assert.equal(error.code, ErrorCode.WrongMode);
		assert.match(error.detail, /42/);
		assert.equal(submit.clicks, 0);
		assert.deepEqual(composer.events, []);
		assert.equal(page.disposed.length, 1);
	});

	test("refuses a thread read that is not a thread with not-ready", async () => {
		const { page } = build();
		let reads = 0;
		page.rewrite = (expression, value) => {
			if (!expression.includes("count: 0")) return value;
			reads += 1;
			// The pre-send read correlates the turn; the post-send one is the lie.
			return reads === 1 ? value : null;
		};

		const error = await rejection(driver(page).ask(question(), options()));

		assert.equal(error.code, ErrorCode.NotReady);
		assert.match(error.detail, /instead of a thread/);
		assert.equal(page.disposed.length, 1);
	});

	test("refuses a trace that is not a list of steps with not-ready", async () => {
		const { page } = build();
		page.rewrite = (expression, value) =>
			expression.includes("data-depth") ? "Searching" : value;

		const error = await rejection(driver(page).ask(question(), options()));

		assert.equal(error.code, ErrorCode.NotReady);
		assert.match(error.detail, /instead of a list of steps/);
		assert.equal(page.disposed.length, 1);
	});

	test("refuses sources that are not a list with not-ready", async () => {
		const { page } = build();
		page.rewrite = (expression, value) =>
			expression.includes("a[href") ? "https://example.test/" : value;

		const error = await rejection(driver(page).ask(question(), options()));

		assert.equal(error.code, ErrorCode.NotReady);
		assert.match(error.detail, /instead of a list of sources/);
		assert.equal(page.disposed.length, 1);
	});
});

describe("every failure stays inside the four codes", () => {
	/**
	 * A consumer narrows on `ChatError.code`, so a `TypeError` leaving the port
	 * would leave it with no branch to take. The manifest has no "the caller was
	 * wrong" code, so a call the driver cannot even begin is `not-ready`: nothing
	 * was asked, and therefore no answer arrived.
	 */
	test("refuses a call with no options as not-ready, not a TypeError", async () => {
		const { page } = build();

		const error = await rejection(
			driver(page).ask(
				question(),
				undefined as unknown as OperationOptions,
			),
		);

		assert.equal(error.code, ErrorCode.NotReady);
		assert.deepEqual(page.opened, []);
	});

	test("refuses a call with no question as not-ready, not a TypeError", async () => {
		const { page } = build();

		const error = await rejection(
			driver(page).ask(undefined as unknown as Question, options()),
		);

		assert.equal(error.code, ErrorCode.NotReady);
		assert.deepEqual(page.opened, []);
	});
});

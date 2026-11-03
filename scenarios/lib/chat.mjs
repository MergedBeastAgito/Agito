/**
 * The TypeScript side of the differential run for `agentic.chat.ask`.
 *
 * A scenario is a fixture (one page, one question, one driver) and a list of
 * steps. The fake below is the only thing the driver can reach, and it records
 * every interaction in `effects`, in order: that list is the protocol the
 * implementation spoke, and it is what makes an implementation that returns the
 * right value by the wrong route fail.
 *
 * The fake is the same shape as `Test/Chat.test.ts`'s: every expression the
 * driver sends is evaluated for real against a page-shaped object, with `new
 * Function` and a `document` that answers by selector. The driver's expressions
 * are recognised by comparing them to the driver's own builders - not by
 * matching a substring this file happened to expect - so a change to the
 * protocol breaks the fake instead of passing silently. That is also why the
 * Rust half of this run recognises them the same way, with
 * `AgenticChatDriver::Expressions::*`.
 *
 * Two things the fake owns rather than the page:
 *
 * - **The thread is a function of how often it was read.** A real answer grows
 *   between reads, so the fixture carries the sequence the page serves, one
 *   entry per read of the thread (the first read is the correlation read the
 *   driver takes before the send). Before each read the fake writes the next
 *   scripted text into the newest answer, exactly as `Test/Chat.test.ts`'s
 *   `serve(read)` does. Indexing on the read - not on wall-clock time - is what
 *   lets the Rust half, which has no DOM, serve the identical sequence.
 * - **The observation is written with sorted keys.** The Rust half serialises a
 *   `serde_json` map, which is ordered by key, and the two observation files
 *   have to be comparable as text. So `trace` and `sources` are re-keyed here in
 *   alphabetical order and the scenario file is written the same way.
 */
import {
	PressMode,
	PressSubmit,
	ReadAnswers,
	ReadCollapse,
	ReadMode,
	ReadModel,
	ReadSources,
	ReadTrace,
	TypeQuestion,
} from "../../packages/chat-driver/src/Expressions.ts";
import {
	ChatError,
	Create,
	DefaultSelectors,
} from "../../packages/chat-driver/src/index.ts";

const S = DefaultSelectors;

/** Fold a label the way the driver folds it, for the fake's own bookkeeping. */
const normalise = (value) =>
	typeof value === "string" ? value.replace(/\s+/g, " ").trim().toLowerCase() : "";

/**
 * Which of the driver's expressions the fake was handed, decided by the
 * driver's own builders. `null` means the driver speaks a protocol this fake
 * does not know, which is a failure of the run rather than of a case.
 */
const KindOf = (question, expression) => {
	const known = {
		mode: ReadMode(S),
		press: PressMode(S),
		model: ReadModel(S),
		composer: TypeQuestion(S, question),
		submit: PressSubmit(S),
		answers: ReadAnswers(S),
		collapse: ReadCollapse(S),
		trace: ReadTrace(S),
		sources: ReadSources(S),
	};
	for (const [kind, expected] of Object.entries(known))
		if (expression === expected) return kind;
	return null;
};

/** One element of the fake page. */
class FakeElement {
	#text;
	#reads = 0;

	constructor(options = {}) {
		this.tagName = options.tagName ?? "DIV";
		this.attributes = options.attributes ?? {};
		this.disabled = options.disabled ?? false;
		this.clicks = 0;
		this.events = [];
		this.value = options.value;
		this.onClick = options.onClick;
		// What the element's text is on its Nth read. A streaming answer is one
		// element whose serve returns a different text each time it is asked.
		this.serve = options.serve;
		this.children = new Map();
		this.#text = options.innerText ?? "";
	}

	get innerText() {
		const read = this.#reads;
		this.#reads += 1;
		return this.serve === undefined ? this.#text : this.serve(read);
	}

	set innerText(text) {
		this.#text = text;
	}

	getAttribute(name) {
		return Object.hasOwn(this.attributes, name) ? this.attributes[name] : null;
	}

	setAttribute(name, value) {
		this.attributes[name] = value;
	}

	click() {
		this.clicks += 1;
		this.onClick?.();
	}

	dispatchEvent(event) {
		this.events.push(event.type);
		return true;
	}

	querySelector(selector) {
		return this.querySelectorAll(selector)[0] ?? null;
	}

	querySelectorAll(selector) {
		return this.children.get(selector) ?? [];
	}

	/** Place child elements under a selector, the way a page would answer. */
	place(selector, ...elements) {
		this.children.set(selector, elements);
		return elements;
	}
}

/** The page's document, answering by selector. */
class FakeDocument {
	constructor() {
		this.elements = new Map();
	}

	querySelector(selector) {
		return this.querySelectorAll(selector)[0] ?? null;
	}

	querySelectorAll(selector) {
		return this.elements.get(selector) ?? [];
	}

	place(selector, ...elements) {
		this.elements.set(selector, elements);
		return elements;
	}
}

/** One step of a trace, as an element inside the newest answer. */
const traceElement = (step) =>
	new FakeElement({
		tagName: "LI",
		innerText: step.label ?? "",
		attributes: {
			"data-depth": String(step.depth ?? 0),
			"data-detail": step.detail ?? "",
		},
	});

/** One cited source, as an anchor inside the newest answer. */
const sourceElement = (source) =>
	new FakeElement({
		tagName: "A",
		innerText: source.label ?? "",
		attributes: { href: source.url ?? "" },
	});

/**
 * The page the scenario describes.
 *
 * Every scripted field is flat inside `page`, so a case's `before` patch merges
 * one level deep and means the same thing on both sides of the run.
 */
class FakePage {
	constructor(fixture, effects) {
		this.effects = effects;
		this.state = { ...fixture.page };
		this.question = fixture.question.text;
		this.reads = 0;
		this.pressed = normalise(this.state.mode) === "search" ? "search" : "computer";
		this.build();
	}

	/** Apply a step's `before` patch and rebuild the page from it. */
	patch(values) {
		Object.assign(this.state, values);
		this.pressed = normalise(this.state.mode) === "search" ? "search" : "computer";
		this.build();
	}

	/** Everything the page holds before the driver asks anything. */
	build() {
		const state = this.state;
		this.document = new FakeDocument();
		this.composer = null;

		// A page that cannot name its mode has no toggles to name it with: the
		// value it answers is the script's, whatever shape that is.
		if (typeof state.mode === "string") {
			const search = new FakeElement({
				tagName: "BUTTON",
				attributes: {
					"aria-label": "Search",
					"aria-pressed": this.pressed === "search" ? "true" : "false",
				},
			});
			const computer = new FakeElement({
				tagName: "BUTTON",
				attributes: {
					"aria-label": "Computer",
					"aria-pressed": this.pressed === "search" ? "false" : "true",
				},
			});
			const press = () => {
				// `stuck` is a toggle that is pressed and does not move: the page
				// accepts the click and stays in the mode it was in.
				this.pressed =
					normalise(state.modeAfterPress) === "search" ? "search" : "computer";
				search.setAttribute(
					"aria-pressed",
					this.pressed === "search" ? "true" : "false",
				);
				computer.setAttribute(
					"aria-pressed",
					this.pressed === "search" ? "false" : "true",
				);
			};
			search.onClick = press;
			computer.onClick = press;
			this.document.place(S.mode, search, computer);
		}

		if (typeof state.model === "string")
			this.document.place(
				S.model,
				new FakeElement({ tagName: "BUTTON", innerText: state.model }),
			);

		if (state.composer !== "missing") {
			this.composer = new FakeElement({ tagName: "TEXTAREA", value: "" });
			this.document.place(S.composer, this.composer);
		}

		if (state.submit !== "missing")
			this.document.place(
				S.submit,
				new FakeElement({
					tagName: "BUTTON",
					attributes: { "aria-label": "Submit" },
					disabled: state.submit === "disabled",
				}),
			);

		this.document.place(S.answer);
	}

	/** The thread as the page shows it after `count` answers, newest `text`. */
	setThread(count, text) {
		const state = this.state;
		const trace = Array.isArray(state.trace) ? state.trace : [];
		const sources = Array.isArray(state.sources) ? state.sources : [];
		const earlierTrace = Array.isArray(state.earlierTrace) ? state.earlierTrace : [];
		const earlierSources = Array.isArray(state.earlierSources)
			? state.earlierSources
			: [];
		const answers = [];
		for (let index = 0; index < count; index += 1) {
			const newest = index === count - 1;
			const answer = new FakeElement({
				tagName: "DIV",
				innerText: newest ? text : "an earlier answer",
				serve: newest ? () => text : undefined,
			});
			if (newest) {
				if (typeof state.collapse === "string")
					answer.place(
						S.collapse,
						new FakeElement({ tagName: "BUTTON", innerText: state.collapse }),
					);
				answer.place(S.trace, ...trace.map(traceElement));
				answer.place(S.sources, ...sources.map(sourceElement));
			} else {
				// An earlier turn's steps and citations stay with that turn.
				answer.place(S.trace, ...earlierTrace.map(traceElement));
				answer.place(S.sources, ...earlierSources.map(sourceElement));
			}
			answers.push(answer);
		}
		this.document.place(S.answer, ...answers);
	}

	/** What the page's thread holds on its `read`th read, counting from zero. */
	threadAt(read) {
		if (this.state.answersStall === true)
			return { count: 1, text: `still writing ${read}` };
		const reads = this.state.answersReads ?? [];
		if (reads.length === 0) return { count: 0, text: "" };
		return reads[Math.min(read, reads.length - 1)];
	}

	/** Evaluate one of the driver's expressions against the page for real. */
	run(expression) {
		const fn = new Function("document", `return (${expression});`);
		return fn(this.document);
	}

	async open(url, _options) {
		this.effects.push(`open:${url}`);
		if (this.state.open === "hang") return new Promise(() => {});
		if (this.state.open === "throw") {
			this.effects.push(`failed:open:${this.state.openFailure}`);
			throw new Error(this.state.openFailure);
		}
		this.effects.push(`opened:${url}`);
		return { url };
	}

	async evaluate(_target, expression, _options) {
		const kind = KindOf(this.question, expression);
		if (kind === null)
			throw new Error(
				`the page was asked something this fake does not know: ${expression.slice(0, 80)}`,
			);
		this.effects.push(`evaluate:${kind}`);
		const state = this.state;

		if (state.evaluate === "hang") return new Promise(() => {});
		if (state.evaluate === "throw") {
			this.effects.push(`failed:${kind}:${state.evaluateFailure}`);
			throw new Error(state.evaluateFailure);
		}

		// A provider that answers the wrong shape is answered as it spoke: the
		// driver's rules have to refuse it, not this fake.
		if (kind === "mode" && typeof state.mode !== "string") return state.mode;
		if (kind === "model" && typeof state.model !== "string") return state.model;
		if (kind === "collapse" && state.collapse !== null && typeof state.collapse !== "string")
			return state.collapse;
		if (kind === "trace" && !Array.isArray(state.trace)) return state.trace;
		if (kind === "sources" && !Array.isArray(state.sources)) return state.sources;

		if (kind === "answers") {
			const read = this.reads;
			this.reads += 1;
			const at = read + 1;
			if (state.answersHangFrom === at) return new Promise(() => {});
			if (state.answersFailFrom === at) {
				this.effects.push(`failed:answers:${state.answersFailMessage}`);
				throw new Error(state.answersFailMessage);
			}
			const value = this.threadAt(read);
			if (typeof value !== "object" || value === null) return value;
			this.setThread(value.count, value.text ?? "");
		}

		const value = this.run(expression);

		// What the page observed, not what the driver asked for: the composer is
		// read back after the expression ran, and the send control reports what
		// it actually did.
		if (kind === "composer" && value === true && this.composer !== null)
			this.effects.push(`written:${this.composer.value}`);
		if (kind === "submit" && value === "sent") this.effects.push("sent");
		if (kind === "submit" && value === "disabled")
			this.effects.push("refused:submit:disabled");

		return value;
	}

	async dispose(target) {
		this.effects.push(`release:${target.url}`);
		if (this.state.release === "hang") return new Promise(() => {});
	}
}

/** The caller's signal, as the fixture describes it. */
const signalOf = (kind) => {
	if (kind === "absent") return { signal: undefined, clear: () => {} };
	const controller = new AbortController();
	if (kind === "aborted") controller.abort();
	let timer;
	if (typeof kind === "string" && kind.startsWith("after:"))
		timer = setTimeout(
			() => controller.abort(),
			Number(kind.slice("after:".length)),
		);
	return {
		signal: controller.signal,
		clear: () => {
			if (timer !== undefined) clearTimeout(timer);
		},
	};
};

/** Run one chat case and answer what it did. */
export const runChat = async (entry) => {
	const effects = [];
	const fixture = entry.fixture;
	const page = new FakePage(fixture, effects);
	const { signal, clear } = signalOf(fixture.driver.signal);
	let reports = 0;

	const driver = Create(page, {
		url: fixture.url,
		pollMs: fixture.driver.pollMs,
		disposeTimeoutMs: fixture.driver.disposeTimeoutMs,
		report: (failure) => {
			effects.push(`report:${failure.error?.code ?? "unknown"}`);
			reports += 1;
		},
	});

	const question = {
		text: fixture.question.text,
		mode: fixture.question.mode,
		model: fixture.question.model,
	};
	const options =
		signal === undefined
			? { timeoutMs: fixture.driver.timeoutMs }
			: { timeoutMs: fixture.driver.timeoutMs, signal };

	let result = { kind: "void" };
	for (const one of entry.steps) {
		if (one.before !== undefined && one.before.page !== undefined)
			page.patch(one.before.page);
		if (one.op !== "ask") throw new Error(`unknown chat step ${JSON.stringify(one.op)}`);
		try {
			const answer = await driver.ask(question, options);
			result = {
				kind: "answer",
				text: answer.text,
				trace: answer.trace.map((step) => ({
					depth: step.depth,
					detail: step.detail,
					label: step.label,
				})),
				sources: answer.sources.map((source) => ({
					label: source.label,
					url: source.url,
				})),
				model: answer.model,
				mode: answer.mode,
			};
		} catch (error) {
			if (!(error instanceof ChatError))
				throw new Error(
					`${entry.id} rejected with something that is not a ChatError: ${String(error)}`,
				);
			result = {
				kind: "error",
				code: error.code,
				message: error.message,
				detail: error.detail,
			};
			break;
		}
	}
	clear();
	return { result, effects, reports };
};

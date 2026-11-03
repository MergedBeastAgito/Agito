/**
 * The timing scenarios, generated rather than hand-written.
 *
 * Cancellation mid-operation, deadline expiry, transport death mid-operation and
 * the stability threshold crossed on the third read are the four cases where a
 * hand-written file would encode one particular budget and one particular moment,
 * and where the interesting question is the *rule* rather than the number. So
 * they are produced by a loop over (phase x event), each case carrying the
 * parameters that produced it, and the output is committed so both
 * implementations can read it without running this file.
 *
 * `node scenarios/generate.mjs` rewrites `scenarios/generated/`. `run.mjs`
 * regenerates and refuses to compare if the committed files have drifted from
 * what this file produces, because a generated file that nobody regenerates is a
 * hand-written file with extra steps.
 */
import { writeFileSync, mkdirSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const Here = dirname(fileURLToPath(import.meta.url));

/** The page every browse scenario is bound to. */
const Endpoint = "ws://127.0.0.1:9222/devtools/page/1";
const Url = "https://example.test/";

/**
 * The browse timing matrix.
 *
 * `phase` is the operation that is cut short; `event` is what cuts it short. A
 * deadline is silence, a cancellation is the caller, and a death is the socket
 * going away under a call that had already started - three different facts the
 * port must not collapse into one code.
 */
const browseTiming = [
	{
		id: "generated/open-deadline",
		typescript: "rejects with timeout when the launcher never answers",
		rust: "fails_with_timeout_when_the_launcher_never_answers",
		phase: "open",
		event: "deadline",
		budgetMs: 40,
		fixture: { launcher: { open: "hang" } },
		steps: [{ op: "open", url: Url, timeoutMs: 40 }],
		expect: {
			result: { kind: "error", code: "timeout", message: "deadline" },
			effects: [`launch:${Url}`],
			reports: 0,
		},
	},
	{
		id: "generated/open-cancel",
		typescript: "rejects with timeout when the caller's signal fires",
		rust: "fails_with_timeout_when_the_callers_signal_fires",
		phase: "open",
		event: "cancellation",
		budgetMs: 5000,
		fixture: { launcher: { open: "hang" } },
		steps: [{ op: "open", url: Url, timeoutMs: 5000, signal: "after:5" }],
		expect: {
			result: { kind: "error", code: "timeout", message: "cancelled" },
			effects: [`launch:${Url}`],
			reports: 0,
		},
	},
	{
		id: "generated/evaluate-deadline",
		typescript: "rejects with timeout when the page never answers",
		rust: "fails_with_timeout_when_the_page_never_answers",
		phase: "evaluate",
		event: "deadline",
		budgetMs: 40,
		fixture: { transport: { behaviour: "hang" } },
		steps: [
			{ op: "open", url: Url, timeoutMs: 40 },
			{ op: "evaluate", expression: "document.title", timeoutMs: 40 },
		],
		expect: {
			result: { kind: "error", code: "timeout", message: "deadline" },
			effects: [`launch:${Url}`, `handed:${Endpoint}`, "evaluate:document.title"],
			reports: 0,
		},
	},
	{
		id: "generated/evaluate-cancel",
		typescript: "rejects with timeout when the signal fires mid-flight",
		rust: "fails_with_timeout_when_the_signal_fires_mid_flight",
		phase: "evaluate",
		event: "cancellation",
		budgetMs: 5000,
		fixture: { transport: { behaviour: "hang" } },
		steps: [
			{ op: "open", url: Url, timeoutMs: 40 },
			{ op: "evaluate", expression: "document.title", timeoutMs: 5000, signal: "after:5" },
		],
		expect: {
			result: { kind: "error", code: "timeout", message: "cancelled" },
			effects: [`launch:${Url}`, `handed:${Endpoint}`, "evaluate:document.title"],
			reports: 0,
		},
	},
	{
		id: "generated/evaluate-death",
		typescript: "rejects with browser-unavailable when the transport closed during the call",
		rust: "fails_with_browser_unavailable_when_the_transport_closed_during_the_call",
		phase: "evaluate",
		event: "death",
		budgetMs: 40,
		fixture: { transport: { behaviour: "dead" } },
		steps: [
			{ op: "open", url: Url, timeoutMs: 40 },
			{ op: "evaluate", expression: "document.title", timeoutMs: 40 },
		],
		expect: {
			result: {
				kind: "error",
				code: "browser-unavailable",
				message: "closed while the page was being asked",
			},
			effects: [
				`launch:${Url}`,
				`handed:${Endpoint}`,
				"evaluate:document.title",
				`refused:evaluate:${Endpoint}`,
			],
			reports: 0,
		},
	},
	{
		id: "generated/dispose-hang",
		typescript: "reports a release that hangs instead of waiting forever",
		rust: "reports_a_release_that_hangs_instead_of_waiting_for_it",
		phase: "dispose",
		event: "deadline",
		budgetMs: 20,
		fixture: { transport: { hangOnClose: true }, session: { disposeTimeoutMs: 20 } },
		steps: [
			{ op: "open", url: Url, timeoutMs: 40 },
			{ op: "dispose", target: "last" },
		],
		expect: {
			result: { kind: "void" },
			effects: [
				`launch:${Url}`,
				`handed:${Endpoint}`,
				`close:${Endpoint}`,
				`report:${Endpoint}`,
			],
			reports: 1,
		},
	},
];

/** The fixture every browse scenario starts from. */
const browseFixture = {
	endpoint: Endpoint,
	transport: {
		behaviour: "answer",
		answer: null,
		closed: false,
		closeFailure: null,
		hangOnClose: false,
	},
	launcher: { open: "answer", failure: null, deadOnArrival: false },
	session: { disposeTimeoutMs: 40, report: "record" },
};

/** The chat product every chat scenario is bound to. */
const ChatUrl = "https://chat.example.test/";
const ChatQuestion = "What is the smallest portable workflow for asking a web chat UI?";

/** The effects of a question that was answered after `reads` reads of it. */
const chatAnswered = (reads) => [
	`open:${ChatUrl}`,
	`opened:${ChatUrl}`,
	"evaluate:mode",
	"evaluate:model",
	"evaluate:answers",
	"evaluate:composer",
	`written:${ChatQuestion}`,
	"evaluate:submit",
	"sent",
	...Array.from({ length: reads }, () => "evaluate:answers"),
	"evaluate:collapse",
	"evaluate:trace",
	"evaluate:sources",
	`release:${ChatUrl}`,
];

/**
 * The chat timing matrix.
 *
 * Same shape as the browse one, and the same reason: a hand-written case would
 * encode one budget and one moment, while the interesting question is the rule.
 * `phase` is the part of the call that is cut short; `event` is what cuts it
 * short. The four the port turns on are a cancellation, a deadline, a page that
 * dies under a read, and the stability threshold itself - the fifth is the
 * release, which is the only part of the call a caller never sees fail.
 *
 * A generated case names no test: three of them restate a rule a hand-written
 * case already rests on, and a test maps to one scenario, not two. What they add
 * is the matrix - the same rule at a different phase, and a read count pinned
 * where the hand-written case can only pin the code.
 */
const chatTiming = [
	{
		id: "generated/chat-answer-cancel",
		typescript: "reports not-ready when the caller gives up while the answer is arriving",
		rust: "reports_not_ready_when_the_caller_gives_up_while_the_answer_is_arriving",
		phase: "answer",
		event: "cancellation",
		budgetMs: 3000,
		fixture: {
			page: { answersHangFrom: 2 },
			driver: { pollMs: 0, timeoutMs: 3000, disposeTimeoutMs: 40, signal: "after:3" },
		},
		expect: {
			result: { kind: "error", code: "not-ready", message: "cancelled" },
			effects: [
				`open:${ChatUrl}`,
				`opened:${ChatUrl}`,
				"evaluate:mode",
				"evaluate:model",
				"evaluate:answers",
				"evaluate:composer",
				`written:${ChatQuestion}`,
				"evaluate:submit",
				"sent",
				"evaluate:answers",
				`release:${ChatUrl}`,
			],
			reports: 0,
		},
	},
	{
		id: "generated/chat-answer-deadline",
		typescript: "refuses with not-ready when the answer never stops changing",
		rust: "refuses_with_not_ready_when_the_answer_never_stops_changing",
		phase: "answer",
		event: "deadline",
		budgetMs: 40,
		fixture: {
			page: {
				answersReads: [
					{ count: 0, text: "" },
					{ count: 1, text: "Half an answer" },
				],
				answersHangFrom: 3,
			},
			driver: { pollMs: 0, timeoutMs: 40, disposeTimeoutMs: 40, signal: "fresh" },
		},
		expect: {
			result: { kind: "error", code: "not-ready", message: "deadline" },
			note: "A partial answer is in hand when the budget runs out - `Half an answer` - and the port says the answer did not arrive. A driver that returned what it had would answer here instead of failing.",
			effects: [
				`open:${ChatUrl}`,
				`opened:${ChatUrl}`,
				"evaluate:mode",
				"evaluate:model",
				"evaluate:answers",
				"evaluate:composer",
				`written:${ChatQuestion}`,
				"evaluate:submit",
				"sent",
				"evaluate:answers",
				"evaluate:answers",
				`release:${ChatUrl}`,
			],
			reports: 0,
		},
	},
	{
		id: "generated/chat-answer-death",
		typescript: "reports not-ready when the page fails while it is being read",
		rust: "reports_not_ready_when_the_page_fails_while_it_is_being_read",
		phase: "answer",
		event: "death",
		budgetMs: 2000,
		fixture: {
			page: {
				answersReads: [
					{ count: 0, text: "" },
					{ count: 1, text: "Half an answer" },
				],
				answersFailFrom: 3,
				answersFailMessage: "the page is gone",
			},
			driver: { pollMs: 0, timeoutMs: 2000, disposeTimeoutMs: 40, signal: "fresh" },
		},
		expect: {
			result: {
				kind: "error",
				code: "not-ready",
				message: "could not be read from the page",
				observed: "the page is gone",
			},
			effects: [
				`open:${ChatUrl}`,
				`opened:${ChatUrl}`,
				"evaluate:mode",
				"evaluate:model",
				"evaluate:answers",
				"evaluate:composer",
				`written:${ChatQuestion}`,
				"evaluate:submit",
				"sent",
				"evaluate:answers",
				"evaluate:answers",
				"failed:answers:the page is gone",
				`release:${ChatUrl}`,
			],
			reports: 0,
		},
	},
	{
		id: "generated/chat-answer-settles",
		typescript: "returns the last text of an answer that stops changing",
		rust: "returns_the_last_text_of_an_answer_that_stops_changing",
		phase: "answer",
		event: "stability",
		budgetMs: 2000,
		fixture: {
			driver: { pollMs: 0, timeoutMs: 2000, disposeTimeoutMs: 40, signal: "fresh" },
		},
		expect: {
			result: {
				kind: "answer",
				text: "The whole answer",
				model: "GPT-6.1 Sol",
				mode: "search",
				trace: [],
				sources: [],
			},
			note: "Exactly five reads of the thread: one to correlate the turn and four of the answer. The fourth is the third read that agreed with the one before it, so a threshold of one would settle two reads earlier and a threshold of four would never settle at all.",
			effects: chatAnswered(4),
			reports: 0,
		},
	},
	{
		id: "generated/chat-release-deadline",
		typescript: "reports a release that hangs instead of waiting for it",
		rust: "reports_a_release_that_hangs_instead_of_waiting_for_it",
		phase: "release",
		event: "deadline",
		budgetMs: 20,
		fixture: {
			page: { release: "hang" },
			driver: { pollMs: 0, timeoutMs: 400, disposeTimeoutMs: 20, signal: "fresh" },
		},
		expect: {
			result: {
				kind: "answer",
				text: "The whole answer",
				model: "GPT-6.1 Sol",
				mode: "search",
				trace: [],
				sources: [],
			},
			effects: [...chatAnswered(4), "report:not-ready"],
			reports: 1,
		},
	},
];

/** The fixture every chat scenario starts from. */
const chatFixture = {
	url: ChatUrl,
	page: {
		open: "answer",
		openFailure: "connect ECONNREFUSED 127.0.0.1:9222",
		evaluate: "answer",
		evaluateFailure: "the page is gone",
		mode: "Search",
		modeAfterPress: "Search",
		model: "GPT-6.1 Sol",
		composer: "textarea",
		submit: "sent",
		answersReads: [
			{ count: 0, text: "" },
			{ count: 1, text: "The whole answer" },
		],
		answersStall: false,
		answersHangFrom: null,
		answersFailFrom: null,
		answersFailMessage: "the page is gone",
		collapse: null,
		trace: [],
		sources: [],
		earlierTrace: [],
		earlierSources: [],
		release: "answer",
	},
	question: { text: ChatQuestion, mode: "search", model: "GPT-6.1 Sol" },
	driver: { pollMs: 0, disposeTimeoutMs: 40, timeoutMs: 400, signal: "fresh" },
};

/** A generated case, in the shape the scenario files use. */
const browseCase = (one) => ({
	id: one.id,
	typescript: one.typescript,
	rust: one.rust,
	generated: {
		by: "scenarios/generate.mjs",
		phase: one.phase,
		event: one.event,
		budgetMs: one.budgetMs,
	},
	fixture: one.fixture,
	steps: one.steps,
	expect: one.expect,
});

/** The same, for a capability whose call is one question rather than a step list. */
const chatCase = (one) => ({
	id: one.id,
	typescript: one.typescript,
	rust: one.rust,
	generated: {
		by: "scenarios/generate.mjs",
		phase: one.phase,
		event: one.event,
		budgetMs: one.budgetMs,
	},
	fixture: one.fixture,
	steps: [{ op: "ask" }],
	expect: one.expect,
});

/** Every generated document, by the path it is written to. */
export const documents = () => ({
	"generated/browser-cdp.timing.json": {
		$comment:
			"Generated by scenarios/generate.mjs from the (phase x event) matrix. Do not edit: edit the matrix. Both implementations read this file.",
		capability: "agentic.browse",
		version: 1,
		fixture: browseFixture,
		cases: browseTiming.map(browseCase),
	},
	"generated/chat-driver.timing.json": {
		$comment:
			"Generated by scenarios/generate.mjs from the chat (phase x event) matrix. Do not edit: edit the matrix. Both implementations read this file.",
		capability: "agentic.chat.ask",
		version: 1,
		fixture: chatFixture,
		cases: chatTiming.map(chatCase),
	},
});

/** Write every generated document, and answer the paths written. */
export const write = () => {
	const written = [];
	for (const [name, document] of Object.entries(documents())) {
		const path = join(Here, name);
		mkdirSync(dirname(path), { recursive: true });
		writeFileSync(path, `${JSON.stringify(document, null, "\t")}\n`, "utf8");
		written.push(path);
	}
	return written;
};

if (process.argv[1] === fileURLToPath(import.meta.url)) {
	for (const path of write()) console.log(`wrote ${path}`);
}

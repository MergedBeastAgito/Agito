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
			effects: [`launch:${Url}`, `handed:${Endpoint}`, `close:${Endpoint}`, `report:${Endpoint}`],
			reports: 1,
		},
	},
];

/** The fixture every browse scenario starts from. */
const browseFixture = {
	endpoint: Endpoint,
	transport: { behaviour: "answer", answer: null, closed: false, closeFailure: null, hangOnClose: false },
	launcher: { open: "answer", failure: null, deadOnArrival: false },
	session: { disposeTimeoutMs: 40, report: "record" },
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

/**
 * Watch a chat page and report only what changed.
 *
 * The problem this solves: after a send, the script has no way to know whether the
 * answer is streaming, finished, refused, or still waiting for an attachment - so
 * it either sleeps a fixed time or reads a transcript it does not need. This
 * installs a MutationObserver over the composer and the answer, keeps a small
 * delta log in the page, and prints one line per change until the page settles.
 *
 *   node observe.mjs <ws-url> [--interval 500] [--idle 4] [--timeout 240] [--json]
 *
 * Exit 0 when the page settled, 1 when the timeout fired first, 2 when it could
 * not run. `--json` prints the final state as one object instead of a log.
 */
const Args = process.argv.slice(2);
const WsUrl = Args[0];
const At = (Name, Fallback) => {
	const Index = Args.indexOf(Name);
	return Index === -1 ? Fallback : Number(Args[Index + 1]);
};
const Interval = At("--interval", 500);
const Idle = At("--idle", 4);
const Timeout = At("--timeout", 240);
const AsJson = Args.includes("--json");

if (!WsUrl) {
	console.error("usage: node observe.mjs <ws-url> [--interval 500] [--idle 4] [--timeout 240] [--json]");
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

const Evaluate = async (Expression) => {
	const Result = await Send("Runtime.evaluate", { expression: Expression, returnByValue: true });
	if (Result.exceptionDetails) throw new Error(Result.exceptionDetails.text);
	return Result.result.value;
};

// The probe is idempotent: it installs once and reports the current state after.
const Probe = `(() => {
	if (window.__agenticObserve === undefined) {
		const State = { events: 0, last: "", installedAt: Date.now() };
		window.__agenticObserve = State;
		const Observe = new MutationObserver((Records) => {
			State.events += Records.length;
			State.last = Records[Records.length - 1].target?.nodeName ?? "";
		});
		Observe.observe(document.body, { childList: true, subtree: true, characterData: true, attributes: true, attributeFilter: ["aria-pressed", "aria-expanded", "disabled", "data-state"] });
	}

	const Text = document.body.innerText || "";
	const Composer = [...document.querySelectorAll('textarea, [contenteditable="true"]')].find((Element) => {
		const Box = Element.getBoundingClientRect();
		return Box.width > 0 && Box.height > 0;
	});
	const Submit = [...document.querySelectorAll("button")].find((Button) => (Button.getAttribute("aria-label") || "").toLowerCase() === "submit");
	const Mode = [...document.querySelectorAll("[data-command-menu-control] button")].find((Button) => (Button.getAttribute("aria-label") || "") === "Search");
	const Attachments = document.querySelectorAll('[class*="attachment"], [data-testid*="attachment"]').length;
	const Files = [...document.querySelectorAll("[class*=attachment]")].map((Element) => (Element.innerText || "").trim()).filter(Boolean);

	return JSON.stringify({
		chars: Text.length,
		events: window.__agenticObserve.events,
		composer: Composer === undefined ? null : (Composer.innerText || "").trim().length,
		submit: Submit === undefined ? null : !Submit.disabled,
		search: Mode === undefined ? null : Mode.getAttribute("aria-pressed") === "true",
		attachments: Attachments,
		files: Files.slice(0, 5),
	});
})()`;

let Previous = null;
let Quiet = 0;
let SawChange = false;
const Deadline = Date.now() + Timeout * 1000;
let Final = null;

while (Date.now() < Deadline) {
	await sleep(Interval);
	const Now = JSON.parse(await Evaluate(Probe));

	const Delta = Previous === null ? null : Now.chars - Previous.chars;
	const Changed =
		Previous === null ||
		Now.chars !== Previous.chars ||
		Now.attachments !== Previous.attachments ||
		Now.composer !== Previous.composer ||
		Now.submit !== Previous.submit ||
		Now.search !== Previous.search;

	if (Changed && !AsJson) {
		console.log(
			`chars ${Now.chars}${Delta === null ? "" : ` (${Delta >= 0 ? "+" : ""}${Delta})`}  composer ${Now.composer}  submit ${Now.submit}  search ${Now.search}  attachments ${Now.attachments}  events ${Now.events}`,
		);
	}

	// A page that has not changed at all since the watch began is a page whose
	// answer has not started, not a page that has settled. Settling before the
	// first change is how a watcher reports "done" on a question it never saw
	// answered.
	if (Changed) {
		SawChange = true;
		Quiet = 0;
	} else {
		Quiet += 1;
	}

	Previous = Now;
	Final = Now;
	if (SawChange && Quiet >= Idle && Now.composer === 0) break;
}

Socket.close();

if (Final === null) {
	console.error("blocker: no reading was taken");
	process.exit(2);
}

if (AsJson) console.log(JSON.stringify(Final));
const Settled = Quiet >= Idle;
process.exit(Settled ? 0 : 1);

declare const __PARAM_Binding__: string;
declare const __PARAM_Answer__: string;
declare const __PARAM_Quiet__: number;

(() => {
	const Prior = window["__Agito"];
	if (Prior !== undefined && typeof Prior.Disarm === "function") Prior.Disarm();

	const State = {
		Generation: (Prior === undefined ? 0 : Prior.Generation) + 1,
		Quiet: __PARAM_Quiet__,
		Armed: true,
		Bound: typeof window[__PARAM_Binding__] === "function",
		Busy: false,
		Offscreen: false,
		Chars: 0,
		Turn: "",
		At: Date.now(),
		Timer: 0,
		Mutations: 0,
		Stalls: 0,
		Disarm: null,
	};

	// The newest answer, and what it says. Every reading here is a read: the
	// agent never scrolls, focuses or otherwise moves the page it is in.
	const Newest = () => {
		const Answers = [...document.querySelectorAll(__PARAM_Answer__)];
		return Answers.length === 0 ? null : Answers[Answers.length - 1];
	};

	// The answer container. Its parent is the thread, which is what a new turn
	// is added to; with no answer yet there is no container, so the body is
	// watched until one appears.
	const Root = () => {
		const Tail = Newest();
		if (Tail === null || Tail.parentElement === null) return document.body;
		return Tail.parentElement;
	};

	const Snapshot = () => {
		const Tail = Newest();
		const Text = Tail === null ? "" : (Tail.innerText || "").trim();
		let Offscreen = true;
		if (Tail !== null) {
			const Rect = Tail.getBoundingClientRect();
			Offscreen = Rect.bottom <= 0 || Rect.top >= window.innerHeight;
		}
		return {
			Chars: [...Text].length,
			Mark: [...Text].length + ":" + [...Text].slice(0, 48).join(""),
			Offscreen: Offscreen,
		};
	};

	// The one call out of the page. A binding that is not there is not an
	// exception: it is the fact that this page cannot signal, which the caller
	// reads and answers by reading the thread instead.
	const Notify = (Event) => {
		const Call = window[__PARAM_Binding__];
		if (typeof Call !== "function") {
			State.Bound = false;
			return false;
		}
		State.Bound = true;
		const Seen = Snapshot();
		State.Chars = Seen.Chars;
		State.Turn = Seen.Mark;
		State.Offscreen = Seen.Offscreen;
		State.At = Date.now();
		try {
			Call(
				JSON.stringify({
					event: Event,
					turn: State.Turn,
					chars: State.Chars,
					offscreen: State.Offscreen,
					at: State.At,
					generation: State.Generation,
					mutations: State.Mutations,
				}),
			);
		} catch (Why) {
			State.Bound = false;
			return false;
		}
		return true;
	};

	// The container the viewport observer is watching. It is re-pointed when the
	// thread changes shape, because the answer's parent is not the same element
	// before an answer exists as after one does.
	const Observed = { Root: null };
	const Retarget = () => {
		const Container = Root();
		if (Observed.Root === Container) return;
		Observed.Root = Container;
		Intersects.disconnect();
		Intersects.observe(Container);
	};

	const Quiet = () => {
		State.Timer = 0;
		State.Busy = false;
		Retarget();
		Notify("quiet");
	};

	// Nothing has changed for `Quiet` milliseconds. The timer lives here, so it
	// fires at the moment the page actually stopped rather than at the next
	// tick of a poll interval.
	const Arm = () => {
		if (State.Timer !== 0) clearTimeout(State.Timer);
		State.Timer = setTimeout(Quiet, State.Quiet);
	};

	const Bump = () => {
		if (!State.Busy) {
			State.Busy = true;
			Notify("busy");
		}
		State.Mutations += 1;
		Arm();
	};

	// A record matters when it is inside the answer container, or when it
	// mounts or unmounts an answer. Anything else - a clock in the header, a
	// counter in a sidebar - is somebody else's animation and must not hold
	// this answer open.
	const Relevant = (Records) => {
		const Container = Root();
		for (const Record of Records) {
			if (Record.target === Container || Container.contains(Record.target)) return true;
			const Nodes = [...Record.addedNodes, ...Record.removedNodes];
			for (const Node of Nodes) {
				if (Node.nodeType !== 1) continue;
				if (Node.matches(__PARAM_Answer__)) return true;
				if (Node.querySelector(__PARAM_Answer__) !== null) return true;
			}
		}
		return Container === document.body;
	};

	const Mutations = new MutationObserver((Records) => {
		if (!Relevant(Records)) return;
		Retarget();
		const Seen = Snapshot();
		if (Seen.Offscreen !== State.Offscreen) {
			State.Offscreen = Seen.Offscreen;
			Notify("offscreen");
		}
		Bump();
	});
	Mutations.observe(document.body, { subtree: true, childList: true, characterData: true });

	// Layout growth a mutation record can miss: a virtualised thread
	// re-rendering its rows changes no text but does change the body's box.
	const Resizes = new ResizeObserver(() => Bump());
	Resizes.observe(document.body);

	// The only thing here that reads scroll state, and it never writes it. Its
	// job is to say whether the person has the answer in view, so the caller
	// can say `offscreen: true` instead of bringing the answer into view.
	const Intersects = new IntersectionObserver(
		(Entries) => {
			const Entry = Entries[Entries.length - 1];
			if (Entry === undefined) return;
			const Offscreen = !Entry.isIntersecting;
			if (Offscreen === State.Offscreen) return;
			State.Offscreen = Offscreen;
			Notify("offscreen");
		},
		{ threshold: [0, 0.01, 1] },
	);
	Observed.Root = Root();
	Intersects.observe(Observed.Root);

	const Disarm = () => {
		Mutations.disconnect();
		Resizes.disconnect();
		Intersects.disconnect();
		if (State.Timer !== 0) {
			clearTimeout(State.Timer);
			State.Timer = 0;
		}
		State.Armed = false;
	};

	State.Disarm = Disarm;
	window["__Agito"] = State;

	// A page that was already still before this call was armed still reports
	// itself: the first quiet window starts now rather than at the first
	// mutation, which on a finished answer never comes.
	Arm();

	return {
		armed: true,
		bound: State.Bound,
		generation: State.Generation,
		quiet: State.Quiet,
		answer: Newest() !== null,
	};
})();

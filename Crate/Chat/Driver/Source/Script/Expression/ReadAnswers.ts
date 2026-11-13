declare const __PARAM_Serialise__: (Root: Element | null) => string;
declare const __PARAM_Answer__: string;
declare const __PARAM_Busy__: string;
declare const __PARAM_Pattern__: RegExp | null;
declare const __PARAM_Turn__: string;
declare const __PARAM_Seen__: (Answer: Element) => string;

(() => {
	__PARAM_Serialise__; // A read is a read. Nothing here scrolls the page, presses its
	// scroll-to-end control, focuses anything or brings the tab forward: a
	// person may be looking at this thread, and text is readable through the
	// DOM whether or not it is on screen. Whether the newest answer is off
	// screen is reported instead of being corrected.
	const Answers = [...document.querySelectorAll(__PARAM_Answer__)];
	const Stopping = [...document.querySelectorAll(__PARAM_Busy__)].some(
		(Control) => Control.getClientRects().length > 0 && Control.disabled !== true,
	);
	const Pattern = __PARAM_Pattern__;
	const Unfinished = (Answer) => {
		if (Pattern === null) return false;
		const Turn = __PARAM_Turn__ === "" ? Answer : Answer.closest(__PARAM_Turn__);
		if (Turn === null) return true;
		return !Pattern.test((Turn.innerText || "").trim());
	};
	const Seen = __PARAM_Seen__;
	const Mark = (Text) => [...Text].length + ":" + [...Text].slice(0, 48).join("");
	const Offscreen = (Answer) => {
		if (Answer === undefined) return true;
		const Rect = Answer.getBoundingClientRect();
		return Rect.bottom <= 0 || Rect.top >= window.innerHeight;
	};
	if (Answers.length === 0)
		return { count: 0, text: "", marks: [], busy: Stopping, offscreen: true };
	const Last = Answers[Answers.length - 1];
	return {
		count: Answers.length,
		text: Seen(Last),
		marks: Answers.map((Answer) => Mark(Seen(Answer))),
		busy: Stopping || Unfinished(Last),
		offscreen: Offscreen(Last),
	};
})();

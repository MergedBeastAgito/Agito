declare const __PARAM_Serialise__: (Root: Element | null) => string;
declare const __PARAM_Scroll__: string;
declare const __PARAM_Answer__: string;
declare const __PARAM_Busy__: string;
declare const __PARAM_Pattern__: RegExp | null;
declare const __PARAM_Turn__: string;
declare const __PARAM_Seen__: (Answer: Element) => string;

(() => {
__PARAM_Serialise__	// A virtualised thread does not mount what is off-screen, so a read that
	// does not return to the end measures whichever turns happen to be
	// mounted - which is how an old answer gets read as a new one. The page's
	// own control is tried first; scrolling the last answer into view is the
	// fallback for a page that has no such control.
	const End = document.querySelector(__PARAM_Scroll__);
	if (End !== null && End.getClientRects().length > 0) End.click();
	const Answers = [...document.querySelectorAll(__PARAM_Answer__)];
	const Tail = Answers[Answers.length - 1];
	if (Tail !== undefined && Tail.getBoundingClientRect().bottom > window.innerHeight) Tail.scrollIntoView({block: "end", behavior: "instant" });
	const Stopping = [...document.querySelectorAll(__PARAM_Busy__)].some((Control) => Control.getClientRects().length > 0 && Control.disabled !== true);
	const Pattern = __PARAM_Pattern__;
	const Unfinished = (Answer) => {
		if (Pattern === null) return false;
		const Turn = __PARAM_Turn__ === '' ? Answer : Answer.closest(__PARAM_Turn__);
		if (Turn === null) return true;
		return !Pattern.test((Turn.innerText || "").trim());
	};
	const Seen = __PARAM_Seen__;
	const Mark = (Text) => [...Text].length + ":" + [...Text].slice(0, 48).join("");
	if (Answers.length === 0) return { count: 0, text: "", marks: [], busy: Stopping };
	const Last = Answers[Answers.length - 1];
	return { count: Answers.length, text: Seen(Last), marks: Answers.map((Answer) => Mark(Seen(Answer))), busy: Stopping || Unfinished(Last) };
})()
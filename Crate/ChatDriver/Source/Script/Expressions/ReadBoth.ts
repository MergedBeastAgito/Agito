/**
 * One read of the thread, rendered twice.
 *
 * The two readings of an answer - the page's rendered text and the page's own
 * Markdown - are each a whole read of the page. Asking for them in two calls
 * reads the page twice, and the second read can see a page that has changed
 * under it: an answer that streams, a source chip that appears, a control that
 * collapses the text. Two renderings of one answer then disagree about what it
 * said, and nothing in either of them says which reading is the older one.
 *
 * So this expression returns both from one snapshot. `text` is the rendered
 * text, `markdown` is `Serialise` of the same element, and both are taken from
 * `Last` inside one evaluate - the page cannot change between two statements of
 * the same synchronous function, which is what makes the pair describe one
 * moment rather than two that happen to agree.
 *
 * The text half is the reading every other expression uses, and deliberately:
 * the completion rule compares text to text, and the turn marks are computed
 * from the text, so a call that asks for both settles and correlates exactly as
 * a call that asks for text. A mark computed from the serialised rendering
 * would make every read of a thread look new to a driver correlating on text.
 *
 * This is a file of its own rather than a third parameter of `ReadAnswers.ts`
 * for one reason: the two single-format expressions are what a page already
 * receives, and an expression that gained a field would not be that expression.
 * What must not happen is the two files drifting, so the thread read below is
 * the same text `ReadAnswers.ts` carries, from its first line to the object
 * each one returns: the only difference is the object itself.
 */
declare const __PARAM_Serialise__: (Root: Element | null) => string;
declare const __PARAM_Scroll__: string;
declare const __PARAM_Answer__: string;
declare const __PARAM_Busy__: string;
declare const __PARAM_Pattern__: RegExp | null;
declare const __PARAM_Turn__: string;

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
	const Seen = (Answer) => (Answer.innerText || "").trim();
	const Mark = (Text) => [...Text].length + ":" + [...Text].slice(0, 48).join("");
	if (Answers.length === 0) return { count: 0, text: "", marks: [], busy: Stopping };
	const Last = Answers[Answers.length - 1];
	return { count: Answers.length, text: Seen(Last), markdown: Serialise(Last), marks: Answers.map((Answer) => Mark(Seen(Answer))), busy: Stopping || Unfinished(Last) };
})()

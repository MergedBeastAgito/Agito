(() => {
{{serialise}}	// A virtualised thread does not mount what is off-screen, so a read that
	// does not return to the end measures whichever turns happen to be
	// mounted - which is how an old answer gets read as a new one. The page's
	// own control is tried first; scrolling the last answer into view is the
	// fallback for a page that has no such control.
	const End = document.querySelector({{scroll}});
	if (End !== null && End.getClientRects().length > 0) End.click();
	const Answers = [...document.querySelectorAll({{answer}})];
	const Tail = Answers[Answers.length - 1];
	if (Tail !== undefined && Tail.getBoundingClientRect().bottom > window.innerHeight) Tail.scrollIntoView({block: "end", behavior: "instant" });
	const Stopping = [...document.querySelectorAll({{busy}})].some((Control) => Control.getClientRects().length > 0 && Control.disabled !== true);
	const Pattern = {{pattern}};
	const Unfinished = (Answer) => {
		if (Pattern === null) return false;
		const Turn = {{turn}} === '' ? Answer : Answer.closest({{turn}});
		if (Turn === null) return true;
		return !Pattern.test((Turn.innerText || "").trim());
	};
	const Seen = {{seen}};
	const Mark = (Text) => [...Text].length + ":" + [...Text].slice(0, 48).join("");
	if (Answers.length === 0) return { count: 0, text: "", marks: [], busy: Stopping };
	const Last = Answers[Answers.length - 1];
	return { count: Answers.length, text: Seen(Last), marks: Answers.map((Answer) => Mark(Seen(Answer))), busy: Stopping || Unfinished(Last) };
})()
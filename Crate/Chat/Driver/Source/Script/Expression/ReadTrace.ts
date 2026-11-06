declare const __PARAM_Answer__: string;
declare const __PARAM_Trace__: string;

(() => {
	const Answers = [...document.querySelectorAll(__PARAM_Answer__)];
	if (Answers.length === 0) return [];
	const Last = Answers[Answers.length - 1];
	return [...Last.querySelectorAll(__PARAM_Trace__)].map((Step) => ({
		depth: Number(Step.getAttribute("data-depth") || 0),
		label: (Step.innerText || "").trim(),
		detail: (Step.getAttribute("data-detail") || Step.getAttribute("aria-label") || "").trim(),
	}));
})();

(() => {
	const Answers = [...document.querySelectorAll({{answer}})];
	if (Answers.length === 0) return [];
	const Last = Answers[Answers.length - 1];
	return [...Last.querySelectorAll({{trace}})].map((Step) => ({
		depth: Number(Step.getAttribute("data-depth") || 0),
		label: (Step.innerText || "").trim(),
		detail: (Step.getAttribute("data-detail") || Step.getAttribute("aria-label") || "").trim(),
	}));
})()
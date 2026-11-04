(() => {
	const Answers = [...document.querySelectorAll({{answer}})];
	if (Answers.length === 0) return [];
	const Last = Answers[Answers.length - 1];
	return [...Last.querySelectorAll({{sources}})].map((Source) => ({
		url: Source.getAttribute("href") || "",
		label: (Source.innerText || "").trim(),
	}));
})()
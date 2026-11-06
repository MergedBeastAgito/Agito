declare const __PARAM_Answer__: string;
declare const __PARAM_Sources__: string;

(() => {
	const Answers = [...document.querySelectorAll(__PARAM_Answer__)];
	if (Answers.length === 0) return [];
	const Last = Answers[Answers.length - 1];
	return [...Last.querySelectorAll(__PARAM_Sources__)].map((Source) => ({
		url: Source.getAttribute("href") || "",
		label: (Source.innerText || "").trim(),
	}));
})();

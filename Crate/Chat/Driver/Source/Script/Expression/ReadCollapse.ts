declare const __PARAM_Answer__: string;
declare const __PARAM_Collapse__: string;

(() => {
	const Answers = [...document.querySelectorAll(__PARAM_Answer__)];
	if (Answers.length === 0) return null;
	const Last = Answers[Answers.length - 1];
	for (const Control of [...Last.querySelectorAll(__PARAM_Collapse__)]) {
		const Label = (Control.getAttribute("aria-label") || Control.innerText || "").trim();
		if (/^read more$/i.test(Label)) return Label;
	}
	return null;
})();

(() => {
	const Answers = [...document.querySelectorAll({{answer}})];
	if (Answers.length === 0) return null;
	const Last = Answers[Answers.length - 1];
	for (const Control of [...Last.querySelectorAll({{collapse}})]) {
		const Label = (Control.getAttribute("aria-label") || Control.innerText || "").trim();
		if (/^read more$/i.test(Label)) return Label;
	}
	return null;
})()
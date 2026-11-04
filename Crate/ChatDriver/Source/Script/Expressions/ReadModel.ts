(() => {
	const Label = document.querySelector({{model}});
	if (Label === null) return null;
	return (Label.innerText || Label.getAttribute("aria-label") || "").trim();
})()
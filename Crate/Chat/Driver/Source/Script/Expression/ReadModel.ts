declare const __PARAM_Model__: string;

(() => {
	const Label = document.querySelector(__PARAM_Model__);
	if (Label === null) return null;
	return (Label.innerText || Label.getAttribute("aria-label") || "").trim();
})()
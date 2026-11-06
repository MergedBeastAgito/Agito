declare const __PARAM_Mode__: string;

(() => {
	const Label = (Element) =>
		(Element.getAttribute("aria-label") || Element.innerText || "").trim().toLowerCase();
	const Toggles = [...document.querySelectorAll(__PARAM_Mode__)];
	const Wanted = Toggles.find((Toggle) => Label(Toggle) === "search");
	const Target = Wanted === undefined ? Toggles[0] : Wanted;
	if (Target === undefined) return false;
	Target.click();
	return true;
})();

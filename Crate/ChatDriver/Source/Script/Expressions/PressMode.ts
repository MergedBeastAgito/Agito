(() => {
	const Label = (Element) => (Element.getAttribute("aria-label") || Element.innerText || "").trim().toLowerCase();
	const Toggles = [...document.querySelectorAll({{mode}})];
	const Wanted = Toggles.find((Toggle) => Label(Toggle) === "search");
	const Target = Wanted === undefined ? Toggles[0] : Wanted;
	if (Target === undefined) return false;
	Target.click();
	return true;
})()
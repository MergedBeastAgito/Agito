(() => {
	const Label = (Element) => (Element.getAttribute("aria-label") || Element.innerText || "").trim().toLowerCase();
	const Pressed = [...document.querySelectorAll({{mode}})].filter((Toggle) => Toggle.getAttribute("aria-pressed") === "true");
	return Pressed.length === 1 ? Label(Pressed[0]) : null;
})()
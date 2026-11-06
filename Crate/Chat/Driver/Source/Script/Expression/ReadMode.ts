declare const __PARAM_Mode__: string;

(() => {
	const Label = (Element) => (Element.getAttribute("aria-label") || Element.innerText || "").trim().toLowerCase();
	const Pressed = [...document.querySelectorAll(__PARAM_Mode__)].filter((Toggle) => Toggle.getAttribute("aria-pressed") === "true");
	return Pressed.length === 1 ? Label(Pressed[0]) : null;
})()
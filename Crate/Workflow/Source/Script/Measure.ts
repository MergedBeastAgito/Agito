(() => {
	const Read = (Selector) => {
		const Element = document.querySelector(Selector);
		if (Element === null) return { selector: Selector, present: false, value: null };
		const Raw = Element.getAttribute("data-value");
		if (Raw !== null) {
			const Parsed = Number(Raw);
			return { selector: Selector, present: true, value: Number.isNaN(Parsed) ? Raw : Parsed };
		}
		return { selector: Selector, present: true, value: (Element.innerText || "").trim() };
	};
	return {{selectors}}.map(Read);
})()
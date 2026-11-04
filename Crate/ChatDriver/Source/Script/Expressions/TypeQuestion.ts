(() => {
	const Composer = document.querySelector({{composer}});
	if (Composer === null) return false;
	const Text = {{text}};
	Composer.focus();
	if (Composer.tagName === "TEXTAREA" || Composer.tagName === "INPUT") {
		const Proto = Composer.tagName === "TEXTAREA" ? HTMLTextAreaElement.prototype : HTMLInputElement.prototype;
		const Set = Object.getOwnPropertyDescriptor(Proto, "value").set;
		Set.call(Composer, Text);
		Composer.dispatchEvent(new Event("input", { bubbles: true }));
		Composer.dispatchEvent(new Event("change", { bubbles: true }));
		return true;
	}
	const Range = document.createRange();
	Range.selectNodeContents(Composer);
	const Selection = window.getSelection();
	Selection.removeAllRanges();
	Selection.addRange(Range);
	if (((Composer.innerText || "").trim() === "" || document.execCommand("delete")) && document.execCommand("insertText", false, Text)) return true;
	Composer.innerText = Text;
	Composer.dispatchEvent(new InputEvent("input", { bubbles: true, inputType: "insertText", data: Text }));
	return true;
})()
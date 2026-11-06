declare const __PARAM_Composer__: string;
declare const __PARAM_Text__: string;

(() => {
	const Composer = document.querySelector(__PARAM_Composer__);
	if (Composer === null) return "missing";
	const Text = __PARAM_Text__;
	Composer.focus();
	if (Composer.tagName === "TEXTAREA" || Composer.tagName === "INPUT") {
		const Proto = Composer.tagName === "TEXTAREA" ? HTMLTextAreaElement.prototype : HTMLInputElement.prototype;
		const Set = Object.getOwnPropertyDescriptor(Proto, "value").set;
		Set.call(Composer, Text);
		Composer.dispatchEvent(new Event("input", { bubbles: true }));
		Composer.dispatchEvent(new Event("change", { bubbles: true }));
		return "typed";
	}
	// No delete, and no select-all-then-delete either. `execCommand("delete")`
	// over a selected range answers `true` on an editor that keeps its own
	// document model and deletes nothing, so a "clear then insert" pair appends
	// the question to the draft it was supposed to replace. Measured on the
	// reference product: a composer holding 4,167 characters grew to 8,496 over
	// two calls while every delete reported success.
	//
	// The composer is emptied through the input domain before this runs, and
	// the read-back afterwards is what refuses an insert that did not land.
	// This only inserts, at the end of whatever the editor holds, so an editor
	// that did keep something is appended to deliberately rather than by
	// accident - and the read-back names that case.
	const Selection = window.getSelection();
	const Range = document.createRange();
	Range.selectNodeContents(Composer);
	Range.collapse(false);
	Selection.removeAllRanges();
	Selection.addRange(Range);
	if (!document.execCommand("insertText", false, Text)) return "refused";
	return "typed";
})()

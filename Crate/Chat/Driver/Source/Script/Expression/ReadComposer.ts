declare const __PARAM_Composer__: string;

(() => {
	const Composer = document.querySelector(__PARAM_Composer__);
	if (Composer === null) return null;
	const Kind = Composer.tagName === "TEXTAREA" ? "textarea" : Composer.tagName === "INPUT" ? "input" : "editable";
	const DOM = Kind === "editable" ? Composer.innerText || Composer.textContent || "" : Composer.value || "";
	// The editor's own document, when the editor can be asked for it. A
	// contenteditable is a rendering of a model it keeps elsewhere, and the two
	// are not the same string: the model is what an insert actually changed,
	// and the DOM is what the page drew afterwards. Reading only the DOM is how
	// a driver appends to a draft it believes is empty.
	let Model = null;
	let Source = "";
	if (Kind === "editable") {
		// ProseMirror hangs its view description on the node it renders into.
		const Description = Composer.pmViewDesc;
		if (Description && Description.view && Description.view.state && Description.view.state.doc) {
			Model = Description.view.state.doc.textContent;
			Source = "prosemirror";
		} else if (typeof Composer.__lexicalTextContent === "string") {
			// Lexical caches the plain text of its own root on that root.
			Model = Composer.__lexicalTextContent;
			Source = "lexical";
		} else if (Composer.__quill && typeof Composer.__quill.getText === "function") {
			Model = Composer.__quill.getText();
			Source = "quill";
		}
	} else {
		// A textarea's value is its model: there is nothing behind it.
		Model = DOM;
		Source = "plain";
	}
	return { kind: Kind, dom: DOM, model: Model, source: Source };
})()

declare const __PARAM_Composer__: string;
declare const __PARAM_Expected__: string;
declare const __PARAM_Submit__: string;

(() => {
	const Composer = document.querySelector(__PARAM_Composer__);
	// Whitespace is removed rather than collapsed, because an editor may not
	// keep a paragraph break at all: the text is inserted, the blocks render
	// without a separator, and `innerText` reads "them.The" where the caller
	// wrote "them. The". Collapsing to one space calls that a mismatch and
	// refuses to send a prompt that is exactly the one that was typed, which
	// is a false refusal about the editor's rendering rather than the text.
	const Strip = (Text) => (Text || "").replace(/\s+/g, "");
	const Wanted = Strip(__PARAM_Expected__);
	const Held = Composer === null ? null : Strip(Composer.innerText || Composer.value || "");
	if (Held !== null && Held !== Wanted) return "mismatch";
	const Submit = document.querySelector(__PARAM_Submit__);
	if (Submit === null) return "missing";
	if (Submit.disabled === true) return "disabled";
	Submit.click();
	return "sent";
})()
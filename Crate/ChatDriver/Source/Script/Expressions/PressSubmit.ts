declare const __PARAM_Composer__: string;
declare const __PARAM_Expected__: string;
declare const __PARAM_Submit__: string;

(() => {
	const Composer = document.querySelector(__PARAM_Composer__);
	const Strip = (Text) => (Text || "").replace(/\s+/g, " ").trim();
	const Wanted = Strip(__PARAM_Expected__);
	const Held = Composer === null ? null : Strip(Composer.innerText || Composer.value || "");
	if (Held !== null && Held !== Wanted) return "mismatch";
	const Submit = document.querySelector(__PARAM_Submit__);
	if (Submit === null) return "missing";
	if (Submit.disabled === true) return "disabled";
	Submit.click();
	return "sent";
})()
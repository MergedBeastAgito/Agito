(() => {
	const Composer = document.querySelector({{composer}});
	const Strip = (Text) => (Text || "").replace(/\s+/g, " ").trim();
	const Wanted = Strip({{expected}});
	const Held = Composer === null ? null : Strip(Composer.innerText || Composer.value || "");
	if (Held !== null && Held !== Wanted) return "mismatch";
	const Submit = document.querySelector({{submit}});
	if (Submit === null) return "missing";
	if (Submit.disabled === true) return "disabled";
	Submit.click();
	return "sent";
})()
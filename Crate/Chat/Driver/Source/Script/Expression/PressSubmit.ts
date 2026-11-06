declare const __PARAM_Composer__: string;
declare const __PARAM_Submit__: string;

(() => {
	// No comparison here. The composer is read back by the driver, which has
	// both readings - the editor's model and the DOM - and one place to compare
	// them; a second comparison in this expression would be a second rule, and
	// the two would disagree the first time one of them was changed.
	//
	// A disabled control is not clicked: the click would land on nothing, the
	// page would look sent, and the caller would wait out its whole deadline
	// for an answer nobody asked for.
	const Composer = document.querySelector(__PARAM_Composer__);
	if (Composer === null) return "missing";
	const Submit = document.querySelector(__PARAM_Submit__);
	if (Submit === null) return "missing";
	if (Submit.disabled === true) return "disabled";
	Submit.click();
	return "sent";
})();

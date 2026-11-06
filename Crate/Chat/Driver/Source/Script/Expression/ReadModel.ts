declare const __PARAM_Model__: string;

(() => {
	// The controls the caller says sit where the model is named. On a build
	// that renders several, the row's first popup control is not always the
	// model - a drafts control sits beside it and its label is a count - so
	// every candidate is kept and classified rather than the first one read.
	//
	// The classification is a heuristic and says so: a label that is a count,
	// or that names a control rather than a model, is not a model's name. What
	// is left is taken as the model. A row with nothing left answers `null`,
	// which the driver reports as `unknown` and proceeds with - a build it
	// cannot identify is a build it will not refuse a caller over.
	const Buttons = Array.from(document.querySelectorAll(__PARAM_Model__));
	const Controls = [
		"model",
		"draft",
		"drafts",
		"session",
		"options",
		"settings",
		"share",
		"more",
		"details",
		"filter",
		"search",
		"computer",
		"attach",
		"voice",
		"microphone",
		"account",
		"profile",
		"help",
		"upgrade",
		"new",
	];
	const Candidates = [];
	for (let Index = 0; Index < Buttons.length; Index++) {
		const Button = Buttons[Index];
		const Label = (Button.getAttribute("aria-label") || Button.innerText || "").trim();
		const Text = (Button.innerText || "").trim();
		const Popup = Button.getAttribute("aria-haspopup") || "";
		let Reason = "model";
		if (Label === "") Reason = "empty";
		else if (/^\d+([.,]\d+)?(\s+\w+)?$/.test(Label)) Reason = "count";
		else if (Controls.indexOf(Label.toLowerCase()) !== -1) Reason = "names-a-control";
		Candidates.push({ index: Index, label: Label, text: Text, popup: Popup, reason: Reason });
	}
	let Chosen = null;
	for (let Index = 0; Index < Candidates.length; Index++) {
		if (Candidates[Index].reason === "model") {
			Chosen = Index;
			break;
		}
	}
	return {
		chosen: Chosen,
		model: Chosen === null ? null : Candidates[Chosen].label,
		candidates: Candidates,
	};
})();

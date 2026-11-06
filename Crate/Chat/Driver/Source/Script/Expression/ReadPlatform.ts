(() => {
	// Which chord selects everything in this page. Read from the page rather
	// than from this process: a driver attached to somebody else's browser is
	// typing on the machine that browser runs on, which need not be the machine
	// the driver runs on.
	const Data = navigator.userAgentData;
	const Name = (Data && Data.platform) || navigator.platform || "";
	return { mac: /mac|iphone|ipad|ipod/i.test(Name) };
})();

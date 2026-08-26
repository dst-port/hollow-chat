const PRESENCE_URL = "http://127.0.0.1:47821/presence";

chrome.runtime.onMessage.addListener((message) => {
	if (!message || message.type !== "hollowchat-presence") return;

	fetch(PRESENCE_URL, {
		method: "POST",
		headers: { "content-type": "application/json" },
		body: JSON.stringify({
			source: message.source,
			title: message.title,
			subtitle: message.subtitle ?? null,
			playing: message.playing
		})
	}).catch(() => {
		// HollowChat isn't running, or the bridge isn't listening yet - fine, just skip this tick.
	});
});

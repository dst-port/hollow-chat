// Avatar/banner URLs carry the original (percent-encoded) filename, so an
// animated one is spotted by its extension. `.` encodes to `%2e`.
export function isVideoMedia(url: string | null | undefined): boolean {
	return !!url && /(\.|%2e)(mp4|webm)(\?|#|$)/i.test(url);
}

// WebKitGTK (and Safari) don't always honour the `autoplay` attribute for
// muted inline video - kick playback by hand and paint the first frame so the
// element never sits there as a black rectangle over the fallback gradient.
export function playInline(node: HTMLVideoElement) {
	node.muted = true;
	node.playsInline = true;
	const kick = () => {
		const p = node.play();
		if (p && typeof p.catch === "function") p.catch(() => {});
	};
	kick();
	node.addEventListener("loadeddata", kick, { once: true });
	node.addEventListener("canplay", kick, { once: true });
}

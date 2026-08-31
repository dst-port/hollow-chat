// Avatar/banner URLs carry the original (percent-encoded) filename, so an
// animated one is spotted by its extension. `.` encodes to `%2e`.
export function isVideoMedia(url: string | null | undefined): boolean {
	return !!url && /(\.|%2e)(mp4|webm)(\?|#|$)/i.test(url);
}

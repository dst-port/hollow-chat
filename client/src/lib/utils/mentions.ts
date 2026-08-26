function escapeRegExp(text: string): string {
	return text.replace(/[.*+?^${}()|[\]\\]/g, "\\$&");
}

/**
 * True if `text` contains an @mention of `username`, or (for channels, not
 * DMs) an @everyone/@here. Content is E2EE, so this only ever runs
 * client-side after decryption - the server never sees or scans it.
 */
export function textMentionsUser(text: string, username: string, isChannel: boolean): boolean {
	if (!username) return false;
	const userRe = new RegExp(`@${escapeRegExp(username)}\\b`, "i");
	if (userRe.test(text)) return true;
	if (isChannel && /@(everyone|here)\b/i.test(text)) return true;
	return false;
}

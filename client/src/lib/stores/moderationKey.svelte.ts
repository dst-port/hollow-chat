// Staff moderation private key, held in memory for the current page session
// only. Never written to storage, never sent to the server. Cleared on
// reload or explicit clear(). Used to decrypt sealed reports client-side
// (see crypto/moderation.ts `openReport`).
let key = $state("");

export const moderationKey = {
	get value() {
		return key;
	},
	set value(v: string) {
		key = v;
	},
	get present() {
		return key.trim().length > 0;
	},
	clear() {
		key = "";
	}
};

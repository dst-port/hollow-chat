function relevantKeys(username: string): string[] {
	const prefixes = [
		`hollowchat_identity_${username}`,
		`hollowchat_session_${username}_`,
		`hollowchat_groupsend_${username}_`,
		`hollowchat_grouprecv_${username}_`,
		"hollowchat_sent_cache"
	];
	const keys: string[] = [];
	for (let i = 0; i < localStorage.length; i++) {
		const key = localStorage.key(i);
		if (!key) continue;
		if (prefixes.some((prefix) => key === prefix || key.startsWith(prefix))) {
			keys.push(key);
		}
	}
	return keys;
}

export function exportLocalCryptoState(username: string): string {
	const entries: [string, string][] = [];
	for (const key of relevantKeys(username)) {
		const value = localStorage.getItem(key);
		if (value !== null) entries.push([key, value]);
	}
	return JSON.stringify(entries);
}

export function importLocalCryptoState(serialized: string): void {
	const entries = JSON.parse(serialized) as [string, string][];
	for (const [key, value] of entries) {
		localStorage.setItem(key, value);
	}
}

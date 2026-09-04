import { PEER_ID_PREFIX } from "./peer-identity";

// The only kinds of key a device-link transfer is allowed to touch. The export
// side is scoped to one username, but the import side has no username to scope
// by, so it enforces the prefixes alone - a blob arriving over the wire must
// never be able to write arbitrary localStorage keys.
const ALLOWED_PREFIXES = [
	"hollowchat_identity_",
	"hollowchat_session_",
	"hollowchat_groupsend_",
	"hollowchat_grouprecv_",
	PEER_ID_PREFIX,
	"hollowchat_sent_cache"
];

function relevantKeys(username: string): string[] {
	const prefixes = [
		`hollowchat_identity_${username}`,
		`hollowchat_session_${username}_`,
		`hollowchat_groupsend_${username}_`,
		`hollowchat_grouprecv_${username}_`,
		`${PEER_ID_PREFIX}${username}_`,
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

export function isImportableKey(key: string): boolean {
	return ALLOWED_PREFIXES.some((prefix) => key === prefix || key.startsWith(prefix));
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
		if (typeof key !== "string" || typeof value !== "string") continue;
		if (!isImportableKey(key)) continue;
		localStorage.setItem(key, value);
	}
}

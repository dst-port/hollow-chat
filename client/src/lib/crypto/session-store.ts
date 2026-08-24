import type { RatchetState } from "./ratchet";

export function sessionStorageKey(myUsername: string, peerUsername: string): string {
	return `hollowchat_session_${myUsername}_${peerUsername}`;
}
const storageKey = sessionStorageKey;

export function loadSession(myUsername: string, peerUsername: string): RatchetState | null {
	const raw = localStorage.getItem(storageKey(myUsername, peerUsername));
	if (!raw) return null;
	try {
		return JSON.parse(raw) as RatchetState;
	} catch {
		return null;
	}
}

export function saveSession(myUsername: string, peerUsername: string, state: RatchetState) {
	localStorage.setItem(storageKey(myUsername, peerUsername), JSON.stringify(state));
}

export function clearSession(myUsername: string, peerUsername: string) {
	localStorage.removeItem(storageKey(myUsername, peerUsername));
}

export function renameAllSessions(oldUsername: string, newUsername: string) {
	const prefix = `hollowchat_session_${oldUsername}_`;
	const renames: [string, string][] = [];
	for (let i = 0; i < localStorage.length; i++) {
		const key = localStorage.key(i);
		if (key && key.startsWith(prefix)) {
			const peer = key.slice(prefix.length);
			renames.push([key, storageKey(newUsername, peer)]);
		}
	}
	for (const [oldKey, newKey] of renames) {
		const raw = localStorage.getItem(oldKey);
		if (raw) localStorage.setItem(newKey, raw);
		localStorage.removeItem(oldKey);
	}
}

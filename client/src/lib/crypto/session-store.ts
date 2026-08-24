import type { RatchetState } from "./ratchet";

function storageKey(myUsername: string, peerUsername: string): string {
	return `hollowchat_session_${myUsername}_${peerUsername}`;
}

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

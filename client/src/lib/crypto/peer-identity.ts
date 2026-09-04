// Trust-on-first-use pinning of a peer's long-term identity key.
//
// X3DH by itself proves only that whoever sent a handshake knows the private
// half of the identity key *named in that handshake* - and the key bundle we
// would compare it against is served by the same server that relays the
// message. So neither the header nor the bundle can authenticate a peer on
// its own: the server could substitute its own key in both places, complete
// the handshake, and read everything we send back.
//
// Pinning the key we first saw closes that: a substituted key no longer
// matches, and the client refuses rather than silently starting a new session
// with a stranger. See `IdentityChangedError` for what the UI should do.

import { kdf } from "./primitives";
import { fromBase64, concatBytes } from "./encoding";

export type PinScope = { myUsername: string; peerUsername: string };

/**
 * A number two people can read to each other to confirm they hold the same
 * keys. Pinning alone can't tell a legitimate first contact from a server that
 * substituted its own key at that moment - only comparing the value out of
 * band can, so this is what makes the guarantee complete.
 *
 * Both sides must arrive at the same string, so the two keys are sorted before
 * hashing rather than ordered by who is asking.
 */
export function safetyNumber(myIdentityKey: string, theirIdentityKey: string): string {
	const [first, second] = [myIdentityKey, theirIdentityKey].sort();
	const material = concatBytes(fromBase64(first), fromBase64(second));
	// 60 bytes -> twelve 5-byte chunks -> twelve 5-digit groups, 60 digits.
	const digest = kdf(material, new Uint8Array(32), "HollowChatSafetyNumber", 60);

	const groups: string[] = [];
	for (let i = 0; i < digest.length; i += 5) {
		let value = 0;
		for (let j = 0; j < 5; j++) value = value * 256 + digest[i + j];
		groups.push((value % 100000).toString().padStart(5, "0"));
	}
	return groups.join(" ");
}

export const PEER_ID_PREFIX = "hollowchat_peerid_";

export function peerIdentityStorageKey(myUsername: string, peerUsername: string): string {
	return `${PEER_ID_PREFIX}${myUsername}_${peerUsername}`;
}

/** Thrown when a peer's identity key doesn't match the one we pinned. */
export class IdentityChangedError extends Error {
	readonly peerUsername: string;

	constructor(peerUsername: string) {
		super(
			`the identity key for ${peerUsername} has changed - this can mean they reinstalled, ` +
				`or that someone is impersonating them`
		);
		this.name = "IdentityChangedError";
		this.peerUsername = peerUsername;
	}
}

export function loadPeerIdentity(myUsername: string, peerUsername: string): string | null {
	return localStorage.getItem(peerIdentityStorageKey(myUsername, peerUsername));
}

export function savePeerIdentity(myUsername: string, peerUsername: string, identityKey: string) {
	localStorage.setItem(peerIdentityStorageKey(myUsername, peerUsername), identityKey);
}

/**
 * Deliberately forget a peer's pinned key, so the next handshake pins afresh.
 * Only ever call this from an explicit user action - the whole point of the
 * pin is that nothing automatic can clear it.
 */
export function clearPeerIdentity(myUsername: string, peerUsername: string) {
	localStorage.removeItem(peerIdentityStorageKey(myUsername, peerUsername));
}

/**
 * Check `identityKey` against the pin, pinning it if this is first contact.
 * Throws `IdentityChangedError` when it doesn't match what we already trust.
 */
export function pinOrVerifyPeerIdentity(
	myUsername: string,
	peerUsername: string,
	identityKey: string
): void {
	const pinned = loadPeerIdentity(myUsername, peerUsername);
	if (pinned === null) {
		savePeerIdentity(myUsername, peerUsername, identityKey);
		return;
	}
	if (pinned !== identityKey) {
		throw new IdentityChangedError(peerUsername);
	}
}

export function renameAllPeerIdentities(oldUsername: string, newUsername: string) {
	const prefix = `${PEER_ID_PREFIX}${oldUsername}_`;
	const renames: [string, string][] = [];
	for (let i = 0; i < localStorage.length; i++) {
		const key = localStorage.key(i);
		if (key && key.startsWith(prefix)) {
			const peer = key.slice(prefix.length);
			renames.push([key, peerIdentityStorageKey(newUsername, peer)]);
		}
	}
	for (const [oldKey, newKey] of renames) {
		const raw = localStorage.getItem(oldKey);
		if (raw) localStorage.setItem(newKey, raw);
		localStorage.removeItem(oldKey);
	}
}

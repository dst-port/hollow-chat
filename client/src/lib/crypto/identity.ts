import {
	generateEd25519KeyPair,
	generateX25519KeyPair,
	sign,
	type KeyPair
} from "./primitives";
import { toBase64, fromBase64 } from "./encoding";
import * as api from "$lib/api/client";

const PREKEY_BATCH_SIZE = 20;
const REPLENISH_THRESHOLD = 5;

type StoredKeyPair = { priv: string; pub: string };

type StoredIdentity = {
	identityEd: StoredKeyPair;
	identityX: StoredKeyPair;
	signedPrekeyId: number;
	signedPrekey: StoredKeyPair;
	signedPrekeySig: string;
	oneTimePrekeys: Record<number, StoredKeyPair>;
	nextPrekeyId: number;
};

function storageKey(username: string): string {
	return `hollowchat_identity_${username}`;
}

function toStored(pair: KeyPair): StoredKeyPair {
	return { priv: toBase64(pair.privateKey), pub: toBase64(pair.publicKey) };
}

function fromStored(pair: StoredKeyPair): KeyPair {
	return { privateKey: fromBase64(pair.priv), publicKey: fromBase64(pair.pub) };
}

function load(username: string): StoredIdentity | null {
	const raw = localStorage.getItem(storageKey(username));
	if (!raw) return null;
	try {
		return JSON.parse(raw) as StoredIdentity;
	} catch {
		return null;
	}
}

function save(username: string, identity: StoredIdentity) {
	localStorage.setItem(storageKey(username), JSON.stringify(identity));
}

export function renameLocalIdentity(oldUsername: string, newUsername: string) {
	const raw = localStorage.getItem(storageKey(oldUsername));
	if (!raw) return;
	localStorage.setItem(storageKey(newUsername), raw);
	localStorage.removeItem(storageKey(oldUsername));
}

function generateOneTimePrekeys(startId: number, count: number): Record<number, StoredKeyPair> {
	const out: Record<number, StoredKeyPair> = {};
	for (let i = 0; i < count; i++) {
		out[startId + i] = toStored(generateX25519KeyPair());
	}
	return out;
}

function createIdentity(): StoredIdentity {
	const identityEd = generateEd25519KeyPair();
	const identityX = generateX25519KeyPair();
	const signedPrekey = generateX25519KeyPair();
	const signedPrekeySig = sign(identityEd.privateKey, signedPrekey.publicKey);

	return {
		identityEd: toStored(identityEd),
		identityX: toStored(identityX),
		signedPrekeyId: 1,
		signedPrekey: toStored(signedPrekey),
		signedPrekeySig: toBase64(signedPrekeySig),
		oneTimePrekeys: generateOneTimePrekeys(1, PREKEY_BATCH_SIZE),
		nextPrekeyId: 1 + PREKEY_BATCH_SIZE
	};
}

function bundleUploadPayload(identity: StoredIdentity, onlyNewPrekeys?: Record<number, StoredKeyPair>): api.UploadBundleRequest {
	const prekeys = onlyNewPrekeys ?? identity.oneTimePrekeys;
	return {
		ed25519_public: identity.identityEd.pub,
		x25519_public: identity.identityX.pub,
		signed_prekey_id: identity.signedPrekeyId,
		signed_prekey_public: identity.signedPrekey.pub,
		signed_prekey_signature: identity.signedPrekeySig,
		one_time_prekeys: Object.entries(prekeys).map(([keyId, pair]) => ({
			key_id: Number(keyId),
			public_key: pair.pub
		}))
	};
}

export async function ensureIdentity(token: string, username: string): Promise<void> {
	let identity = load(username);
	if (!identity) {
		identity = createIdentity();
		save(username, identity);
		await api.uploadKeyBundle(token, bundleUploadPayload(identity));
		return;
	}

	try {
		const { count } = await api.prekeyCount(token);
		if (count < REPLENISH_THRESHOLD) {
			const fresh = generateOneTimePrekeys(identity.nextPrekeyId, PREKEY_BATCH_SIZE);
			identity.oneTimePrekeys = { ...identity.oneTimePrekeys, ...fresh };
			identity.nextPrekeyId += PREKEY_BATCH_SIZE;
			save(username, identity);
			await api.uploadKeyBundle(token, bundleUploadPayload(identity, fresh));
		}
	} catch {
		// non-fatal: replenishment can retry next time
	}
}

export function getIdentityX25519(username: string): KeyPair {
	const identity = load(username);
	if (!identity) throw new Error("no local identity for " + username);
	return fromStored(identity.identityX);
}

export function getSignedPrekey(username: string): { id: number; keyPair: KeyPair } {
	const identity = load(username);
	if (!identity) throw new Error("no local identity for " + username);
	return { id: identity.signedPrekeyId, keyPair: fromStored(identity.signedPrekey) };
}

export function takeOneTimePrekey(username: string, keyId: number): KeyPair | null {
	const identity = load(username);
	if (!identity) return null;
	const stored = identity.oneTimePrekeys[keyId];
	if (!stored) return null;
	delete identity.oneTimePrekeys[keyId];
	save(username, identity);
	return fromStored(stored);
}

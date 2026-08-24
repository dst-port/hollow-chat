import * as api from "$lib/api/client";
import * as x3dh from "./x3dh";
import * as ratchet from "./ratchet";
import { getIdentityX25519, getSignedPrekey, takeOneTimePrekey } from "./identity";
import { loadSession, saveSession } from "./session-store";
import { toBase64, fromBase64, utf8Encode, utf8Decode } from "./encoding";

type X3dhHeader = {
	ik: string;
	ek: string;
	spkId: number;
	opkId: number | null;
};

type Envelope = {
	v: 1;
	x3dh?: X3dhHeader;
	header: ratchet.Header;
	nonce: string;
	ciphertext: string;
};

export const ENVELOPE_PREFIX = "hcE2EE1:";

export function isEnvelope(content: string): boolean {
	return content.startsWith(ENVELOPE_PREFIX);
}

async function fetchBundle(token: string, peerUsername: string): Promise<x3dh.PrekeyBundle> {
	const res = await api.fetchKeyBundle(token, peerUsername);
	return {
		identityEd25519Public: fromBase64(res.ed25519_public),
		identityX25519Public: fromBase64(res.x25519_public),
		signedPrekeyId: res.signed_prekey_id,
		signedPrekeyPublic: fromBase64(res.signed_prekey_public),
		signedPrekeySignature: fromBase64(res.signed_prekey_signature),
		oneTimePrekeyId: res.one_time_prekey?.key_id ?? null,
		oneTimePrekeyPublic: res.one_time_prekey ? fromBase64(res.one_time_prekey.public_key) : null
	};
}

export async function encryptForPeer(
	token: string,
	myUsername: string,
	peerUsername: string,
	plaintext: string
): Promise<string> {
	let session = loadSession(myUsername, peerUsername);
	let x3dhHeader: X3dhHeader | undefined;

	if (!session) {
		const myIdentityX = getIdentityX25519(myUsername);
		const bundle = await fetchBundle(token, peerUsername);
		const result = x3dh.initiate(myIdentityX, bundle);
		session = ratchet.initAsSender(result.sharedSecret, bundle.signedPrekeyPublic);
		x3dhHeader = {
			ik: toBase64(myIdentityX.publicKey),
			ek: toBase64(result.ephemeralPublic),
			spkId: result.usedSignedPrekeyId,
			opkId: result.usedOneTimePrekeyId
		};
	}

	const { header, nonce, ciphertext } = await ratchet.ratchetEncrypt(session, utf8Encode(plaintext));
	saveSession(myUsername, peerUsername, session);

	const envelope: Envelope = { v: 1, x3dh: x3dhHeader, header, nonce, ciphertext };
	return ENVELOPE_PREFIX + JSON.stringify(envelope);
}

export async function decryptFromPeer(
	myUsername: string,
	peerUsername: string,
	content: string
): Promise<string> {
	if (!isEnvelope(content)) return content;
	const envelope = JSON.parse(content.slice(ENVELOPE_PREFIX.length)) as Envelope;

	let session = loadSession(myUsername, peerUsername);

	if (envelope.x3dh) {
		const myIdentityX = getIdentityX25519(myUsername);
		const { id: spkId, keyPair: signedPrekey } = getSignedPrekey(myUsername);
		if (spkId !== envelope.x3dh.spkId) {
			throw new Error("message references an unknown signed prekey");
		}
		const oneTimePrekey =
			envelope.x3dh.opkId !== null ? takeOneTimePrekey(myUsername, envelope.x3dh.opkId) : null;

		const sharedSecret = x3dh.respond(
			myIdentityX,
			signedPrekey,
			oneTimePrekey,
			fromBase64(envelope.x3dh.ik),
			fromBase64(envelope.x3dh.ek)
		);
		session = ratchet.initAsReceiver(sharedSecret, signedPrekey);
	}

	if (!session) throw new Error("no session and no handshake data to bootstrap one");

	const plaintext = await ratchet.ratchetDecrypt(session, envelope.header, envelope.nonce, envelope.ciphertext);
	saveSession(myUsername, peerUsername, session);

	return utf8Decode(plaintext);
}

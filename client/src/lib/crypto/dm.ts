import * as api from "$lib/api/client";
import * as x3dh from "./x3dh";
import * as ratchet from "./ratchet";
import { getIdentityX25519, getSignedPrekey, takeOneTimePrekey } from "./identity";
import { loadSession, saveSession, sessionStorageKey } from "./session-store";
import { IdentityChangedError, loadPeerIdentity, pinOrVerifyPeerIdentity } from "./peer-identity";
import { toBase64, fromBase64, utf8Encode, utf8Decode } from "./encoding";
import { deviceSync } from "$lib/devicelink/sync";

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

async function fetchBundleAfterCoordination(
	token: string,
	myUsername: string,
	peerUsername: string
): Promise<x3dh.PrekeyBundle | null> {
	try {
		return await fetchBundle(token, peerUsername);
	} catch (err) {
		if (!(err instanceof api.ApiError) || err.status !== 409) throw err;
		await deviceSync.waitForSync(myUsername, sessionStorageKey(myUsername, peerUsername));
		if (loadSession(myUsername, peerUsername)) return null;
		return fetchBundle(token, peerUsername);
	}
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
		const decision = await deviceSync.claimNewSession(myUsername, peerUsername);
		if (decision === "wait-for-sync") {
			await deviceSync.waitForSync(myUsername, sessionStorageKey(myUsername, peerUsername));
			session = loadSession(myUsername, peerUsername);
		}
	}

	if (!session) {
		const bundle = await fetchBundleAfterCoordination(token, myUsername, peerUsername);
		if (bundle) {
			// The bundle comes from the server, so it can't authenticate the
			// peer by itself - but it must still agree with the key we pinned,
			// or we'd be encrypting to a substituted one.
			pinOrVerifyPeerIdentity(myUsername, peerUsername, toBase64(bundle.identityX25519Public));

			const myIdentityX = getIdentityX25519(myUsername);
			const result = x3dh.initiate(myIdentityX, bundle);
			session = ratchet.initAsSender(result.sharedSecret, bundle.signedPrekeyPublic);
			x3dhHeader = {
				ik: toBase64(myIdentityX.publicKey),
				ek: toBase64(result.ephemeralPublic),
				spkId: result.usedSignedPrekeyId,
				opkId: result.usedOneTimePrekeyId
			};
		} else {
			session = loadSession(myUsername, peerUsername);
		}
	}

	if (!session) throw new Error("could not establish a session with " + peerUsername);

	const { header, nonce, ciphertext } = await ratchet.ratchetEncrypt(session, utf8Encode(plaintext));
	saveSession(myUsername, peerUsername, session);
	deviceSync.broadcastChange(myUsername, sessionStorageKey(myUsername, peerUsername), JSON.stringify(session));

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
		// A handshake header names its own identity key, so on its own it
		// proves nothing about who sent the message - anyone able to place a
		// message in this conversation could name a key they control and have
		// us derive a session with them instead of the peer. Only accept a key
		// we've already pinned for this peer (or pin it on first contact).
		//
		// An established session is never torn down by an unpinned handshake:
		// for conversations that predate pinning there is nothing to compare
		// against, and silently resetting the ratchet is exactly the move an
		// impersonator needs.
		if (session && loadPeerIdentity(myUsername, peerUsername) === null) {
			throw new IdentityChangedError(peerUsername);
		}
		pinOrVerifyPeerIdentity(myUsername, peerUsername, envelope.x3dh.ik);

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

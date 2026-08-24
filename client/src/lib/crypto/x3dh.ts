import { dh, kdf, verify, generateX25519KeyPair, type KeyPair } from "./primitives";
import { concatBytes } from "./encoding";

export type PrekeyBundle = {
	identityEd25519Public: Uint8Array;
	identityX25519Public: Uint8Array;
	signedPrekeyId: number;
	signedPrekeyPublic: Uint8Array;
	signedPrekeySignature: Uint8Array;
	oneTimePrekeyId: number | null;
	oneTimePrekeyPublic: Uint8Array | null;
};

function deriveSharedSecret(dhOutputs: Uint8Array[]): Uint8Array {
	const ikm = concatBytes(...dhOutputs);
	return kdf(ikm, new Uint8Array(32), "HollowChatX3DH", 32);
}

export type InitiateResult = {
	sharedSecret: Uint8Array;
	ephemeralPublic: Uint8Array;
	usedSignedPrekeyId: number;
	usedOneTimePrekeyId: number | null;
};

export function initiate(
	myIdentityX25519: KeyPair,
	theirBundle: PrekeyBundle
): InitiateResult {
	const verified = verify(
		theirBundle.identityEd25519Public,
		theirBundle.signedPrekeyPublic,
		theirBundle.signedPrekeySignature
	);
	if (!verified) throw new Error("signed prekey signature verification failed");

	const ephemeral = generateX25519KeyPair();

	const dh1 = dh(myIdentityX25519.privateKey, theirBundle.signedPrekeyPublic);
	const dh2 = dh(ephemeral.privateKey, theirBundle.identityX25519Public);
	const dh3 = dh(ephemeral.privateKey, theirBundle.signedPrekeyPublic);
	const dhOutputs = [dh1, dh2, dh3];
	if (theirBundle.oneTimePrekeyPublic) {
		dhOutputs.push(dh(ephemeral.privateKey, theirBundle.oneTimePrekeyPublic));
	}

	return {
		sharedSecret: deriveSharedSecret(dhOutputs),
		ephemeralPublic: ephemeral.publicKey,
		usedSignedPrekeyId: theirBundle.signedPrekeyId,
		usedOneTimePrekeyId: theirBundle.oneTimePrekeyId
	};
}

export function respond(
	myIdentityX25519: KeyPair,
	mySignedPrekey: KeyPair,
	myOneTimePrekey: KeyPair | null,
	theirIdentityX25519Public: Uint8Array,
	theirEphemeralPublic: Uint8Array
): Uint8Array {
	const dh1 = dh(mySignedPrekey.privateKey, theirIdentityX25519Public);
	const dh2 = dh(myIdentityX25519.privateKey, theirEphemeralPublic);
	const dh3 = dh(mySignedPrekey.privateKey, theirEphemeralPublic);
	const dhOutputs = [dh1, dh2, dh3];
	if (myOneTimePrekey) {
		dhOutputs.push(dh(myOneTimePrekey.privateKey, theirEphemeralPublic));
	}
	return deriveSharedSecret(dhOutputs);
}

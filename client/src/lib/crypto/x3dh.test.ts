import { describe, it, expect } from "vitest";
import { generateX25519KeyPair, generateEd25519KeyPair, sign } from "./primitives";
import { initiate, respond, type PrekeyBundle } from "./x3dh";

function makeResponder() {
	const identityEd25519 = generateEd25519KeyPair();
	const identityX25519 = generateX25519KeyPair();
	const signedPrekey = generateX25519KeyPair();
	const oneTimePrekey = generateX25519KeyPair();
	const signedPrekeySignature = sign(identityEd25519.privateKey, signedPrekey.publicKey);

	const bundle: PrekeyBundle = {
		identityEd25519Public: identityEd25519.publicKey,
		identityX25519Public: identityX25519.publicKey,
		signedPrekeyId: 1,
		signedPrekeyPublic: signedPrekey.publicKey,
		signedPrekeySignature,
		oneTimePrekeyId: 7,
		oneTimePrekeyPublic: oneTimePrekey.publicKey
	};

	return { identityX25519, signedPrekey, oneTimePrekey, bundle };
}

describe("x3dh", () => {
	it("initiator and responder derive the same shared secret (with one-time prekey)", () => {
		const alice = generateX25519KeyPair();
		const bob = makeResponder();

		const result = initiate(alice, bob.bundle);
		const responderSecret = respond(
			bob.identityX25519,
			bob.signedPrekey,
			bob.oneTimePrekey,
			alice.publicKey,
			result.ephemeralPublic
		);

		expect(result.sharedSecret).toEqual(responderSecret);
	});

	it("initiator and responder derive the same shared secret (without a one-time prekey)", () => {
		const alice = generateX25519KeyPair();
		const bob = makeResponder();
		const bundleWithoutOtp: PrekeyBundle = {
			...bob.bundle,
			oneTimePrekeyId: null,
			oneTimePrekeyPublic: null
		};

		const result = initiate(alice, bundleWithoutOtp);
		const responderSecret = respond(bob.identityX25519, bob.signedPrekey, null, alice.publicKey, result.ephemeralPublic);

		expect(result.sharedSecret).toEqual(responderSecret);
		expect(result.usedOneTimePrekeyId).toBeNull();
	});

	it("rejects a bundle whose signed prekey signature doesn't verify", () => {
		const alice = generateX25519KeyPair();
		const bob = makeResponder();
		const otherSigner = generateEd25519KeyPair();
		const forgedBundle: PrekeyBundle = {
			...bob.bundle,
			signedPrekeySignature: sign(otherSigner.privateKey, bob.signedPrekey.publicKey)
		};

		expect(() => initiate(alice, forgedBundle)).toThrow();
	});

	it("two initiations against the same bundle produce different shared secrets", () => {
		const alice = generateX25519KeyPair();
		const bob = makeResponder();

		const first = initiate(alice, bob.bundle);
		const second = initiate(alice, bob.bundle);

		expect(first.sharedSecret).not.toEqual(second.sharedSecret);
		expect(first.ephemeralPublic).not.toEqual(second.ephemeralPublic);
	});
});

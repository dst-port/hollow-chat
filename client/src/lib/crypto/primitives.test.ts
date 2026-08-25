import { describe, it, expect } from "vitest";
import {
	generateX25519KeyPair,
	generateEd25519KeyPair,
	sign,
	verify,
	dh,
	kdf
} from "./primitives";

describe("primitives", () => {
	it("x25519 dh is symmetric between two parties", () => {
		const a = generateX25519KeyPair();
		const b = generateX25519KeyPair();

		const sharedA = dh(a.privateKey, b.publicKey);
		const sharedB = dh(b.privateKey, a.publicKey);

		expect(sharedA).toEqual(sharedB);
	});

	it("dh output differs for different key pairs", () => {
		const a = generateX25519KeyPair();
		const b = generateX25519KeyPair();
		const c = generateX25519KeyPair();

		expect(dh(a.privateKey, b.publicKey)).not.toEqual(dh(a.privateKey, c.publicKey));
	});

	it("ed25519 sign/verify round trip succeeds", () => {
		const keyPair = generateEd25519KeyPair();
		const message = new TextEncoder().encode("hello hollowchat");
		const signature = sign(keyPair.privateKey, message);

		expect(verify(keyPair.publicKey, message, signature)).toBe(true);
	});

	it("verify rejects a tampered message", () => {
		const keyPair = generateEd25519KeyPair();
		const message = new TextEncoder().encode("hello hollowchat");
		const signature = sign(keyPair.privateKey, message);
		const tampered = new TextEncoder().encode("hello hollowchat!");

		expect(verify(keyPair.publicKey, tampered, signature)).toBe(false);
	});

	it("verify rejects a signature from a different key", () => {
		const signer = generateEd25519KeyPair();
		const impostor = generateEd25519KeyPair();
		const message = new TextEncoder().encode("hello hollowchat");
		const signature = sign(signer.privateKey, message);

		expect(verify(impostor.publicKey, message, signature)).toBe(false);
	});

	it("kdf is deterministic for the same inputs", () => {
		const ikm = new Uint8Array([1, 2, 3, 4]);
		const salt = new Uint8Array(32);

		const first = kdf(ikm, salt, "test-info", 32);
		const second = kdf(ikm, salt, "test-info", 32);

		expect(first).toEqual(second);
	});

	it("kdf output differs when the info string differs", () => {
		const ikm = new Uint8Array([1, 2, 3, 4]);
		const salt = new Uint8Array(32);

		const a = kdf(ikm, salt, "info-a", 32);
		const b = kdf(ikm, salt, "info-b", 32);

		expect(a).not.toEqual(b);
	});
});

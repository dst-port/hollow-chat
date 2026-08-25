import { describe, it, expect } from "vitest";
import { encrypt, decrypt } from "./aead";

function key(): Uint8Array {
	return crypto.getRandomValues(new Uint8Array(32));
}

describe("aead", () => {
	it("round trips plaintext through encrypt/decrypt", async () => {
		const k = key();
		const plaintext = new TextEncoder().encode("secret message");
		const aad = new TextEncoder().encode("channel:1");

		const { nonce, ciphertext } = await encrypt(k, plaintext, aad);
		const decrypted = await decrypt(k, nonce, ciphertext, aad);

		expect(decrypted).toEqual(plaintext);
	});

	it("fails to decrypt with the wrong key", async () => {
		const plaintext = new TextEncoder().encode("secret message");
		const aad = new TextEncoder().encode("channel:1");

		const { nonce, ciphertext } = await encrypt(key(), plaintext, aad);

		await expect(decrypt(key(), nonce, ciphertext, aad)).rejects.toThrow();
	});

	it("fails to decrypt with mismatched associated data", async () => {
		const k = key();
		const plaintext = new TextEncoder().encode("secret message");

		const { nonce, ciphertext } = await encrypt(k, plaintext, new TextEncoder().encode("channel:1"));

		await expect(decrypt(k, nonce, ciphertext, new TextEncoder().encode("channel:2"))).rejects.toThrow();
	});

	it("fails to decrypt a tampered ciphertext", async () => {
		const k = key();
		const plaintext = new TextEncoder().encode("secret message");
		const aad = new TextEncoder().encode("channel:1");

		const { nonce, ciphertext } = await encrypt(k, plaintext, aad);
		const tampered = new Uint8Array(ciphertext);
		tampered[0] ^= 0xff;

		await expect(decrypt(k, nonce, tampered, aad)).rejects.toThrow();
	});

	it("uses a fresh random nonce per encryption", async () => {
		const k = key();
		const plaintext = new TextEncoder().encode("same message");
		const aad = new Uint8Array(0);

		const first = await encrypt(k, plaintext, aad);
		const second = await encrypt(k, plaintext, aad);

		expect(first.nonce).not.toEqual(second.nonce);
		expect(first.ciphertext).not.toEqual(second.ciphertext);
	});
});

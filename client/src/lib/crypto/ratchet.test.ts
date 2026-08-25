import { describe, it, expect, beforeEach } from "vitest";
import { generateX25519KeyPair } from "./primitives";
import { initAsSender, initAsReceiver, ratchetEncrypt, ratchetDecrypt, type RatchetState } from "./ratchet";

const utf8 = (text: string) => new TextEncoder().encode(text);
const readUtf8 = (bytes: Uint8Array) => new TextDecoder().decode(bytes);

let alice: RatchetState;
let bob: RatchetState;

beforeEach(() => {
	const sharedSecret = crypto.getRandomValues(new Uint8Array(32));
	const bobInitialRatchet = generateX25519KeyPair();

	bob = initAsReceiver(sharedSecret, bobInitialRatchet);
	alice = initAsSender(sharedSecret, bobInitialRatchet.publicKey);
});

describe("ratchet", () => {
	it("delivers a single message from sender to receiver", async () => {
		const sent = await ratchetEncrypt(alice, utf8("hello bob"));
		const plaintext = await ratchetDecrypt(bob, sent.header, sent.nonce, sent.ciphertext);

		expect(readUtf8(plaintext)).toBe("hello bob");
	});

	it("supports a full back-and-forth conversation with DH ratchet steps", async () => {
		const m1 = await ratchetEncrypt(alice, utf8("hi"));
		expect(readUtf8(await ratchetDecrypt(bob, m1.header, m1.nonce, m1.ciphertext))).toBe("hi");

		const m2 = await ratchetEncrypt(bob, utf8("hey alice"));
		expect(readUtf8(await ratchetDecrypt(alice, m2.header, m2.nonce, m2.ciphertext))).toBe("hey alice");

		const m3 = await ratchetEncrypt(alice, utf8("how are you"));
		expect(readUtf8(await ratchetDecrypt(bob, m3.header, m3.nonce, m3.ciphertext))).toBe("how are you");

		const m4 = await ratchetEncrypt(bob, utf8("good, you?"));
		expect(readUtf8(await ratchetDecrypt(alice, m4.header, m4.nonce, m4.ciphertext))).toBe("good, you?");
	});

	it("handles out-of-order delivery within the same chain", async () => {
		const m1 = await ratchetEncrypt(alice, utf8("one"));
		const m2 = await ratchetEncrypt(alice, utf8("two"));
		const m3 = await ratchetEncrypt(alice, utf8("three"));

		expect(readUtf8(await ratchetDecrypt(bob, m3.header, m3.nonce, m3.ciphertext))).toBe("three");
		expect(readUtf8(await ratchetDecrypt(bob, m1.header, m1.nonce, m1.ciphertext))).toBe("one");
		expect(readUtf8(await ratchetDecrypt(bob, m2.header, m2.nonce, m2.ciphertext))).toBe("two");
	});

	it("handles a skipped message across a DH ratchet step", async () => {
		const m1 = await ratchetEncrypt(alice, utf8("skip me"));
		const m2 = await ratchetEncrypt(alice, utf8("deliver me first"));

		expect(readUtf8(await ratchetDecrypt(bob, m2.header, m2.nonce, m2.ciphertext))).toBe("deliver me first");

		const reply = await ratchetEncrypt(bob, utf8("reply before catching up"));
		expect(readUtf8(await ratchetDecrypt(alice, reply.header, reply.nonce, reply.ciphertext))).toBe(
			"reply before catching up"
		);

		expect(readUtf8(await ratchetDecrypt(bob, m1.header, m1.nonce, m1.ciphertext))).toBe("skip me");
	});

	it("rejects a message replayed twice", async () => {
		const m1 = await ratchetEncrypt(alice, utf8("only once"));
		await ratchetDecrypt(bob, m1.header, m1.nonce, m1.ciphertext);

		await expect(ratchetDecrypt(bob, m1.header, m1.nonce, m1.ciphertext)).rejects.toThrow();
	});

	it("rejects a tampered ciphertext", async () => {
		const m1 = await ratchetEncrypt(alice, utf8("integrity matters"));
		const tampered = { ...m1, ciphertext: m1.ciphertext.slice(0, -2) + (m1.ciphertext.slice(-2) === "AA" ? "BB" : "AA") };

		await expect(ratchetDecrypt(bob, tampered.header, tampered.nonce, tampered.ciphertext)).rejects.toThrow();
	});

	it("produces different ciphertext for the same plaintext sent twice", async () => {
		const first = await ratchetEncrypt(alice, utf8("repeat"));
		const second = await ratchetEncrypt(alice, utf8("repeat"));

		expect(first.ciphertext).not.toBe(second.ciphertext);
		expect(first.header.n).not.toBe(second.header.n);
	});
});

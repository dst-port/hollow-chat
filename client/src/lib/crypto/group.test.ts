// @vitest-environment jsdom
import { describe, it, expect, beforeEach } from "vitest";
import { encryptForChannel, decryptFromChannel, getOrCreateSendState } from "./group";
import { saveReceiveState } from "./group-key-store";

async function linkReceiver(myUsername: string, channelId: string, senderUsername: string) {
	const senderState = await getOrCreateSendState(senderUsername, channelId);
	saveReceiveState(myUsername, channelId, senderUsername, senderState);
}

beforeEach(() => {
	localStorage.clear();
});

describe("group channel encryption", () => {
	it("round trips a single message", async () => {
		await linkReceiver("bob", "chan1", "alice");

		const ciphertext = await encryptForChannel("alice", "chan1", "hello channel");
		const plaintext = await decryptFromChannel("bob", "chan1", "alice", ciphertext);

		expect(plaintext).toBe("hello channel");
	});

	it("round trips a sequence of messages in order", async () => {
		await linkReceiver("bob", "chan1", "alice");

		for (const text of ["one", "two", "three"]) {
			const ciphertext = await encryptForChannel("alice", "chan1", text);
			expect(await decryptFromChannel("bob", "chan1", "alice", ciphertext)).toBe(text);
		}
	});

	it("throws for a receiver with no sender key linked yet", async () => {
		const ciphertext = await encryptForChannel("alice", "chan1", "hello");

		await expect(decryptFromChannel("bob", "chan1", "alice", ciphertext)).rejects.toThrow(
			"no sender key received yet"
		);
	});

	it("advances past a gap when a later message arrives first", async () => {
		await linkReceiver("bob", "chan1", "alice");

		const m1 = await encryptForChannel("alice", "chan1", "first");
		const m2 = await encryptForChannel("alice", "chan1", "second");

		expect(await decryptFromChannel("bob", "chan1", "alice", m2)).toBe("second");
		void m1;
	});

	it("rejects a message whose iteration was already passed", async () => {
		await linkReceiver("bob", "chan1", "alice");

		const m1 = await encryptForChannel("alice", "chan1", "first");
		const m2 = await encryptForChannel("alice", "chan1", "second");

		await decryptFromChannel("bob", "chan1", "alice", m2);

		await expect(decryptFromChannel("bob", "chan1", "alice", m1)).rejects.toThrow();
	});

	it("rejects a replayed message", async () => {
		await linkReceiver("bob", "chan1", "alice");

		const m1 = await encryptForChannel("alice", "chan1", "once");
		await decryptFromChannel("bob", "chan1", "alice", m1);

		await expect(decryptFromChannel("bob", "chan1", "alice", m1)).rejects.toThrow();
	});

	it("keeps per-channel sender keys isolated", async () => {
		await linkReceiver("bob", "chan1", "alice");

		const forOtherChannel = await encryptForChannel("alice", "chan2", "wrong channel");

		await expect(decryptFromChannel("bob", "chan1", "alice", forOtherChannel)).rejects.toThrow();
	});

	it("passes plain (non-envelope) content through unchanged", async () => {
		const plain = "just plain text, not encrypted";
		expect(await decryptFromChannel("bob", "chan1", "alice", plain)).toBe(plain);
	});
});

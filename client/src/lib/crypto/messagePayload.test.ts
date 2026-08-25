import { describe, it, expect } from "vitest";
import { packPayload, unpackPayload } from "./messagePayload";
import type { EncryptedAttachmentMeta } from "./attachment";

const attachment: EncryptedAttachmentMeta = {
	key: "a2V5",
	nonce: "bm9uY2U",
	filename: "photo.png",
	mimeType: "image/png",
	sizeBytes: 12345
};

describe("messagePayload", () => {
	it("returns text as-is when there is no attachment", () => {
		expect(packPayload("hello", undefined)).toBe("hello");
	});

	it("round trips text without an attachment", () => {
		const packed = packPayload("hello", undefined);
		expect(unpackPayload(packed)).toEqual({ text: "hello" });
	});

	it("round trips text with an attachment", () => {
		const packed = packPayload("check this out", attachment);
		const unpacked = unpackPayload(packed);

		expect(unpacked.text).toBe("check this out");
		expect(unpacked.attachment).toEqual(attachment);
	});

	it("round trips an attachment-only message with empty text", () => {
		const packed = packPayload("", attachment);
		const unpacked = unpackPayload(packed);

		expect(unpacked.text).toBe("");
		expect(unpacked.attachment).toEqual(attachment);
	});

	it("treats plain unprefixed text as backward-compatible legacy content", () => {
		expect(unpackPayload("a message sent before this feature existed")).toEqual({
			text: "a message sent before this feature existed"
		});
	});

	it("falls back to raw text if the packed payload is corrupted", () => {
		const packed = packPayload("hi", attachment);
		const corrupted = packed.slice(0, -3);

		expect(unpackPayload(corrupted)).toEqual({ text: corrupted });
	});
});

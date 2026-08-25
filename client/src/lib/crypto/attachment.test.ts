import { describe, it, expect } from "vitest";
import { encryptFile, decryptBlob } from "./attachment";

function makeFile(content: string, name: string, type: string): File {
	return new File([content], name, { type });
}

describe("attachment encryption", () => {
	it("round trips file bytes through encrypt/decrypt", async () => {
		const original = makeFile("these are the file bytes", "notes.txt", "text/plain");

		const { blob, meta } = await encryptFile(original);
		const decrypted = await decryptBlob(await blob.arrayBuffer(), meta);

		expect(await decrypted.text()).toBe("these are the file bytes");
	});

	it("uploads a generic, opaque blob — never the real name or mime type", async () => {
		const original = makeFile("secret contents", "my-private-photo.png", "image/png");

		const { blob, meta } = await encryptFile(original);

		expect(blob.type).not.toBe("image/png");
		expect(meta.filename).toBe("my-private-photo.png");
		expect(meta.mimeType).toBe("image/png");
	});

	it("restores the real mime type on decrypt via the metadata", async () => {
		const original = makeFile("image bytes here", "photo.png", "image/png");
		const { blob, meta } = await encryptFile(original);

		const decrypted = await decryptBlob(await blob.arrayBuffer(), meta);

		expect(decrypted.type).toBe("image/png");
	});

	it("fails to decrypt with the wrong key", async () => {
		const original = makeFile("data", "f.bin", "application/octet-stream");
		const { blob, meta } = await encryptFile(original);
		const wrongMeta = { ...meta, key: "AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA" };

		await expect(decryptBlob(await blob.arrayBuffer(), wrongMeta)).rejects.toThrow();
	});

	it("generates a fresh random key for every file", async () => {
		const a = await encryptFile(makeFile("x", "a.txt", "text/plain"));
		const b = await encryptFile(makeFile("x", "b.txt", "text/plain"));

		expect(a.meta.key).not.toBe(b.meta.key);
	});
});

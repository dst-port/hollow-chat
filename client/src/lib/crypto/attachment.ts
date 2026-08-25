import * as aead from "./aead";
import { toBase64, fromBase64 } from "./encoding";

const KEY_LEN = 32;
const GENERIC_FILENAME = "file.bin";
const GENERIC_MIME = "application/octet-stream";

export type EncryptedAttachmentMeta = {
	key: string;
	nonce: string;
	filename: string;
	mimeType: string;
	sizeBytes: number;
};

export async function encryptFile(file: File): Promise<{ blob: Blob; meta: EncryptedAttachmentMeta }> {
	const key = crypto.getRandomValues(new Uint8Array(KEY_LEN));
	const plaintext = new Uint8Array(await file.arrayBuffer());
	const { nonce, ciphertext } = await aead.encrypt(key, plaintext, new Uint8Array(0));

	return {
		blob: new Blob([ciphertext as BufferSource], { type: GENERIC_MIME }),
		meta: {
			key: toBase64(key),
			nonce: toBase64(nonce),
			filename: file.name,
			mimeType: file.type || GENERIC_MIME,
			sizeBytes: file.size
		}
	};
}

export function genericUploadName(): string {
	return GENERIC_FILENAME;
}

export async function decryptBlob(
	ciphertext: ArrayBuffer,
	meta: Pick<EncryptedAttachmentMeta, "key" | "nonce" | "mimeType">
): Promise<Blob> {
	const plaintext = await aead.decrypt(
		fromBase64(meta.key),
		fromBase64(meta.nonce),
		new Uint8Array(ciphertext),
		new Uint8Array(0)
	);
	return new Blob([plaintext as BufferSource], { type: meta.mimeType });
}

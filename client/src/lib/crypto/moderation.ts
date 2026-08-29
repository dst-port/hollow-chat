import { dh, generateX25519KeyPair } from "./primitives";
import * as aead from "./aead";
import { toBase64, fromBase64, concatBytes } from "./encoding";

const STAFF_PUBLIC_KEY_B64 = "QLoGpjgJAyr63N1ewBW5lwMEtrhSJR8B3anlXdqWO0M=";
const SEAL_INFO = new TextEncoder().encode("HollowChatModerationSeal");

async function deriveSealKey(shared: Uint8Array): Promise<Uint8Array> {
	const material = await crypto.subtle.digest(
		"SHA-256",
		concatBytes(shared, SEAL_INFO) as BufferSource
	);
	return new Uint8Array(material);
}

export type SealedBox = {
	ephemeralPublicKey: string;
	nonce: string;
	ciphertext: string;
};

async function seal(plaintext: Uint8Array): Promise<SealedBox> {
	const staffPublicKey = fromBase64(STAFF_PUBLIC_KEY_B64);
	const ephemeral = generateX25519KeyPair();
	const shared = dh(ephemeral.privateKey, staffPublicKey);
	const key = await deriveSealKey(shared);
	const { nonce, ciphertext } = await aead.encrypt(key, plaintext, new Uint8Array(0));
	return {
		ephemeralPublicKey: toBase64(ephemeral.publicKey),
		nonce: toBase64(nonce),
		ciphertext: toBase64(ciphertext)
	};
}

export type ReportedMessage = {
	id: string;
	senderUsername: string;
	timestamp: string;
	text: string;
	attachmentFilename?: string;
};

export type ReportPayload = {
	version: 1;
	category: string;
	reason: string;
	reportedUsername: string;
	messages: ReportedMessage[];
	screenshot?: { mimeType: string; dataBase64: string };
};

export type SealedReport = {
	sealedKey: SealedBox;
	payloadNonce: string;
	payloadCiphertext: string;
};

export async function sealReport(payload: ReportPayload): Promise<SealedReport> {
	const dataKey = crypto.getRandomValues(new Uint8Array(32));
	const plaintext = new TextEncoder().encode(JSON.stringify(payload));
	const { nonce, ciphertext } = await aead.encrypt(dataKey, plaintext, new Uint8Array(0));
	const sealedKey = await seal(dataKey);

	return {
		sealedKey,
		payloadNonce: toBase64(nonce),
		payloadCiphertext: toBase64(ciphertext)
	};
}

export type SealedReportFields = {
	sealed_key_ephemeral_public: string;
	sealed_key_nonce: string;
	sealed_key_ciphertext: string;
	payload_nonce: string;
	payload_ciphertext: string;
};

/**
 * Client-side counterpart of tools/decrypt-report.mjs. Unseals one report
 * with the staff X25519 private key (base64). The key is supplied per call
 * and never persisted or sent anywhere by this function.
 */
export async function openReport(
	fields: SealedReportFields,
	staffPrivateKeyB64: string
): Promise<ReportPayload> {
	const staffPrivateKey = fromBase64(staffPrivateKeyB64.trim());
	if (staffPrivateKey.length !== 32) {
		throw new Error("staff key must be 32 bytes (base64)");
	}
	const ephemeralPublic = fromBase64(fields.sealed_key_ephemeral_public);
	const shared = dh(staffPrivateKey, ephemeralPublic);
	const sealKey = await deriveSealKey(shared);

	const dataKey = await aead.decrypt(
		sealKey,
		fromBase64(fields.sealed_key_nonce),
		fromBase64(fields.sealed_key_ciphertext),
		new Uint8Array(0)
	);

	const plaintext = await aead.decrypt(
		dataKey,
		fromBase64(fields.payload_nonce),
		fromBase64(fields.payload_ciphertext),
		new Uint8Array(0)
	);

	return JSON.parse(new TextDecoder().decode(plaintext)) as ReportPayload;
}

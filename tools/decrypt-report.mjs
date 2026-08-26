#!/usr/bin/env node
// Offline moderation report decryptor.
// Staff only. Never run this with the private key anywhere near the server.
//
// Usage:
//   HC_STAFF_PRIVATE_KEY=<base64> node tools/decrypt-report.mjs report.json
//
// report.json is a raw export of one `reports` row with these base64 fields:
//   sealed_key_ephemeral_public, sealed_key_nonce, sealed_key_ciphertext,
//   payload_nonce, payload_ciphertext
//
// Fetch a row with:
//   psql "$DATABASE_URL" -c "SELECT \
//     encode(sealed_key_ephemeral_public, 'base64') AS sealed_key_ephemeral_public, \
//     encode(sealed_key_nonce, 'base64') AS sealed_key_nonce, \
//     encode(sealed_key_ciphertext, 'base64') AS sealed_key_ciphertext, \
//     encode(payload_nonce, 'base64') AS payload_nonce, \
//     encode(payload_ciphertext, 'base64') AS payload_ciphertext \
//     FROM reports WHERE id = '<report id>'" -t -A -F, > report.csv

import { readFileSync } from "node:fs";
import { x25519 } from "@noble/curves/ed25519.js";

function fromBase64(b64) {
	return new Uint8Array(Buffer.from(b64, "base64"));
}

async function aesGcmDecrypt(key, nonce, ciphertext) {
	const cryptoKey = await crypto.subtle.importKey("raw", key, "AES-GCM", false, ["decrypt"]);
	const plaintext = await crypto.subtle.decrypt(
		{ name: "AES-GCM", iv: nonce },
		cryptoKey,
		ciphertext
	);
	return new Uint8Array(plaintext);
}

async function main() {
	const privateKeyB64 = process.env.HC_STAFF_PRIVATE_KEY;
	const path = process.argv[2];
	if (!privateKeyB64 || !path) {
		console.error("Usage: HC_STAFF_PRIVATE_KEY=<base64> node tools/decrypt-report.mjs report.json");
		process.exit(1);
	}

	const privateKey = fromBase64(privateKeyB64);
	const row = JSON.parse(readFileSync(path, "utf8"));

	const ephemeralPublic = fromBase64(row.sealed_key_ephemeral_public);
	const shared = x25519.getSharedSecret(privateKey, ephemeralPublic);
	const sealInfo = new TextEncoder().encode("HollowChatModerationSeal");
	const combined = new Uint8Array(shared.length + sealInfo.length);
	combined.set(shared, 0);
	combined.set(sealInfo, shared.length);
	const sealKey = new Uint8Array(await crypto.subtle.digest("SHA-256", combined));

	const dataKey = await aesGcmDecrypt(
		sealKey,
		fromBase64(row.sealed_key_nonce),
		fromBase64(row.sealed_key_ciphertext)
	);

	const payload = await aesGcmDecrypt(
		dataKey,
		fromBase64(row.payload_nonce),
		fromBase64(row.payload_ciphertext)
	);

	console.log(JSON.stringify(JSON.parse(new TextDecoder().decode(payload)), null, 2));
}

main();

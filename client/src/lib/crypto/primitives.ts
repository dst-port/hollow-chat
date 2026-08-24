import { x25519, ed25519 } from "@noble/curves/ed25519.js";
import { hkdf } from "@noble/hashes/hkdf.js";
import { sha256 } from "@noble/hashes/sha2.js";

export type KeyPair = { privateKey: Uint8Array; publicKey: Uint8Array };

export function generateX25519KeyPair(): KeyPair {
	const privateKey = x25519.utils.randomSecretKey();
	return { privateKey, publicKey: x25519.getPublicKey(privateKey) };
}

export function generateEd25519KeyPair(): KeyPair {
	const privateKey = ed25519.utils.randomSecretKey();
	return { privateKey, publicKey: ed25519.getPublicKey(privateKey) };
}

export function sign(privateKey: Uint8Array, message: Uint8Array): Uint8Array {
	return ed25519.sign(message, privateKey);
}

export function verify(publicKey: Uint8Array, message: Uint8Array, signature: Uint8Array): boolean {
	return ed25519.verify(signature, message, publicKey);
}

export function dh(privateKey: Uint8Array, publicKey: Uint8Array): Uint8Array {
	return x25519.getSharedSecret(privateKey, publicKey);
}

export function kdf(
	inputKeyMaterial: Uint8Array,
	salt: Uint8Array,
	info: string,
	length: number
): Uint8Array {
	return hkdf(sha256, inputKeyMaterial, salt, new TextEncoder().encode(info), length);
}

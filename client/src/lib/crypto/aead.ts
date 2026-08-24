const NONCE_LEN = 12;

async function importKey(keyBytes: Uint8Array): Promise<CryptoKey> {
	return crypto.subtle.importKey("raw", keyBytes as BufferSource, "AES-GCM", false, [
		"encrypt",
		"decrypt"
	]);
}

export async function encrypt(
	key: Uint8Array,
	plaintext: Uint8Array,
	associatedData: Uint8Array
): Promise<{ nonce: Uint8Array; ciphertext: Uint8Array }> {
	const nonce = crypto.getRandomValues(new Uint8Array(NONCE_LEN));
	const cryptoKey = await importKey(key);
	const result = await crypto.subtle.encrypt(
		{ name: "AES-GCM", iv: nonce as BufferSource, additionalData: associatedData as BufferSource },
		cryptoKey,
		plaintext as BufferSource
	);
	return { nonce, ciphertext: new Uint8Array(result) };
}

export async function decrypt(
	key: Uint8Array,
	nonce: Uint8Array,
	ciphertext: Uint8Array,
	associatedData: Uint8Array
): Promise<Uint8Array> {
	const cryptoKey = await importKey(key);
	const result = await crypto.subtle.decrypt(
		{ name: "AES-GCM", iv: nonce as BufferSource, additionalData: associatedData as BufferSource },
		cryptoKey,
		ciphertext as BufferSource
	);
	return new Uint8Array(result);
}

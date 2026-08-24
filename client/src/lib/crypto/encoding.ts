export function toBase64(bytes: Uint8Array): string {
	let binary = "";
	for (let i = 0; i < bytes.length; i++) binary += String.fromCharCode(bytes[i]);
	return btoa(binary);
}

export function fromBase64(b64: string): Uint8Array {
	const binary = atob(b64);
	const bytes = new Uint8Array(binary.length);
	for (let i = 0; i < binary.length; i++) bytes[i] = binary.charCodeAt(i);
	return bytes;
}

export function utf8Encode(text: string): Uint8Array {
	return new TextEncoder().encode(text);
}

export function utf8Decode(bytes: Uint8Array): string {
	return new TextDecoder().decode(bytes);
}

export function concatBytes(...chunks: Uint8Array[]): Uint8Array {
	const total = chunks.reduce((sum, c) => sum + c.length, 0);
	const out = new Uint8Array(total);
	let offset = 0;
	for (const chunk of chunks) {
		out.set(chunk, offset);
		offset += chunk.length;
	}
	return out;
}

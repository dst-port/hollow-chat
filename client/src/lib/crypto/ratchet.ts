import { hmac } from "@noble/hashes/hmac.js";
import { sha256 } from "@noble/hashes/sha2.js";
import { generateX25519KeyPair, dh, kdf, type KeyPair } from "./primitives";
import { encrypt as aeadEncrypt, decrypt as aeadDecrypt } from "./aead";
import { toBase64, fromBase64, utf8Encode } from "./encoding";

const MAX_SKIPPED_KEYS = 80;

export type Header = {
	dh: string;
	pn: number;
	n: number;
};

type ChainState = {
	key: string | null;
	n: number;
};

export type RatchetState = {
	rootKey: string;
	dhSelfPriv: string;
	dhSelfPub: string;
	dhRemote: string | null;
	sendChain: ChainState;
	recvChain: ChainState;
	skipped: Record<string, string>;
	skippedOrder: string[];
};

function kdfRootKey(rootKey: Uint8Array, dhOut: Uint8Array): { rootKey: Uint8Array; chainKey: Uint8Array } {
	const out = kdf(dhOut, rootKey, "HollowChatRootKDF", 64);
	return { rootKey: out.slice(0, 32), chainKey: out.slice(32, 64) };
}

function kdfChainKey(chainKey: Uint8Array): { chainKey: Uint8Array; messageKey: Uint8Array } {
	return {
		chainKey: hmac(sha256, chainKey, new Uint8Array([0x01])),
		messageKey: hmac(sha256, chainKey, new Uint8Array([0x02]))
	};
}

function headerBytes(header: Header): Uint8Array {
	return utf8Encode(`${header.dh}|${header.pn}|${header.n}`);
}

function messageAeadKey(messageKey: Uint8Array): Uint8Array {
	return kdf(messageKey, new Uint8Array(32), "HollowChatMessageKey", 32);
}

export function initAsSender(sharedSecret: Uint8Array, theirRatchetPublic: Uint8Array): RatchetState {
	const dhSelf = generateX25519KeyPair();
	const dhOut = dh(dhSelf.privateKey, theirRatchetPublic);
	const { rootKey, chainKey } = kdfRootKey(sharedSecret, dhOut);
	return {
		rootKey: toBase64(rootKey),
		dhSelfPriv: toBase64(dhSelf.privateKey),
		dhSelfPub: toBase64(dhSelf.publicKey),
		dhRemote: toBase64(theirRatchetPublic),
		sendChain: { key: toBase64(chainKey), n: 0 },
		recvChain: { key: null, n: 0 },
		skipped: {},
		skippedOrder: []
	};
}

export function initAsReceiver(sharedSecret: Uint8Array, myRatchetKeyPair: KeyPair): RatchetState {
	return {
		rootKey: toBase64(sharedSecret),
		dhSelfPriv: toBase64(myRatchetKeyPair.privateKey),
		dhSelfPub: toBase64(myRatchetKeyPair.publicKey),
		dhRemote: null,
		sendChain: { key: null, n: 0 },
		recvChain: { key: null, n: 0 },
		skipped: {},
		skippedOrder: []
	};
}

export async function ratchetEncrypt(
	state: RatchetState,
	plaintext: Uint8Array
): Promise<{ header: Header; nonce: string; ciphertext: string }> {
	if (!state.sendChain.key) throw new Error("no send chain established");
	const { chainKey, messageKey } = kdfChainKey(fromBase64(state.sendChain.key));
	state.sendChain.key = toBase64(chainKey);

	const header: Header = { dh: state.dhSelfPub, pn: 0, n: state.sendChain.n };
	state.sendChain.n += 1;

	const aeadKey = messageAeadKey(messageKey);
	const { nonce, ciphertext } = await aeadEncrypt(aeadKey, plaintext, headerBytes(header));

	return { header, nonce: toBase64(nonce), ciphertext: toBase64(ciphertext) };
}

function dhRatchetStep(state: RatchetState, theirNewRatchetPublic: Uint8Array) {
	state.dhRemote = toBase64(theirNewRatchetPublic);
	{
		const dhOut = dh(fromBase64(state.dhSelfPriv), theirNewRatchetPublic);
		const { rootKey, chainKey } = kdfRootKey(fromBase64(state.rootKey), dhOut);
		state.rootKey = toBase64(rootKey);
		state.recvChain = { key: toBase64(chainKey), n: 0 };
	}

	const newSelf = generateX25519KeyPair();
	state.dhSelfPriv = toBase64(newSelf.privateKey);
	state.dhSelfPub = toBase64(newSelf.publicKey);

	{
		const dhOut = dh(newSelf.privateKey, theirNewRatchetPublic);
		const { rootKey, chainKey } = kdfRootKey(fromBase64(state.rootKey), dhOut);
		state.rootKey = toBase64(rootKey);
		state.sendChain = { key: toBase64(chainKey), n: 0 };
	}
}

function skipKey(dhPub: string, n: number): string {
	return `${dhPub}|${n}`;
}

function storeSkippedKey(state: RatchetState, dhPub: string, n: number, messageKey: Uint8Array) {
	const key = skipKey(dhPub, n);
	state.skipped[key] = toBase64(messageKey);
	state.skippedOrder.push(key);
	while (state.skippedOrder.length > MAX_SKIPPED_KEYS) {
		const oldest = state.skippedOrder.shift();
		if (oldest) delete state.skipped[oldest];
	}
}

function skipMessageKeys(state: RatchetState, until: number) {
	if (!state.recvChain.key) return;
	if (state.recvChain.n > until) return;
	let chainKey = fromBase64(state.recvChain.key);
	while (state.recvChain.n < until) {
		const { chainKey: nextChain, messageKey } = kdfChainKey(chainKey);
		if (state.dhRemote) storeSkippedKey(state, state.dhRemote, state.recvChain.n, messageKey);
		chainKey = nextChain;
		state.recvChain.n += 1;
	}
	state.recvChain.key = toBase64(chainKey);
}

export async function ratchetDecrypt(
	state: RatchetState,
	header: Header,
	nonceB64: string,
	ciphertextB64: string
): Promise<Uint8Array> {
	const nonce = fromBase64(nonceB64);
	const ciphertext = fromBase64(ciphertextB64);

	const skipped = state.skipped[skipKey(header.dh, header.n)];
	if (skipped) {
		const aeadKey = messageAeadKey(fromBase64(skipped));
		const key = skipKey(header.dh, header.n);
		delete state.skipped[key];
		state.skippedOrder = state.skippedOrder.filter((k) => k !== key);
		return aeadDecrypt(aeadKey, nonce, ciphertext, headerBytes(header));
	}

	if (header.dh !== state.dhRemote) {
		skipMessageKeys(state, header.pn);
		dhRatchetStep(state, fromBase64(header.dh));
	}

	skipMessageKeys(state, header.n);

	if (!state.recvChain.key) throw new Error("no recv chain established");
	const { chainKey, messageKey } = kdfChainKey(fromBase64(state.recvChain.key));
	state.recvChain.key = toBase64(chainKey);
	state.recvChain.n += 1;

	const aeadKey = messageAeadKey(messageKey);
	return aeadDecrypt(aeadKey, nonce, ciphertext, headerBytes(header));
}

import { kdf } from "./primitives";
import * as aead from "./aead";
import { toBase64, fromBase64, utf8Encode, utf8Decode } from "./encoding";
import { encryptForPeer, decryptFromPeer } from "./dm";
import {
	loadSendState,
	saveSendState,
	loadReceiveState,
	saveReceiveState,
	type ChainState
} from "./group-key-store";

export const GROUP_ENVELOPE_PREFIX = "hcGE2EE1:";

type GroupEnvelope = {
	v: 1;
	iteration: number;
	nonce: string;
	ciphertext: string;
};

type DistributionPayload = {
	channelId: string;
	chainKey: string;
	iteration: number;
};

function randomChainKey(): Uint8Array {
	return crypto.getRandomValues(new Uint8Array(32));
}

function deriveMessageKey(chainKey: Uint8Array): Uint8Array {
	return kdf(chainKey, new Uint8Array(0), "hollowchat-group-message-key", 32);
}

function advanceChain(chainKey: Uint8Array): Uint8Array {
	return kdf(chainKey, new Uint8Array(0), "hollowchat-group-chain-key", 32);
}

export function isGroupEnvelope(content: string): boolean {
	return content.startsWith(GROUP_ENVELOPE_PREFIX);
}

export function getOrCreateSendState(myUsername: string, channelId: string): ChainState {
	const existing = loadSendState(myUsername, channelId);
	if (existing) return existing;
	const fresh: ChainState = { chainKey: toBase64(randomChainKey()), iteration: 0 };
	saveSendState(myUsername, channelId, fresh);
	return fresh;
}

export async function packageDistribution(
	token: string,
	myUsername: string,
	channelId: string,
	state: ChainState,
	recipientUsername: string
): Promise<string> {
	const payload: DistributionPayload = { channelId, chainKey: state.chainKey, iteration: state.iteration };
	return encryptForPeer(token, myUsername, recipientUsername, JSON.stringify(payload));
}

export async function absorbDistribution(
	myUsername: string,
	senderUsername: string,
	ciphertext: string
): Promise<void> {
	const decrypted = await decryptFromPeer(myUsername, senderUsername, ciphertext);
	const payload = JSON.parse(decrypted) as DistributionPayload;
	saveReceiveState(myUsername, payload.channelId, senderUsername, {
		chainKey: payload.chainKey,
		iteration: payload.iteration
	});
}

export async function encryptForChannel(
	myUsername: string,
	channelId: string,
	plaintext: string
): Promise<string> {
	const state = getOrCreateSendState(myUsername, channelId);
	const chainKeyBytes = fromBase64(state.chainKey);
	const messageKey = deriveMessageKey(chainKeyBytes);
	const associatedData = utf8Encode(`${channelId}:${state.iteration}`);
	const { nonce, ciphertext } = await aead.encrypt(messageKey, utf8Encode(plaintext), associatedData);

	const envelope: GroupEnvelope = {
		v: 1,
		iteration: state.iteration,
		nonce: toBase64(nonce),
		ciphertext: toBase64(ciphertext)
	};

	const nextState: ChainState = { chainKey: toBase64(advanceChain(chainKeyBytes)), iteration: state.iteration + 1 };
	saveSendState(myUsername, channelId, nextState);

	return GROUP_ENVELOPE_PREFIX + JSON.stringify(envelope);
}

export async function decryptFromChannel(
	myUsername: string,
	channelId: string,
	senderUsername: string,
	content: string
): Promise<string> {
	if (!isGroupEnvelope(content)) return content;
	const envelope = JSON.parse(content.slice(GROUP_ENVELOPE_PREFIX.length)) as GroupEnvelope;

	const state = loadReceiveState(myUsername, channelId, senderUsername);
	if (!state) throw new Error("no sender key received yet");

	let chainKeyBytes = fromBase64(state.chainKey);
	let iteration = state.iteration;

	if (envelope.iteration < iteration) {
		throw new Error("message key already used or out of order");
	}

	while (iteration < envelope.iteration) {
		chainKeyBytes = advanceChain(chainKeyBytes);
		iteration++;
	}

	const messageKey = deriveMessageKey(chainKeyBytes);
	const associatedData = utf8Encode(`${channelId}:${envelope.iteration}`);
	const plaintext = await aead.decrypt(
		messageKey,
		fromBase64(envelope.nonce),
		fromBase64(envelope.ciphertext),
		associatedData
	);

	saveReceiveState(myUsername, channelId, senderUsername, {
		chainKey: toBase64(advanceChain(chainKeyBytes)),
		iteration: iteration + 1
	});

	return utf8Decode(plaintext);
}

export function markDistributedTo(myUsername: string, channelId: string, memberIds: string[]) {
	saveSendState(myUsername, channelId, {
		...getOrCreateSendState(myUsername, channelId),
		distributedTo: [...memberIds].sort()
	});
}

export function needsRedistribution(myUsername: string, channelId: string, currentMemberIds: string[]): boolean {
	const state = loadSendState(myUsername, channelId);
	if (!state || !state.distributedTo) return true;
	const current = [...currentMemberIds].sort();
	if (state.distributedTo.length !== current.length) return true;
	return state.distributedTo.some((id, i) => id !== current[i]);
}

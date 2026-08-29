import { kdf } from "./primitives";
import * as aead from "./aead";
import { toBase64, fromBase64, utf8Encode, utf8Decode } from "./encoding";
import { encryptForPeer, decryptFromPeer } from "./dm";
import { listSenderKeys, listDmSenderKeys } from "$lib/api/client";
import {
	loadSendState,
	saveSendState,
	loadReceiveState,
	saveReceiveState,
	sendKey,
	type ChainState
} from "./group-key-store";
import { deviceSync } from "$lib/devicelink/sync";

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

export async function getOrCreateSendState(myUsername: string, channelId: string): Promise<ChainState> {
	const existing = loadSendState(myUsername, channelId);
	if (existing) return existing;

	const decision = await deviceSync.claimNewSession(myUsername, `channel:${channelId}`);
	if (decision === "wait-for-sync") {
		await deviceSync.waitForSync(myUsername, sendKey(myUsername, channelId));
		const synced = loadSendState(myUsername, channelId);
		if (synced) return synced;
	}

	const fresh: ChainState = { chainKey: toBase64(randomChainKey()), iteration: 0 };
	saveSendState(myUsername, channelId, fresh);
	deviceSync.broadcastChange(myUsername, sendKey(myUsername, channelId), JSON.stringify(fresh));
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
	const state = await getOrCreateSendState(myUsername, channelId);
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
	deviceSync.broadcastChange(myUsername, sendKey(myUsername, channelId), JSON.stringify(nextState));

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

export async function absorbSenderKeyFor(
	token: string,
	myUsername: string,
	channelId: string,
	senderUsername: string
): Promise<boolean> {
	try {
		const pending = await listSenderKeys(token, channelId);
		const entry = pending.find((p) => p.sender_username === senderUsername);
		if (!entry) return false;
		await absorbDistribution(myUsername, entry.sender_username, entry.ciphertext);
		return true;
	} catch {
		return false;
	}
}

export async function absorbDmSenderKeyFor(
	token: string,
	myUsername: string,
	dmId: string,
	senderUsername: string
): Promise<boolean> {
	try {
		const pending = await listDmSenderKeys(token, dmId);
		const entry = pending.find((p) => p.sender_username === senderUsername);
		if (!entry) return false;
		await absorbDistribution(myUsername, entry.sender_username, entry.ciphertext);
		return true;
	} catch {
		return false;
	}
}

export async function markDistributedTo(myUsername: string, channelId: string, memberIds: string[]) {
	const state: ChainState = {
		...(await getOrCreateSendState(myUsername, channelId)),
		distributedTo: [...memberIds].sort()
	};
	saveSendState(myUsername, channelId, state);
	deviceSync.broadcastChange(myUsername, sendKey(myUsername, channelId), JSON.stringify(state));
}

export function needsRedistribution(myUsername: string, channelId: string, currentMemberIds: string[]): boolean {
	const state = loadSendState(myUsername, channelId);
	if (!state || !state.distributedTo) return true;
	const current = [...currentMemberIds].sort();
	if (state.distributedTo.length !== current.length) return true;
	return state.distributedTo.some((id, i) => id !== current[i]);
}

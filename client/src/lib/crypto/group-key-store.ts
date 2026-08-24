export type ChainState = {
	chainKey: string;
	iteration: number;
	distributedTo?: string[];
};

export function sendKey(username: string, channelId: string): string {
	return `hollowchat_groupsend_${username}_${channelId}`;
}

function receiveKey(username: string, channelId: string, senderUsername: string): string {
	return `hollowchat_grouprecv_${username}_${channelId}_${senderUsername}`;
}

function parse(raw: string | null): ChainState | null {
	if (!raw) return null;
	try {
		return JSON.parse(raw) as ChainState;
	} catch {
		return null;
	}
}

export function loadSendState(username: string, channelId: string): ChainState | null {
	return parse(localStorage.getItem(sendKey(username, channelId)));
}

export function saveSendState(username: string, channelId: string, state: ChainState) {
	localStorage.setItem(sendKey(username, channelId), JSON.stringify(state));
}

export function loadReceiveState(
	username: string,
	channelId: string,
	senderUsername: string
): ChainState | null {
	return parse(localStorage.getItem(receiveKey(username, channelId, senderUsername)));
}

export function saveReceiveState(
	username: string,
	channelId: string,
	senderUsername: string,
	state: ChainState
) {
	localStorage.setItem(receiveKey(username, channelId, senderUsername), JSON.stringify(state));
}

export function renameAllGroupKeys(oldUsername: string, newUsername: string) {
	const prefixes = [`hollowchat_groupsend_${oldUsername}_`, `hollowchat_grouprecv_${oldUsername}_`];
	const renames: [string, string][] = [];
	for (let i = 0; i < localStorage.length; i++) {
		const key = localStorage.key(i);
		if (!key) continue;
		for (const prefix of prefixes) {
			if (key.startsWith(prefix)) {
				const suffix = key.slice(prefix.length);
				const newPrefix = prefix.replace(`_${oldUsername}_`, `_${newUsername}_`);
				renames.push([key, newPrefix + suffix]);
			}
		}
	}
	for (const [oldKey, newKey] of renames) {
		const raw = localStorage.getItem(oldKey);
		if (raw) localStorage.setItem(newKey, raw);
		localStorage.removeItem(oldKey);
	}
}

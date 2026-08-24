const BASE_URL = "http://127.0.0.1:8080";

export class ApiError extends Error {
	status: number;

	constructor(status: number, message: string) {
		super(message);
		this.status = status;
	}
}

const REQUEST_TIMEOUT_MS = 8000;

async function request<T>(path: string, options: RequestInit = {}): Promise<T> {
	const controller = new AbortController();
	const timeout = setTimeout(() => controller.abort(), REQUEST_TIMEOUT_MS);

	let response: Response;
	try {
		response = await fetch(`${BASE_URL}${path}`, {
			...options,
			signal: controller.signal,
			headers: {
				"content-type": "application/json",
				...options.headers
			}
		});
	} catch (err) {
		if (err instanceof DOMException && err.name === "AbortError") {
			throw new ApiError(0, "request timed out");
		}
		throw new ApiError(0, "network error");
	} finally {
		clearTimeout(timeout);
	}

	if (!response.ok) {
		const body = await response.json().catch(() => ({ error: response.statusText }));
		throw new ApiError(response.status, body.error ?? "request failed");
	}

	if (response.status === 204) {
		return undefined as T;
	}

	const text = await response.text();
	return text ? JSON.parse(text) : (undefined as T);
}

export type RegisterResponse = {
	username: string;
	password: string;
};

export type LoginResponse = {
	token: string;
	expires_at: string;
};

export type MeResponse = {
	username: string;
};

export function register(username: string) {
	return request<RegisterResponse>("/auth/register", {
		method: "POST",
		body: JSON.stringify({ username })
	});
}

export function login(username: string, password: string) {
	return request<LoginResponse>("/auth/login", {
		method: "POST",
		body: JSON.stringify({ username, password })
	});
}

export function me(token: string) {
	return request<MeResponse>("/auth/me", {
		headers: { authorization: `Bearer ${token}` }
	});
}

export type FrequentEmoji = {
	emoji: string;
	count: number;
};

export function recordEmojiUse(token: string, emoji: string) {
	return request<void>("/emoji/use", {
		method: "POST",
		headers: { authorization: `Bearer ${token}` },
		body: JSON.stringify({ emoji })
	});
}

export function frequentEmoji(token: string, limit = 5) {
	return request<FrequentEmoji[]>(`/emoji/frequent?limit=${limit}`, {
		headers: { authorization: `Bearer ${token}` }
	});
}

export type ApiChannel = {
	id: string;
	name: string;
	type: "text" | "voice";
	category: string | null;
};

export type ApiServer = {
	id: string;
	name: string;
	owner_id: string;
	channels: ApiChannel[];
};

export type ApiMessage = {
	id: string;
	author_id: string | null;
	author: string;
	content: string;
	timestamp: string;
};

export function listServers(token: string) {
	return request<ApiServer[]>("/servers", {
		headers: { authorization: `Bearer ${token}` }
	});
}

export function createServer(token: string, name: string) {
	return request<ApiServer>("/servers", {
		method: "POST",
		headers: { authorization: `Bearer ${token}` },
		body: JSON.stringify({ name })
	});
}

export function renameServer(token: string, serverId: string, name: string) {
	return request<Omit<ApiServer, "channels">>(`/servers/${serverId}`, {
		method: "PATCH",
		headers: { authorization: `Bearer ${token}` },
		body: JSON.stringify({ name })
	});
}

export function leaveServer(token: string, serverId: string) {
	return request<void>(`/servers/${serverId}/leave`, {
		method: "DELETE",
		headers: { authorization: `Bearer ${token}` }
	});
}

export function createChannel(
	token: string,
	serverId: string,
	name: string,
	type: "text" | "voice",
	category?: string
) {
	return request<ApiChannel>(`/servers/${serverId}/channels`, {
		method: "POST",
		headers: { authorization: `Bearer ${token}` },
		body: JSON.stringify({ name, type, category })
	});
}

export type ApiFriend = {
	id: string;
	username: string;
};

export type ApiFriendRequest = {
	id: string;
	username: string;
	direction: "incoming" | "outgoing";
};

export function listFriends(token: string) {
	return request<ApiFriend[]>("/friends", {
		headers: { authorization: `Bearer ${token}` }
	});
}

export function removeFriend(token: string, userId: string) {
	return request<void>(`/friends/${userId}`, {
		method: "DELETE",
		headers: { authorization: `Bearer ${token}` }
	});
}

export function listFriendRequests(token: string) {
	return request<ApiFriendRequest[]>("/friends/requests", {
		headers: { authorization: `Bearer ${token}` }
	});
}

export function sendFriendRequest(token: string, username: string) {
	return request<{ result: "sent" | "accepted"; id?: string }>("/friends/requests", {
		method: "POST",
		headers: { authorization: `Bearer ${token}` },
		body: JSON.stringify({ username })
	});
}

export function acceptFriendRequest(token: string, requestId: string) {
	return request<void>(`/friends/requests/${requestId}/accept`, {
		method: "POST",
		headers: { authorization: `Bearer ${token}` }
	});
}

export function declineFriendRequest(token: string, requestId: string) {
	return request<void>(`/friends/requests/${requestId}`, {
		method: "DELETE",
		headers: { authorization: `Bearer ${token}` }
	});
}

export type ApiDmChannel = {
	id: string;
	peer_id: string;
	peer_username: string;
};

export function listDms(token: string) {
	return request<ApiDmChannel[]>("/dms", {
		headers: { authorization: `Bearer ${token}` }
	});
}

export function openDm(token: string, username: string) {
	return request<ApiDmChannel>("/dms", {
		method: "POST",
		headers: { authorization: `Bearer ${token}` },
		body: JSON.stringify({ username })
	});
}

export function listDmMessages(
	token: string,
	dmId: string,
	opts: { before?: string; after?: string; limit?: number } = {}
) {
	const params = new URLSearchParams({ limit: String(opts.limit ?? 50) });
	if (opts.before) params.set("before", opts.before);
	if (opts.after) params.set("after", opts.after);
	return request<ApiMessage[]>(`/dms/${dmId}/messages?${params}`, {
		headers: { authorization: `Bearer ${token}` }
	});
}

export function sendDmMessage(token: string, dmId: string, content: string) {
	return request<ApiMessage>(`/dms/${dmId}/messages`, {
		method: "POST",
		headers: { authorization: `Bearer ${token}` },
		body: JSON.stringify({ content })
	});
}

export type OneTimePrekeyUpload = {
	key_id: number;
	public_key: string;
};

export type UploadBundleRequest = {
	ed25519_public: string;
	x25519_public: string;
	signed_prekey_id: number;
	signed_prekey_public: string;
	signed_prekey_signature: string;
	one_time_prekeys: OneTimePrekeyUpload[];
};

export function uploadKeyBundle(token: string, bundle: UploadBundleRequest) {
	return request<void>("/keys/bundle", {
		method: "PUT",
		headers: { authorization: `Bearer ${token}` },
		body: JSON.stringify(bundle)
	});
}

export type PrekeyBundleResponse = {
	ed25519_public: string;
	x25519_public: string;
	signed_prekey_id: number;
	signed_prekey_public: string;
	signed_prekey_signature: string;
	one_time_prekey: { key_id: number; public_key: string } | null;
};

export function fetchKeyBundle(token: string, username: string) {
	return request<PrekeyBundleResponse>(`/keys/bundle/${encodeURIComponent(username)}`, {
		headers: { authorization: `Bearer ${token}` }
	});
}

export function prekeyCount(token: string) {
	return request<{ count: number }>("/keys/prekey-count", {
		headers: { authorization: `Bearer ${token}` }
	});
}

export type ApiMember = {
	id: string;
	username: string;
};

export function listMembers(token: string, serverId: string) {
	return request<ApiMember[]>(`/servers/${serverId}/members`, {
		headers: { authorization: `Bearer ${token}` }
	});
}

export function listMessages(
	token: string,
	channelId: string,
	opts: { before?: string; after?: string; limit?: number } = {}
) {
	const params = new URLSearchParams({ limit: String(opts.limit ?? 50) });
	if (opts.before) params.set("before", opts.before);
	if (opts.after) params.set("after", opts.after);
	return request<ApiMessage[]>(`/channels/${channelId}/messages?${params}`, {
		headers: { authorization: `Bearer ${token}` }
	});
}

export function sendMessage(token: string, channelId: string, content: string) {
	return request<ApiMessage>(`/channels/${channelId}/messages`, {
		method: "POST",
		headers: { authorization: `Bearer ${token}` },
		body: JSON.stringify({ content })
	});
}

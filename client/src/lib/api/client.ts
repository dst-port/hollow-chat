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

export function fileUrl(id: string, filename: string): string {
	return `${BASE_URL}/files/${id}/${encodeURIComponent(filename)}`;
}

export async function uploadFile(token: string, file: File): Promise<ApiAttachment> {
	const form = new FormData();
	form.append("file", file, file.name);

	let response: Response;
	try {
		response = await fetch(`${BASE_URL}/files`, {
			method: "PUT",
			headers: { authorization: `Bearer ${token}` },
			body: form
		});
	} catch {
		throw new ApiError(0, "network error");
	}

	if (!response.ok) {
		const body = await response.json().catch(() => ({ error: response.statusText }));
		throw new ApiError(response.status, body.error ?? "upload failed");
	}

	return response.json();
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
	id: string;
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

export function changeUsername(token: string, username: string) {
	return request<{ username: string }>("/auth/username", {
		method: "PATCH",
		headers: { authorization: `Bearer ${token}` },
		body: JSON.stringify({ username })
	});
}

export function regeneratePassword(token: string) {
	return request<{ password: string }>("/auth/regenerate-password", {
		method: "POST",
		headers: { authorization: `Bearer ${token}` }
	});
}

export type ApiSession = {
	id: string;
	user_agent: string | null;
	ip_address: string | null;
	created_at: string;
	current: boolean;
};

export function listSessions(token: string) {
	return request<ApiSession[]>("/auth/sessions", {
		headers: { authorization: `Bearer ${token}` }
	});
}

export function revokeSession(token: string, id: string) {
	return request<void>(`/auth/sessions/${id}`, {
		method: "DELETE",
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
	slowmode_seconds: number;
};

export type ApiServer = {
	id: string;
	name: string;
	owner_id: string;
	channels: ApiChannel[];
};

export type ApiAttachment = {
	id: string;
	filename: string;
	mime_type: string;
	size_bytes: number;
};

export type ApiReplyPreview = {
	id: string;
	author: string;
	content: string | null;
	has_attachment: boolean;
};

export type ApiReaction = {
	emoji: string;
	count: number;
	reacted: boolean;
};

export type ApiMessage = {
	id: string;
	author_id: string | null;
	author: string;
	content: string | null;
	attachment: ApiAttachment | null;
	reply_to: ApiReplyPreview | null;
	reactions: ApiReaction[];
	pinned: boolean;
	timestamp: string;
	edited_at: string | null;
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

export function setSlowmode(token: string, serverId: string, channelId: string, seconds: number) {
	return request<ApiChannel>(`/servers/${serverId}/channels/${channelId}/slowmode`, {
		method: "PATCH",
		headers: { authorization: `Bearer ${token}` },
		body: JSON.stringify({ seconds })
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

export type ApiBlockedUser = {
	id: string;
	username: string;
};

export function listBlocked(token: string) {
	return request<ApiBlockedUser[]>("/blocks", {
		headers: { authorization: `Bearer ${token}` }
	});
}

export function blockUser(token: string, userId: string) {
	return request<void>(`/blocks/${userId}`, {
		method: "PUT",
		headers: { authorization: `Bearer ${token}` }
	});
}

export function unblockUser(token: string, userId: string) {
	return request<void>(`/blocks/${userId}`, {
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

export function sendDmMessage(
	token: string,
	dmId: string,
	content: string | null,
	attachmentId?: string,
	replyToId?: string
) {
	return request<ApiMessage>(`/dms/${dmId}/messages`, {
		method: "POST",
		headers: { authorization: `Bearer ${token}` },
		body: JSON.stringify({ content, attachment_id: attachmentId, reply_to_id: replyToId })
	});
}

export type MessageScope = "channel" | "dm";

function messagesBase(scope: MessageScope, id: string): string {
	return scope === "channel" ? `/channels/${id}` : `/dms/${id}`;
}

export function editMessage(
	token: string,
	scope: MessageScope,
	id: string,
	messageId: string,
	content: string
) {
	return request<ApiMessage>(`${messagesBase(scope, id)}/messages/${messageId}`, {
		method: "PATCH",
		headers: { authorization: `Bearer ${token}` },
		body: JSON.stringify({ content })
	});
}

export function deleteMessage(token: string, scope: MessageScope, id: string, messageId: string) {
	return request<void>(`${messagesBase(scope, id)}/messages/${messageId}`, {
		method: "DELETE",
		headers: { authorization: `Bearer ${token}` }
	});
}

export function pinMessage(token: string, scope: MessageScope, id: string, messageId: string) {
	return request<void>(`${messagesBase(scope, id)}/messages/${messageId}/pin`, {
		method: "PUT",
		headers: { authorization: `Bearer ${token}` }
	});
}

export function unpinMessage(token: string, scope: MessageScope, id: string, messageId: string) {
	return request<void>(`${messagesBase(scope, id)}/messages/${messageId}/pin`, {
		method: "DELETE",
		headers: { authorization: `Bearer ${token}` }
	});
}

export function listPinned(token: string, scope: MessageScope, id: string) {
	return request<ApiMessage[]>(`${messagesBase(scope, id)}/pinned`, {
		headers: { authorization: `Bearer ${token}` }
	});
}

export function addReaction(
	token: string,
	scope: MessageScope,
	id: string,
	messageId: string,
	emoji: string
) {
	return request<void>(
		`${messagesBase(scope, id)}/messages/${messageId}/reactions/${encodeURIComponent(emoji)}`,
		{ method: "PUT", headers: { authorization: `Bearer ${token}` } }
	);
}

export function removeReaction(
	token: string,
	scope: MessageScope,
	id: string,
	messageId: string,
	emoji: string
) {
	return request<void>(
		`${messagesBase(scope, id)}/messages/${messageId}/reactions/${encodeURIComponent(emoji)}`,
		{ method: "DELETE", headers: { authorization: `Bearer ${token}` } }
	);
}

export type ApiThread = {
	id: string;
	channel_id: string;
	parent_message_id: string | null;
	name: string;
	created_by: string | null;
	created_by_username: string | null;
	archived: boolean;
	created_at: string;
	message_count: number;
	last_message_at: string | null;
};

export function listThreads(token: string, channelId: string) {
	return request<ApiThread[]>(`/channels/${channelId}/threads`, {
		headers: { authorization: `Bearer ${token}` }
	});
}

export function createThread(
	token: string,
	channelId: string,
	messageId: string,
	name?: string
) {
	return request<ApiThread>(`/channels/${channelId}/messages/${messageId}/threads`, {
		method: "POST",
		headers: { authorization: `Bearer ${token}` },
		body: JSON.stringify({ name })
	});
}

export function setThreadArchived(
	token: string,
	channelId: string,
	threadId: string,
	archived: boolean
) {
	return request<ApiThread>(`/channels/${channelId}/threads/${threadId}`, {
		method: "PATCH",
		headers: { authorization: `Bearer ${token}` },
		body: JSON.stringify({ archived })
	});
}

export function listThreadMessages(token: string, channelId: string, threadId: string) {
	return request<ApiMessage[]>(`/channels/${channelId}/threads/${threadId}/messages`, {
		headers: { authorization: `Bearer ${token}` }
	});
}

export function sendThreadMessage(
	token: string,
	channelId: string,
	threadId: string,
	content: string | null,
	attachmentId?: string,
	replyToId?: string
) {
	return request<ApiMessage>(`/channels/${channelId}/threads/${threadId}/messages`, {
		method: "POST",
		headers: { authorization: `Bearer ${token}` },
		body: JSON.stringify({ content, attachment_id: attachmentId, reply_to_id: replyToId })
	});
}

export type BillingStatus = {
	tier: "free" | "premium";
	subscription_status: string | null;
};

export function billingStatus(token: string) {
	return request<BillingStatus>("/billing/status", {
		headers: { authorization: `Bearer ${token}` }
	});
}

export function createCheckout(token: string) {
	return request<{ url: string }>("/billing/checkout", {
		method: "POST",
		headers: { authorization: `Bearer ${token}` }
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

export function getServerInvite(token: string, serverId: string) {
	return request<{ code: string }>(`/servers/${serverId}/invite`, {
		headers: { authorization: `Bearer ${token}` }
	});
}

export function joinServer(token: string, code: string) {
	return request<ApiServer>("/servers/join", {
		method: "POST",
		headers: { authorization: `Bearer ${token}` },
		body: JSON.stringify({ code })
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

export function sendMessage(
	token: string,
	channelId: string,
	content: string | null,
	attachmentId?: string,
	replyToId?: string
) {
	return request<ApiMessage>(`/channels/${channelId}/messages`, {
		method: "POST",
		headers: { authorization: `Bearer ${token}` },
		body: JSON.stringify({ content, attachment_id: attachmentId, reply_to_id: replyToId })
	});
}

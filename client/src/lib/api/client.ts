const BASE_URL = "http://127.0.0.1:8080";
export const WS_BASE_URL = BASE_URL.replace(/^http/, "ws");

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

export function resolveUrl(path: string, token?: string | null): string {
	if (/^https?:\/\//i.test(path)) return path;
	if (!token) return `${BASE_URL}${path}`;
	const separator = path.includes("?") ? "&" : "?";
	return `${BASE_URL}${path}${separator}token=${encodeURIComponent(token)}`;
}

export function bannerBackground(
	profile: { banner_url: string | null; banner_color: string | null; accent_color: string | null } | null | undefined,
	token?: string | null
): string {
	if (profile?.banner_url) return `url(${resolveUrl(profile.banner_url, token)}) center/cover`;
	const base = profile?.banner_color || profile?.accent_color || "#5865f2";
	return `linear-gradient(135deg, ${base}, color-mix(in srgb, ${base} 45%, black))`;
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
	requires_totp: boolean;
	challenge_id: string | null;
	token: string | null;
	expires_at: string | null;
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

export function completeTotpLogin(challengeId: string, code: string) {
	return request<LoginResponse>("/auth/login/totp", {
		method: "POST",
		body: JSON.stringify({ challenge_id: challengeId, code })
	});
}

export type TotpStatus = { enabled: boolean };

export function fetchTotpStatus(token: string) {
	return request<TotpStatus>("/auth/2fa/status", {
		headers: { authorization: `Bearer ${token}` }
	});
}

export type TotpSetup = { secret: string; otpauth_url: string };

export function setupTotp(token: string) {
	return request<TotpSetup>("/auth/2fa/setup", {
		method: "POST",
		headers: { authorization: `Bearer ${token}` }
	});
}

export type TotpVerifyResult = { backup_codes: string[] };

export function verifyTotp(token: string, code: string) {
	return request<TotpVerifyResult>("/auth/2fa/verify", {
		method: "POST",
		headers: { authorization: `Bearer ${token}` },
		body: JSON.stringify({ code })
	});
}

export function disableTotp(token: string, code: string) {
	return request<void>("/auth/2fa/disable", {
		method: "POST",
		headers: { authorization: `Bearer ${token}` },
		body: JSON.stringify({ code })
	});
}

export function regenerateBackupCodes(token: string, code: string) {
	return request<TotpVerifyResult>("/auth/2fa/backup-codes", {
		method: "POST",
		headers: { authorization: `Bearer ${token}` },
		body: JSON.stringify({ code })
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
	icon_url: string | null;
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

export function setServerIcon(token: string, serverId: string, attachmentId: string) {
	return request<Omit<ApiServer, "channels">>(`/servers/${serverId}/icon`, {
		method: "PUT",
		headers: { authorization: `Bearer ${token}` },
		body: JSON.stringify({ attachment_id: attachmentId })
	});
}

export function clearServerIcon(token: string, serverId: string) {
	return request<Omit<ApiServer, "channels">>(`/servers/${serverId}/icon`, {
		method: "DELETE",
		headers: { authorization: `Bearer ${token}` }
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
	display_name: string | null;
	presence: "online" | "idle" | "dnd" | "offline";
	status_text: string | null;
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

export function listMutualFriends(token: string, username: string) {
	return request<ApiFriend[]>(`/friends/mutual/${encodeURIComponent(username)}`, {
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

export type ApiSenderKey = {
	sender_id: string;
	sender_username: string;
	ciphertext: string;
};

export function publishSenderKeys(
	token: string,
	channelId: string,
	entries: { recipient_id: string; ciphertext: string }[]
) {
	return request<void>(`/channels/${channelId}/sender-keys`, {
		method: "POST",
		headers: { authorization: `Bearer ${token}` },
		body: JSON.stringify({ entries })
	});
}

export function listSenderKeys(token: string, channelId: string) {
	return request<ApiSenderKey[]>(`/channels/${channelId}/sender-keys`, {
		headers: { authorization: `Bearer ${token}` }
	});
}

export type ApiRole = {
	id: string;
	server_id: string;
	name: string;
	color: string;
	permissions: number;
	position: number;
};

export type ApiMember = {
	id: string;
	username: string;
	is_owner: boolean;
	roles: ApiRole[];
};

export function listMembers(token: string, serverId: string) {
	return request<ApiMember[]>(`/servers/${serverId}/members`, {
		headers: { authorization: `Bearer ${token}` }
	});
}

export const PERMISSIONS = {
	MANAGE_CHANNELS: 1 << 0,
	MANAGE_ROLES: 1 << 1,
	MANAGE_SERVER: 1 << 2,
	KICK_MEMBERS: 1 << 3,
	BAN_MEMBERS: 1 << 4,
	MANAGE_MESSAGES: 1 << 5,
	CREATE_INVITE: 1 << 6
} as const;

export const PERMISSION_LABELS: { key: keyof typeof PERMISSIONS; label: string; description: string }[] = [
	{ key: "MANAGE_CHANNELS", label: "Manage Channels", description: "Create channels and change slowmode" },
	{ key: "MANAGE_ROLES", label: "Manage Roles", description: "Create roles and assign them to members" },
	{ key: "MANAGE_SERVER", label: "Manage Server", description: "Rename the server" },
	{ key: "KICK_MEMBERS", label: "Kick Members", description: "Remove members from the server" },
	{ key: "BAN_MEMBERS", label: "Ban Members", description: "Ban and unban members" },
	{ key: "MANAGE_MESSAGES", label: "Manage Messages", description: "Delete and pin others' messages" },
	{ key: "CREATE_INVITE", label: "Create Invite", description: "Generate an invite link" }
];

export function listRoles(token: string, serverId: string) {
	return request<ApiRole[]>(`/servers/${serverId}/roles`, {
		headers: { authorization: `Bearer ${token}` }
	});
}

export function createRole(token: string, serverId: string, name: string, color: string, permissions: number) {
	return request<ApiRole>(`/servers/${serverId}/roles`, {
		method: "POST",
		headers: { authorization: `Bearer ${token}` },
		body: JSON.stringify({ name, color, permissions })
	});
}

export function updateRole(
	token: string,
	serverId: string,
	roleId: string,
	patch: { name?: string; color?: string; permissions?: number; position?: number }
) {
	return request<ApiRole>(`/servers/${serverId}/roles/${roleId}`, {
		method: "PATCH",
		headers: { authorization: `Bearer ${token}` },
		body: JSON.stringify(patch)
	});
}

export function deleteRole(token: string, serverId: string, roleId: string) {
	return request<void>(`/servers/${serverId}/roles/${roleId}`, {
		method: "DELETE",
		headers: { authorization: `Bearer ${token}` }
	});
}

export function assignRole(token: string, serverId: string, userId: string, roleId: string) {
	return request<void>(`/servers/${serverId}/members/${userId}/roles/${roleId}`, {
		method: "PUT",
		headers: { authorization: `Bearer ${token}` }
	});
}

export function unassignRole(token: string, serverId: string, userId: string, roleId: string) {
	return request<void>(`/servers/${serverId}/members/${userId}/roles/${roleId}`, {
		method: "DELETE",
		headers: { authorization: `Bearer ${token}` }
	});
}

export function kickMember(token: string, serverId: string, userId: string) {
	return request<void>(`/servers/${serverId}/members/${userId}/kick`, {
		method: "POST",
		headers: { authorization: `Bearer ${token}` }
	});
}

export type ApiBan = {
	user_id: string;
	username: string;
	reason: string | null;
};

export function listBans(token: string, serverId: string) {
	return request<ApiBan[]>(`/servers/${serverId}/bans`, {
		headers: { authorization: `Bearer ${token}` }
	});
}

export function banMember(token: string, serverId: string, userId: string, reason?: string) {
	return request<void>(`/servers/${serverId}/bans/${userId}`, {
		method: "PUT",
		headers: { authorization: `Bearer ${token}` },
		body: JSON.stringify({ reason })
	});
}

export function unbanMember(token: string, serverId: string, userId: string) {
	return request<void>(`/servers/${serverId}/bans/${userId}`, {
		method: "DELETE",
		headers: { authorization: `Bearer ${token}` }
	});
}

export type ApiIceServer = {
	urls: string[];
	username: string | null;
	credential: string | null;
};

export function fetchIceServers(token: string) {
	return request<ApiIceServer[]>("/calls/ice-servers", {
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

export type ApiBadge = {
	slug: string;
	label: string;
	description: string;
};

export function badgeCatalog(token: string) {
	return request<ApiBadge[]>("/badges", {
		headers: { authorization: `Bearer ${token}` }
	});
}

export function userBadges(token: string, username: string) {
	return request<string[]>(`/badges/${encodeURIComponent(username)}`, {
		headers: { authorization: `Bearer ${token}` }
	});
}

export type PresenceState = "online" | "idle" | "dnd" | "invisible";

export type ApiProfile = {
	username: string;
	display_name: string | null;
	bio: string | null;
	pronouns: string | null;
	status_text: string | null;
	presence: PresenceState;
	activity_application: string | null;
	activity_details: string | null;
	activity_state: string | null;
	activity_image: string | null;
	activity_started_at: string | null;
	media_application: string | null;
	media_details: string | null;
	media_state: string | null;
	share_activity: boolean;
	accent_color: string | null;
	banner_color: string | null;
	avatar_url: string | null;
	banner_url: string | null;
	member_since: string;
};

export function fetchProfile(token: string, username: string) {
	return request<ApiProfile>(`/profile/${encodeURIComponent(username)}`, {
		headers: { authorization: `Bearer ${token}` }
	});
}

export type UpdateProfileBody = {
	display_name?: string;
	bio?: string;
	pronouns?: string;
	status_text?: string;
	status_clear_minutes?: number;
	accent_color?: string;
	banner_color?: string;
	share_activity?: boolean;
};

export function updateProfile(token: string, body: UpdateProfileBody) {
	return request<ApiProfile>("/profile", {
		method: "PATCH",
		headers: { authorization: `Bearer ${token}` },
		body: JSON.stringify(body)
	});
}

export function setAvatar(token: string, attachmentId: string) {
	return request<ApiProfile>("/profile/avatar", {
		method: "PUT",
		headers: { authorization: `Bearer ${token}` },
		body: JSON.stringify({ attachment_id: attachmentId })
	});
}

export function clearAvatar(token: string) {
	return request<ApiProfile>("/profile/avatar", {
		method: "DELETE",
		headers: { authorization: `Bearer ${token}` }
	});
}

export function setPresence(token: string, presence: PresenceState) {
	return request<ApiProfile>("/profile/presence", {
		method: "PUT",
		headers: { authorization: `Bearer ${token}` },
		body: JSON.stringify({ presence })
	});
}

export type SetActivityBody = {
	application?: string;
	details?: string;
	state?: string;
	image?: string;
	started_at?: string;
	kind?: "game" | "media";
};

export function setActivity(token: string, body: SetActivityBody) {
	return request<ApiProfile>("/profile/activity", {
		method: "PUT",
		headers: { authorization: `Bearer ${token}` },
		body: JSON.stringify(body)
	});
}

export function setBanner(token: string, attachmentId: string) {
	return request<ApiProfile>("/profile/banner", {
		method: "PUT",
		headers: { authorization: `Bearer ${token}` },
		body: JSON.stringify({ attachment_id: attachmentId })
	});
}

export function clearBanner(token: string) {
	return request<ApiProfile>("/profile/banner", {
		method: "DELETE",
		headers: { authorization: `Bearer ${token}` }
	});
}

export type ConnectionService =
	| "github"
	| "youtube"
	| "twitch"
	| "x"
	| "instagram"
	| "tiktok"
	| "reddit"
	| "steam"
	| "spotify"
	| "discord"
	| "facebook"
	| "telegram"
	| "vk"
	| "behance"
	| "dribbble"
	| "soundcloud"
	| "bandcamp"
	| "itchio"
	| "xbox"
	| "playstation"
	| "battlenet"
	| "epicgames"
	| "roblox";

export type ApiConnection = {
	id: string;
	service: ConnectionService;
	label: string;
	url: string;
};

export function listConnections(token: string, username: string) {
	return request<ApiConnection[]>(`/profile/${encodeURIComponent(username)}/connections`, {
		headers: { authorization: `Bearer ${token}` }
	});
}

export function addConnection(token: string, service: ConnectionService, url: string, label?: string) {
	return request<ApiConnection>("/profile/connections", {
		method: "POST",
		headers: { authorization: `Bearer ${token}` },
		body: JSON.stringify({ service, url, label })
	});
}

export function removeConnection(token: string, connectionId: string) {
	return request<void>(`/profile/connections/${encodeURIComponent(connectionId)}`, {
		method: "DELETE",
		headers: { authorization: `Bearer ${token}` }
	});
}

export type WidgetKind = "favorite_game" | "want_to_play" | "games_i_like" | "games_in_rotation";

export type ApiWidget = {
	id: string;
	kind: WidgetKind;
	title: string;
	image_url: string | null;
	description: string | null;
	tags: string[];
	pinned: boolean;
};

export function listWidgets(token: string, username: string) {
	return request<ApiWidget[]>(`/profile/${encodeURIComponent(username)}/widgets`, {
		headers: { authorization: `Bearer ${token}` }
	});
}

export function addWidget(
	token: string,
	kind: WidgetKind,
	title: string,
	options?: { imageAttachmentId?: string; externalImageUrl?: string }
) {
	return request<ApiWidget>("/profile/widgets", {
		method: "POST",
		headers: { authorization: `Bearer ${token}` },
		body: JSON.stringify({
			kind,
			title,
			image_attachment_id: options?.imageAttachmentId,
			external_image_url: options?.externalImageUrl
		})
	});
}

export function updateWidget(
	token: string,
	widgetId: string,
	patch: { description?: string; tags?: string[]; pinned?: boolean }
) {
	return request<ApiWidget>(`/profile/widgets/${encodeURIComponent(widgetId)}`, {
		method: "PATCH",
		headers: { authorization: `Bearer ${token}` },
		body: JSON.stringify(patch)
	});
}

export function removeWidget(token: string, widgetId: string) {
	return request<void>(`/profile/widgets/${encodeURIComponent(widgetId)}`, {
		method: "DELETE",
		headers: { authorization: `Bearer ${token}` }
	});
}

export type LinkPreview = {
	url: string;
	site_name: string | null;
	title: string | null;
	description: string | null;
	image: string | null;
};

export function fetchLinkPreview(token: string, url: string) {
	return request<LinkPreview>("/link-preview", {
		method: "POST",
		headers: { authorization: `Bearer ${token}` },
		body: JSON.stringify({ url })
	});
}

export type SubmitReportBody = {
	reported_user_id: string;
	context_kind: "dm" | "channel";
	context_id: string;
	server_id?: string;
	sealed_key: { ephemeral_public: string; nonce: string; ciphertext: string };
	payload_nonce: string;
	payload_ciphertext: string;
};

export function submitReport(token: string, body: SubmitReportBody) {
	return request<{ id: string }>("/reports", {
		method: "POST",
		headers: { authorization: `Bearer ${token}` },
		body: JSON.stringify(body)
	});
}

export function gameCover(token: string, name: string) {
	return request<{ url: string | null }>(`/games/cover?name=${encodeURIComponent(name)}`, {
		headers: { authorization: `Bearer ${token}` }
	});
}

<script lang="ts">
	import { fade, fly } from "svelte/transition";
	import Hash from "@lucide/svelte/icons/hash";
	import Pin from "@lucide/svelte/icons/pin";
	import Bell from "@lucide/svelte/icons/bell";
	import Users from "@lucide/svelte/icons/users";
	import Inbox from "@lucide/svelte/icons/inbox";
	import Search from "@lucide/svelte/icons/search";
	import Reply from "@lucide/svelte/icons/reply";
	import MoreHorizontal from "@lucide/svelte/icons/more-horizontal";
	import SendHorizontal from "@lucide/svelte/icons/send-horizontal";
	import Paperclip from "@lucide/svelte/icons/paperclip";
	import Smile from "@lucide/svelte/icons/smile";
	import X from "@lucide/svelte/icons/x";
	import FileIcon from "@lucide/svelte/icons/file";
	import UploadCloud from "@lucide/svelte/icons/upload-cloud";
	import Download from "@lucide/svelte/icons/download";
	import MessagesSquare from "@lucide/svelte/icons/messages-square";
	import Phone from "@lucide/svelte/icons/phone";
	import AtSign from "@lucide/svelte/icons/at-sign";
	import EyeOff from "@lucide/svelte/icons/eye-off";
	import Loader2 from "@lucide/svelte/icons/loader-2";
	import Pencil from "@lucide/svelte/icons/pencil";
	import Trash2 from "@lucide/svelte/icons/trash-2";
	import CropAttachmentModal from "$lib/components/CropAttachmentModal.svelte";
	import FullProfileModal from "$lib/components/FullProfileModal.svelte";
	import Lightbox from "$lib/components/Lightbox.svelte";
	import AudioPlayer from "$lib/components/AudioPlayer.svelte";
	import Maximize2 from "@lucide/svelte/icons/maximize-2";
	import PinnedPopover from "$lib/components/PinnedPopover.svelte";
	import InfoPopover from "$lib/components/InfoPopover.svelte";
	import MessageMenu from "$lib/components/MessageMenu.svelte";
	import ReportModal from "$lib/components/ReportModal.svelte";
	import EmojiPicker from "$lib/components/EmojiPicker.svelte";
	import ThreadPanel from "$lib/components/ThreadPanel.svelte";
	import { emojify } from "$lib/actions/emojify";
	import { toast } from "$lib/stores/toast.svelte";
	import { notifyDesktop } from "$lib/utils/notify";
	import { playNotificationSound } from "$lib/utils/sound";
	import { session } from "$lib/stores/session.svelte";
	import { profileStore } from "$lib/stores/profile.svelte";
	import { customEmojiStore } from "$lib/stores/customEmoji.svelte";
	import UserRound from "@lucide/svelte/icons/user-round";
	import {
		recordEmojiUse,
		frequentEmoji,
		listMessages,
		sendMessage,
		listDmMessages,
		sendDmMessage,
		uploadFile,
		fileUrl,
		resolveUrl,
		editMessage as apiEditMessage,
		deleteMessage as apiDeleteMessage,
		pinMessage as apiPinMessage,
		unpinMessage as apiUnpinMessage,
		listPinned as apiListPinned,
		addReaction as apiAddReaction,
		removeReaction as apiRemoveReaction,
		createThread as apiCreateThread,
		listMembers as apiListMembers,
		publishSenderKeys,
		listSenderKeys,
		fetchLinkPreview,
		PERMISSIONS,
		ApiError,
		type ApiMessage,
		type ApiReplyPreview,
		type MessageScope,
		type LinkPreview
	} from "$lib/api/client";
	import { colorForName } from "$lib/utils/color";
	import { textMentionsUser } from "$lib/utils/mentions";
	import { notificationSettings } from "$lib/stores/notifications.svelte";
	import { encryptForPeer, decryptFromPeer } from "$lib/crypto/dm";
	import {
		encryptForChannel,
		decryptFromChannel,
		absorbDistribution,
		needsRedistribution,
		markDistributedTo,
		getOrCreateSendState,
		packageDistribution,
		absorbSenderKeyFor
	} from "$lib/crypto/group";
	import { rememberDecrypted, recallDecrypted } from "$lib/crypto/sent-cache";
	import { encryptFile, genericUploadName } from "$lib/crypto/attachment";
	import { packPayload, unpackPayload } from "$lib/crypto/messagePayload";
	import { call } from "$lib/webrtc/call.svelte";
	import { loadAttachmentBlobUrl, loadEncryptedAttachmentBlobUrl, triggerDownload, AttachmentExpiredError } from "$lib/utils/attachment";
	import { renderMarkdown } from "$lib/utils/markdown";
	import type { Channel, Message, MessageAttachment } from "$lib/data/mock";

	const DEFAULT_QUICK_EMOJI = ["👍", "❤️", "😂", "🔥", "🎉"];
	const POLL_INTERVAL_MS = 3000;

	let { channel, isDm = false, serverId, peerId, onToggleMembers }: {
		channel: Channel;
		isDm?: boolean;
		serverId?: string;
		peerId?: string;
		onToggleMembers?: () => void;
	} = $props();

	const scope: MessageScope = isDm ? "dm" : "channel";

	let profileModalUsername = $state<string | null>(null);

	function openAuthorProfile(username: string) {
		if (username === session.username) return;
		profileModalUsername = username;
	}

	function fetchMessages(
		token: string,
		id: string,
		opts?: { before?: string; after?: string; limit?: number }
	) {
		return isDm ? listDmMessages(token, id, opts) : listMessages(token, id, opts);
	}

	function postMessage(
		token: string,
		id: string,
		content: string | null,
		attachmentId?: string,
		replyToId?: string
	) {
		return isDm
			? sendDmMessage(token, id, content, attachmentId, replyToId)
			: sendMessage(token, id, content, attachmentId, replyToId);
	}

	const decryptedContentCache = new Map<string, Promise<string>>();

	async function decryptOnce(authorUsername: string, messageId: string, blob: string): Promise<string> {
		const persisted = recallDecrypted(messageId);
		if (persisted !== null) return persisted;

		const myUsername = session.username;
		if (!myUsername) return blob;
		if (authorUsername === myUsername) {
			return "[sent from another device]";
		}

		try {
			const content = isDm
				? await decryptFromPeer(myUsername, channel.name, blob)
				: await decryptFromChannel(myUsername, channel.id, authorUsername, blob);
			rememberDecrypted(messageId, content);
			return content;
		} catch {
			const token = session.token;
			let absorbed = false;
			if (!isDm && token) {
				absorbed = await absorbSenderKeyFor(token, myUsername, channel.id, authorUsername);
			}
			if (absorbed) {
				try {
					const content = await decryptFromChannel(myUsername, channel.id, authorUsername, blob);
					rememberDecrypted(messageId, content);
					return content;
				} catch {
					return "[unable to decrypt message]";
				}
			}
			return "[unable to decrypt message]";
		}
	}

	function decryptStoredContent(authorUsername: string, messageId: string, blob: string): Promise<string> {
		const cached = decryptedContentCache.get(messageId);
		if (cached) return cached;

		const promise = decryptOnce(authorUsername, messageId, blob);
		decryptedContentCache.set(messageId, promise);
		return promise;
	}

	async function encryptOutgoing(myUsername: string, token: string, content: string): Promise<string> {
		return isDm
			? encryptForPeer(token, myUsername, channel.name, content)
			: encryptForChannel(myUsername, channel.id, content);
	}

	async function bootstrapChannelKeys(token: string, myUsername: string) {
		if (isDm || !serverId) return;

		const pending = await listSenderKeys(token, channel.id).catch(() => []);
		for (const entry of pending) {
			await absorbDistribution(myUsername, entry.sender_username, entry.ciphertext).catch(() => {});
		}

		try {
			const members = await apiListMembers(token, serverId);
			const others = members.filter((m) => m.username !== myUsername);
			const memberIds = others.map((m) => m.id);
			if (needsRedistribution(myUsername, channel.id, memberIds)) {
				const state = await getOrCreateSendState(myUsername, channel.id);
				const entries = [];
				for (const member of others) {
					const ciphertext = await packageDistribution(token, myUsername, channel.id, state, member.username);
					entries.push({ recipient_id: member.id, ciphertext });
				}
				if (entries.length > 0) await publishSenderKeys(token, channel.id, entries);
				await markDistributedTo(myUsername, channel.id, memberIds);
			}
		} catch {
			return;
		}
	}

	$effect(() => {
		const token = session.token;
		if (!token || !serverId) return;
		customEmojiStore.load(token, serverId);
	});

	const customEmojiMap = $derived(customEmojiStore.mapFor(serverId ?? null, session.token));

	function ensureProfileLoaded(username: string) {
		const token = session.token;
		if (token && !profileStore.forUser(username)) profileStore.load(token, username);
	}

	function displayNameFor(username: string): string {
		return profileStore.forUser(username)?.display_name || username;
	}

	function colorFor(username: string): string {
		return profileStore.forUser(username)?.accent_color || colorForName(username);
	}

	function formatMessageTime(message: Message): string {
		const date = new Date(message.timestampMs);
		const now = new Date();
		const isToday =
			date.getFullYear() === now.getFullYear() &&
			date.getMonth() === now.getMonth() &&
			date.getDate() === now.getDate();
		if (isToday) return message.time;
		return `${date.toLocaleDateString([], { month: "numeric", day: "numeric", year: "2-digit" })}, ${message.time}`;
	}

	async function toReplyPreview(reply: ApiReplyPreview | null) {
		if (!reply) return undefined;
		const text = reply.content
			? unpackPayload(await decryptStoredContent(reply.author, reply.id, reply.content)).text
			: "";
		const content = text || (reply.has_attachment ? "📎 Attachment" : "");
		return { id: reply.id, author: reply.author, content, hasAttachment: reply.has_attachment };
	}

	async function toMessage(apiMsg: ApiMessage): Promise<Message> {
		const decrypted = apiMsg.content
			? await decryptStoredContent(apiMsg.author, apiMsg.id, apiMsg.content)
			: "";
		const payload = unpackPayload(decrypted);
		ensureProfileLoaded(apiMsg.author);

		return {
			id: apiMsg.id,
			author: apiMsg.author,
			color: colorForName(apiMsg.author),
			content: payload.text,
			attachment: apiMsg.attachment
				? {
						id: apiMsg.attachment.id,
						filename: payload.attachment?.filename ?? apiMsg.attachment.filename,
						mimeType: payload.attachment?.mimeType ?? apiMsg.attachment.mime_type,
						sizeBytes: payload.attachment?.sizeBytes ?? apiMsg.attachment.size_bytes,
						key: payload.attachment?.key,
						nonce: payload.attachment?.nonce
					}
				: undefined,
			reactions: apiMsg.reactions.map((r) => ({ emoji: r.emoji, count: r.count, reacted: r.reacted })),
			pinned: apiMsg.pinned,
			replyTo: await toReplyPreview(apiMsg.reply_to),
			edited: !!apiMsg.edited_at,
			time: new Date(apiMsg.timestamp).toLocaleTimeString([], { hour: "2-digit", minute: "2-digit" }),
			timestampMs: new Date(apiMsg.timestamp).getTime(),
			mentionsMe: session.username
				? textMentionsUser(payload.text, session.username, !isDm)
				: false
		};
	}

	async function toMessages(rows: ApiMessage[]): Promise<Message[]> {
		const out: Message[] = [];
		for (const row of rows) out.push(await toMessage(row));
		return out;
	}

	let messages = $state<Message[]>([]);
	let lastId: string | null = null;
	let canManageMessages = $state(false);
	let memberIdByUsername = $state<Record<string, string>>({});
	let reportTarget = $state<Message | null>(null);

	$effect(() => {
		const token = session.token;
		const myUserId = session.userId;
		if (isDm || !serverId || !token || !myUserId) {
			canManageMessages = false;
			return;
		}
		let cancelled = false;
		apiListMembers(token, serverId)
			.then((members) => {
				if (cancelled) return;
				memberIdByUsername = Object.fromEntries(members.map((m) => [m.username, m.id]));
				const me = members.find((m) => m.id === myUserId);
				if (!me) {
					canManageMessages = false;
					return;
				}
				canManageMessages =
					me.is_owner || me.roles.some((r) => (r.permissions & PERMISSIONS.MANAGE_MESSAGES) !== 0);
			})
			.catch(() => {
				canManageMessages = false;
			});
		return () => {
			cancelled = true;
		};
	});

	function reportedUserIdFor(message: Message): string | undefined {
		return isDm ? peerId : memberIdByUsername[message.author];
	}

	$effect(() => {
		const token = session.token;
		const myUsername = session.username;
		const channelId = channel.id;
		const fetcher = fetchMessages;
		if (!token || !myUsername) return;

		messages = [];
		lastId = null;
		let cancelled = false;

		bootstrapChannelKeys(token, myUsername)
			.catch(() => {})
			.then(() => fetcher(token, channelId))
			.then(async (rows) => {
				if (cancelled) return;
				const converted = await toMessages(rows);
				if (cancelled) return;
				messages = converted;
				lastId = rows.at(-1)?.id ?? lastId;
			})
			.catch(() => {});

		let polling = false;
		const interval = setInterval(() => {
			if (!lastId || polling) return;
			polling = true;
			fetcher(token, channelId, { after: lastId })
				.then(async (rows) => {
					if (cancelled || rows.length === 0) return;
					const known = new Set(messages.map((m) => m.id));
					for (const row of rows) {
						if (cancelled) return;
						if (known.has(row.id)) continue;
						const built = await toMessage(row);
						messages.push(built);
						if (
							built.mentionsMe &&
							built.author !== myUsername &&
							notificationSettings.mentionsEnabled
						) {
							const summary = `${built.author} mentioned you in #${channel.name}`;
							toast.push(summary);
							notifyDesktop(isDm ? `${built.author}` : `#${channel.name}`, built.content || summary);
							playNotificationSound();
						}
					}
					lastId = rows.at(-1)!.id;
				})
				.catch(() => {})
				.finally(() => {
					polling = false;
				});
		}, POLL_INTERVAL_MS);

		return () => {
			cancelled = true;
			clearInterval(interval);
		};
	});

	let draft = $state("");
	let pendingFiles = $state<File[]>([]);
	let uploading = $state(false);
	let uploadingIndex = $state(0);
	let uploadingTotalCount = $state(0);
	let uploadingTotalBytes = $state(0);
	let fileInputEl = $state<HTMLInputElement | undefined>();
	let composerInputEl = $state<HTMLInputElement | undefined>();
	let replyingTo = $state<Message | null>(null);
	let mentionQuery = $state<string | null>(null);
	let mentionStart = $state(0);
	let mentionIndex = $state(0);

	type MentionCandidate = { name: string; kind: "everyone" | "here" | "user" };

	const mentionCandidates = $derived.by(() => {
		if (mentionQuery === null) return [];
		const q = mentionQuery.toLowerCase();
		const candidates: MentionCandidate[] = [];
		if (isDm) {
			if (channel.name.toLowerCase().startsWith(q)) candidates.push({ name: channel.name, kind: "user" });
		} else {
			if (q.length > 0 && "everyone".startsWith(q)) candidates.push({ name: "everyone", kind: "everyone" });
			if (q.length > 0 && "here".startsWith(q)) candidates.push({ name: "here", kind: "here" });
			for (const name of Object.keys(memberIdByUsername)) {
				if (name.toLowerCase().startsWith(q)) candidates.push({ name, kind: "user" });
			}
		}
		return candidates.slice(0, 8);
	});

	$effect(() => {
		const token = session.token;
		if (!token) return;
		for (const candidate of mentionCandidates) {
			if (candidate.kind === "user" && !profileStore.forUser(candidate.name)) {
				profileStore.load(token, candidate.name);
			}
		}
	});

	function onComposerInput() {
		const el = composerInputEl;
		if (!el) return;
		const pos = el.selectionStart ?? draft.length;
		const before = draft.slice(0, pos);
		const match = before.match(/(?:^|\s)@([a-zA-Z0-9_]{0,32})$/);
		if (match) {
			mentionQuery = match[1];
			mentionStart = pos - match[1].length - 1;
			mentionIndex = 0;
		} else {
			mentionQuery = null;
		}
	}

	function selectMention(name: string) {
		const query = mentionQuery ?? "";
		const before = draft.slice(0, mentionStart);
		const after = draft.slice(mentionStart + 1 + query.length);
		draft = `${before}@${name} ${after}`;
		mentionQuery = null;
		const caret = before.length + name.length + 2;
		requestAnimationFrame(() => {
			composerInputEl?.focus();
			composerInputEl?.setSelectionRange(caret, caret);
		});
	}

	function onComposerKeydown(event: KeyboardEvent) {
		if (mentionQuery === null || mentionCandidates.length === 0) return;
		if (event.key === "ArrowDown") {
			event.preventDefault();
			mentionIndex = (mentionIndex + 1) % mentionCandidates.length;
		} else if (event.key === "ArrowUp") {
			event.preventDefault();
			mentionIndex = (mentionIndex - 1 + mentionCandidates.length) % mentionCandidates.length;
		} else if (event.key === "Enter" || event.key === "Tab") {
			event.preventDefault();
			selectMention(mentionCandidates[mentionIndex].name);
		} else if (event.key === "Escape") {
			mentionQuery = null;
		}
	}
	let openMenuId = $state<string | null>(null);
	let composerEmojiOpen = $state(false);
	let pinnedOpen = $state(false);
	let notificationsOpen = $state(false);
	let inboxOpen = $state(false);
	let muted = $state(false);
	let emojiCounts = $state<Record<string, number>>({});

	const quickEmoji = $derived.by(() => {
		const used = Object.entries(emojiCounts)
			.sort((a, b) => b[1] - a[1])
			.map(([emoji]) => emoji);
		for (const emoji of DEFAULT_QUICK_EMOJI) {
			if (used.length >= 5) break;
			if (!used.includes(emoji)) used.push(emoji);
		}
		return used.slice(0, 5);
	});

	$effect(() => {
		const token = session.token;
		if (!token) return;
		frequentEmoji(token, 5)
			.then((rows) => {
				const counts: Record<string, number> = {};
				for (const row of rows) counts[row.emoji] = row.count;
				emojiCounts = counts;
			})
			.catch(() => {});
	});

	let imageUrls = $state<Record<string, string>>({});
	let expiredAttachments = $state<Set<string>>(new Set());
	let revealedSpoilers = $state<Set<string>>(new Set());

	function isSpoilerFilename(name: string): boolean {
		return name.startsWith("SPOILER_");
	}

	function displayFilename(name: string): string {
		return isSpoilerFilename(name) ? name.slice("SPOILER_".length) : name;
	}

	function revealSpoiler(messageId: string) {
		revealedSpoilers = new Set(revealedSpoilers).add(messageId);
	}

	$effect(() => {
		const token = session.token;
		if (!token) return;
		for (const message of messages) {
			const attachment = message.attachment;
			const isMedia =
				attachment &&
				(attachment.mimeType.startsWith("image/") ||
					attachment.mimeType.startsWith("video/") ||
					attachment.mimeType.startsWith("audio/"));
			if (attachment && isMedia && !imageUrls[attachment.id]) {
				const loader =
					attachment.key && attachment.nonce
						? loadEncryptedAttachmentBlobUrl(token, attachment.id, attachment.key, attachment.nonce, attachment.mimeType)
						: loadAttachmentBlobUrl(token, attachment.id, attachment.filename);
				loader
					.then((url) => {
						imageUrls[attachment.id] = url;
					})
					.catch((err) => {
						if (err instanceof AttachmentExpiredError) {
							expiredAttachments = new Set(expiredAttachments).add(attachment.id);
						}
					});
			}
		}
	});

	const URL_RE = /https?:\/\/[^\s<>"']+/i;

	function firstUrl(content: string): string | null {
		const match = content.match(URL_RE);
		if (!match) return null;
		return match[0].replace(/[.,;:!?)\]]+$/, "");
	}

	let linkPreviews = $state<Record<string, LinkPreview | null>>({});
	let dismissedPreviews = $state<Set<string>>(new Set());
	const attemptedPreviewUrls = new Set<string>();

	$effect(() => {
		const token = session.token;
		if (!token) return;
		for (const message of messages) {
			if (!message.content) continue;
			const url = firstUrl(message.content);
			if (!url || attemptedPreviewUrls.has(url)) continue;
			attemptedPreviewUrls.add(url);
			fetchLinkPreview(token, url)
				.then((preview) => {
					linkPreviews[url] = preview;
				})
				.catch(() => {
					linkPreviews[url] = null;
				});
		}
	});

	async function downloadAttachment(attachment: MessageAttachment) {
		const token = session.token;
		if (!token) return;
		try {
			const url =
				attachment.key && attachment.nonce
					? await loadEncryptedAttachmentBlobUrl(token, attachment.id, attachment.key, attachment.nonce, attachment.mimeType)
					: await loadAttachmentBlobUrl(token, attachment.id, attachment.filename);
			triggerDownload(url, attachment.filename);
		} catch {
			toast.push("Couldn't download file");
		}
	}

	function pickFile() {
		fileInputEl?.click();
	}

	function onFileChosen(event: Event) {
		const input = event.currentTarget as HTMLInputElement;
		pendingFiles = [...pendingFiles, ...Array.from(input.files ?? [])];
		input.value = "";
	}

	function clearPendingFile(index: number) {
		pendingFiles = pendingFiles.filter((_, i) => i !== index);
	}

	let cropModalOpen = $state(false);
	let cropIndex = $state<number | null>(null);
	let lightbox = $state<{ src: string; kind: "image" | "video"; alt?: string } | null>(null);

	function openCrop(index: number) {
		cropIndex = index;
		cropModalOpen = true;
	}

	function onCropped(file: File) {
		if (cropIndex !== null) {
			pendingFiles = pendingFiles.map((f, i) => (i === cropIndex ? file : f));
		}
		cropModalOpen = false;
		cropIndex = null;
	}

	const SPOILER_PREFIX = "SPOILER_";

	function isPendingSpoiler(file: File): boolean {
		return file.name.startsWith(SPOILER_PREFIX);
	}

	function toggleSpoiler(index: number) {
		pendingFiles = pendingFiles.map((f, i) => {
			if (i !== index) return f;
			return isPendingSpoiler(f)
				? new File([f], f.name.slice(SPOILER_PREFIX.length), { type: f.type })
				: new File([f], `${SPOILER_PREFIX}${f.name}`, { type: f.type });
		});
	}

	let pendingPreviewUrls = $state<(string | null)[]>([]);
	$effect(() => {
		const urls = pendingFiles.map((f) => (f.type.startsWith("image/") ? URL.createObjectURL(f) : null));
		pendingPreviewUrls = urls;
		return () => {
			for (const url of urls) if (url) URL.revokeObjectURL(url);
		};
	});

	function normalizeImageFile(file: File, mimeType: string): File {
		const ext = mimeType.split("/")[1]?.split("+")[0] || "png";
		if (file.type === mimeType) return file;
		return new File([file], `image.${ext}`, { type: mimeType });
	}

	async function onComposerPaste(event: ClipboardEvent) {
		const items = event.clipboardData?.items;
		if (items) {
			const pasted: File[] = [];
			for (const item of items) {
				if (item.kind !== "file") continue;
				const file = item.getAsFile();
				if (!file) continue;
				pasted.push(item.type.startsWith("image/") ? normalizeImageFile(file, item.type) : file);
			}
			if (pasted.length > 0) {
				pendingFiles = [...pendingFiles, ...pasted];
				event.preventDefault();
				return;
			}
		}

		// WebKitGTK on Linux doesn't reliably expose image clipboard entries
		// via ClipboardEvent.clipboardData, so fall back to Tauri's native
		// clipboard reader for image data copied from other apps.
		try {
			const { readImage } = await import("@tauri-apps/plugin-clipboard-manager");
			const image = await readImage();
			const [rgba, size] = await Promise.all([image.rgba(), image.size()]);
			const canvas = document.createElement("canvas");
			canvas.width = size.width;
			canvas.height = size.height;
			const ctx = canvas.getContext("2d");
			if (!ctx) return;
			ctx.putImageData(new ImageData(new Uint8ClampedArray(rgba), size.width, size.height), 0, 0);
			canvas.toBlob((blob) => {
				if (blob) pendingFiles = [...pendingFiles, new File([blob], "image.png", { type: "image/png" })];
			}, "image/png");
		} catch {
			// Clipboard has no image content — nothing to paste.
		}
	}

	let dragActive = $state(false);
	let dragDepth = 0;

	function onChatDragOver(event: DragEvent) {
		if (!event.dataTransfer?.types.includes("Files")) return;
		event.preventDefault();
	}

	function onChatDragEnter(event: DragEvent) {
		if (!event.dataTransfer?.types.includes("Files")) return;
		event.preventDefault();
		dragDepth++;
		dragActive = true;
	}

	function onChatDragLeave(event: DragEvent) {
		if (!event.dataTransfer?.types.includes("Files")) return;
		dragDepth = Math.max(0, dragDepth - 1);
		if (dragDepth === 0) dragActive = false;
	}

	function onChatDrop(event: DragEvent) {
		event.preventDefault();
		dragDepth = 0;
		dragActive = false;
		const files = Array.from(event.dataTransfer?.files ?? []);
		if (files.length === 0) return;
		pendingFiles = [...pendingFiles, ...files];
		if (event.shiftKey) submitPending();
	}

	function formatSize(bytes: number): string {
		if (bytes < 1024) return `${bytes} B`;
		if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`;
		if (bytes < 1024 * 1024 * 1024) return `${(bytes / (1024 * 1024)).toFixed(1)} MB`;
		return `${(bytes / (1024 * 1024 * 1024)).toFixed(2)} GB`;
	}

	async function send(event: SubmitEvent) {
		event.preventDefault();
		await submitPending();
	}

	async function submitPending() {
		mentionQuery = null;
		const content = draft.trim();
		const token = session.token;
		const myUsername = session.username;
		const files = pendingFiles;
		const replyToId = replyingTo?.id;
		if ((!content && files.length === 0) || !token) return;

		draft = "";
		pendingFiles = [];
		replyingTo = null;

		// More than one file goes out as one message per file (the schema
		// is one attachment per message) - only the first carries the typed
		// text and the reply reference, the rest are attachment-only.
		const steps = files.length > 0 ? files.length : 1;
		try {
			if (files.length > 0) {
				uploading = true;
				uploadingIndex = 0;
				uploadingTotalCount = files.length;
				uploadingTotalBytes = files.reduce((sum, f) => sum + f.size, 0);
			}
			for (let i = 0; i < steps; i++) {
				const file = files[i];
				const textForThisMessage = i === 0 ? content : "";

				let attachmentId: string | undefined;
				let attachmentMeta: Awaited<ReturnType<typeof encryptFile>>["meta"] | undefined;
				if (file) {
					uploadingIndex = i + 1;
					const { blob, meta } = await encryptFile(file);
					const uploaded = await uploadFile(token, new File([blob], genericUploadName(), { type: blob.type }));
					attachmentId = uploaded.id;
					attachmentMeta = meta;
				}

				let payload: string | null = null;
				if (myUsername && (textForThisMessage || attachmentMeta)) {
					const packed = packPayload(textForThisMessage, attachmentMeta);
					payload = await encryptOutgoing(myUsername, token, packed);
				}

				const apiMsg = await postMessage(token, channel.id, payload, attachmentId, i === 0 ? replyToId : undefined);
				if (textForThisMessage || attachmentMeta) {
					rememberDecrypted(apiMsg.id, packPayload(textForThisMessage, attachmentMeta));
				}
				messages.push(await toMessage(apiMsg));
				lastId = apiMsg.id;
			}
		} catch (err) {
			if (err instanceof ApiError && err.status === 413) {
				toast.push("File is too large for your plan (50MB free / 2GB premium)");
			} else if (err instanceof ApiError && err.status === 429) {
				toast.push(err.message || "You're sending messages too fast — slow down");
			} else {
				toast.push("Message failed to send");
			}
		} finally {
			uploading = false;
			uploadingIndex = 0;
			uploadingTotalCount = 0;
			uploadingTotalBytes = 0;
		}
	}

	const GROUP_WINDOW_MS = 10 * 60 * 1000;

	function isGrouped(index: number) {
		if (index === 0) return false;
		const prev = messages[index - 1];
		const current = messages[index];
		return (
			prev.author === current.author &&
			!current.replyTo &&
			current.timestampMs - prev.timestampMs <= GROUP_WINDOW_MS
		);
	}

	function copyText(message: Message) {
		navigator.clipboard.writeText(message.content);
		toast.push("Copied");
	}

	function isMine(message: Message) {
		return message.author === session.username;
	}

	async function togglePin(message: Message) {
		const token = session.token;
		if (!token) return;
		const next = !message.pinned;
		message.pinned = next;
		try {
			if (next) await apiPinMessage(token, scope, channel.id, message.id);
			else await apiUnpinMessage(token, scope, channel.id, message.id);
			toast.push(next ? "Message pinned" : "Message unpinned");
		} catch {
			message.pinned = !next;
			toast.push("Couldn't update pin");
		}
	}

	async function deleteMessage(message: Message) {
		const token = session.token;
		if (!token) return;
		const backup = messages;
		messages = messages.filter((m) => m.id !== message.id);
		try {
			await apiDeleteMessage(token, scope, channel.id, message.id);
			toast.push("Message deleted");
		} catch {
			messages = backup;
			toast.push("Couldn't delete message");
		}
	}

	async function toggleReaction(message: Message, emoji: string) {
		const token = session.token;
		if (!token) return;

		if (!message.reactions) message.reactions = [];
		const existing = message.reactions.find((r) => r.emoji === emoji);
		const wasReacted = existing?.reacted ?? false;

		if (existing) {
			existing.reacted = !existing.reacted;
			existing.count += existing.reacted ? 1 : -1;
			if (existing.count <= 0) {
				message.reactions = message.reactions.filter((r) => r.emoji !== emoji);
			}
		} else {
			message.reactions.push({ emoji, count: 1, reacted: true });
		}

		try {
			if (wasReacted) await apiRemoveReaction(token, scope, channel.id, message.id, emoji);
			else await apiAddReaction(token, scope, channel.id, message.id, emoji);
		} catch {
			toast.push("Couldn't update reaction");
		}
	}

	function insertEmoji(emoji: string) {
		draft += emoji;
		trackEmojiUse(emoji);
	}

	function trackEmojiUse(emoji: string) {
		emojiCounts[emoji] = (emojiCounts[emoji] ?? 0) + 1;
		const token = session.token;
		if (token) recordEmojiUse(token, emoji).catch(() => {});
	}

	function react(message: Message, emoji: string) {
		toggleReaction(message, emoji);
		trackEmojiUse(emoji);
	}

	let editingId = $state<string | null>(null);
	let editDraft = $state("");

	function startEdit(message: Message) {
		editingId = message.id;
		editDraft = message.content;
	}

	function cancelEdit() {
		editingId = null;
		editDraft = "";
	}

	async function saveEdit(message: Message) {
		const token = session.token;
		const content = editDraft.trim();
		if (!token || !content) return;

		try {
			const attachmentMeta =
				message.attachment?.key && message.attachment.nonce
					? {
							key: message.attachment.key,
							nonce: message.attachment.nonce,
							filename: message.attachment.filename,
							mimeType: message.attachment.mimeType,
							sizeBytes: message.attachment.sizeBytes
						}
					: undefined;
			const packed = packPayload(content, attachmentMeta);
			let payload = packed;
			if (session.username) {
				payload = await encryptOutgoing(session.username, token, packed);
			}
			await apiEditMessage(token, scope, channel.id, message.id, payload);
			rememberDecrypted(message.id, packed);
			message.content = content;
			message.edited = true;
			editingId = null;
			editDraft = "";
		} catch {
			toast.push("Couldn't edit message");
		}
	}

	async function startDmCall() {
		const token = session.token;
		if (!token) return;
		try {
			await call.join(token, channel.id, channel.name);
		} catch {
			toast.push("Couldn't start the call — check microphone permissions");
		}
	}

	let threadsOpen = $state(false);
	let openThreadId = $state<string | undefined>(undefined);

	function openThreads() {
		openThreadId = undefined;
		threadsOpen = true;
	}

	async function createThread(message: Message) {
		const token = session.token;
		if (!token) return;
		try {
			const thread = await apiCreateThread(token, channel.id, message.id, `Thread: ${message.content.slice(0, 40)}`);
			openThreadId = thread.id;
			threadsOpen = true;
		} catch {
			toast.push("Couldn't create thread");
		}
	}

	let pinnedMessages = $state<Message[]>([]);

	async function openPinned() {
		pinnedOpen = !pinnedOpen;
		if (!pinnedOpen) return;
		const token = session.token;
		if (!token) return;
		try {
			const rows = await apiListPinned(token, scope, channel.id);
			const out: Message[] = [];
			for (const row of rows) out.push(await toMessage(row));
			pinnedMessages = out;
		} catch {
			pinnedMessages = [];
		}
	}
</script>

<div class="chat-row">
<section
	class="chat"
	aria-label="Message area"
	ondragenter={onChatDragEnter}
	ondragover={onChatDragOver}
	ondragleave={onChatDragLeave}
	ondrop={onChatDrop}
>
	{#if dragActive}
		<div class="drop-overlay">
			<div class="drop-card">
				<UploadCloud size={30} strokeWidth={1.75} class="drop-card-bg-icon" />
				<h3>Upload to {isDm ? channel.name : `#${channel.name}`}</h3>
				<p>You can add comments before uploading. Hold shift to upload directly.</p>
				<span class="drop-card-badge"><FileIcon size={18} strokeWidth={2} /></span>
			</div>
		</div>
	{/if}
	<header class="header">
		{#if isDm}
			<UserRound size={18} strokeWidth={2.5} class="hash" />
		{:else}
			<Hash size={18} strokeWidth={2.5} class="hash" />
		{/if}
		{#key channel.id}
			<span class="name" in:fade={{ duration: 150 }}>{channel.name}</span>
		{/key}
		<div class="spacer"></div>
		<div class="header-icons">
			{#if isDm}
				<button class="icon-button" title="Voice call" onclick={startDmCall}>
					<Phone size={17} strokeWidth={2} />
				</button>
			{/if}
			<div class="anchor">
				<button class="icon-button" title="Pinned messages" onclick={openPinned}>
					<Pin size={17} strokeWidth={2} />
				</button>
				{#if pinnedOpen}
					<PinnedPopover pinned={pinnedMessages} onClose={() => (pinnedOpen = false)} />
				{/if}
			</div>
			<div class="anchor">
				<button
					class="icon-button"
					class:active={muted}
					title="Notification settings"
					onclick={() => (notificationsOpen = !notificationsOpen)}
				>
					<Bell size={17} strokeWidth={2} />
				</button>
				{#if notificationsOpen}
					<InfoPopover title="Notifications" onClose={() => (notificationsOpen = false)}>
						<div class="toggle-row">
							<span>Mute channel</span>
							<label class="switch">
								<input type="checkbox" bind:checked={muted} />
								<span class="track"><span class="thumb"></span></span>
							</label>
						</div>
					</InfoPopover>
				{/if}
			</div>
			{#if !isDm}
				<button class="icon-button" class:active={threadsOpen} title="Threads" onclick={openThreads}>
					<MessagesSquare size={17} strokeWidth={2} />
				</button>
				<button class="icon-button" title="Members" onclick={onToggleMembers}>
					<Users size={17} strokeWidth={2} />
				</button>
			{:else}
				<button class="icon-button" title="Profile" onclick={onToggleMembers}>
					<Users size={17} strokeWidth={2} />
				</button>
			{/if}
			<div class="header-search">
				<Search size={13} strokeWidth={2.5} />
				<input type="text" placeholder="Search" />
			</div>
			<div class="anchor">
				<button class="icon-button" title="Inbox" onclick={() => (inboxOpen = !inboxOpen)}>
					<Inbox size={17} strokeWidth={2} />
				</button>
				{#if inboxOpen}
					<InfoPopover title="Inbox" onClose={() => (inboxOpen = false)}>
						<p class="inbox-empty">You're all caught up.</p>
					</InfoPopover>
				{/if}
			</div>
		</div>
	</header>

	{#key channel.id}
	<div class="messages" in:fade={{ duration: 160 }}>
		{#if messages.length === 0}
			<div class="welcome">
				<div class="welcome-icon">
					{#if isDm}
						<UserRound size={28} strokeWidth={2} />
					{:else}
						<Hash size={28} strokeWidth={2} />
					{/if}
				</div>
				{#if isDm}
					<h2>{channel.name}</h2>
					<p>This is the start of your conversation with {channel.name}.</p>
				{:else}
					<h2>Welcome to #{channel.name}</h2>
					<p>This is the start of the channel.</p>
				{/if}
			</div>
		{/if}
			{#each messages as message, index (message.id)}
				<div class="message" class:grouped={isGrouped(index)} class:mentioned={message.mentionsMe} in:fly={{ y: 6, duration: 180, delay: index * 20 }}>
					{#if !isGrouped(index)}
						{@const authorAvatarUrl = profileStore.forUser(message.author)?.avatar_url}
						<div
							class="avatar clickable"
							style:background={authorAvatarUrl ? undefined : colorFor(message.author)}
							style:background-image={authorAvatarUrl ? `url(${resolveUrl(authorAvatarUrl, session.token)})` : undefined}
							onclick={() => openAuthorProfile(message.author)}
							role="button"
							tabindex="0"
							onkeydown={(e) => e.key === "Enter" && openAuthorProfile(message.author)}
						>
							{#if !authorAvatarUrl}{message.author.slice(0, 2).toUpperCase()}{/if}
						</div>
					{:else}
						<div class="avatar-spacer">
							<span class="hover-time">{message.time}</span>
						</div>
					{/if}

					<div class="body">
						{#if message.replyTo}
							<p class="reply-quote">
								<Reply size={12} strokeWidth={2} />
								<span class="reply-author">{displayNameFor(message.replyTo.author)}</span>
								<span class="reply-snippet">{message.replyTo.content}</span>
							</p>
						{/if}
						{#if !isGrouped(index)}
							<p class="meta">
								<span
									class="author clickable"
									style:color={colorFor(message.author)}
									onclick={() => openAuthorProfile(message.author)}
									role="button"
									tabindex="0"
									onkeydown={(e) => e.key === "Enter" && openAuthorProfile(message.author)}
								>{displayNameFor(message.author)}</span>
								<span class="time">{formatMessageTime(message)}</span>
								{#if message.edited}<span class="edited-flag">(edited)</span>{/if}
								{#if message.pinned}<Pin size={11} strokeWidth={2.5} class="pinned-flag" />{/if}
							</p>
						{/if}
						{#if editingId === message.id}
							<form class="edit-form" onsubmit={(e) => (e.preventDefault(), saveEdit(message))}>
								<input type="text" bind:value={editDraft} />
								<div class="edit-actions">
									<button type="button" class="ghost-small" onclick={cancelEdit}>Cancel</button>
									<button type="submit" class="primary-small" disabled={!editDraft.trim()}>Save</button>
								</div>
							</form>
						{:else if message.content}
							<p class="content" use:emojify>
								{@html renderMarkdown(message.content, session.username ?? undefined, customEmojiMap)}
								{#if message.edited && isGrouped(index)}<span class="edited-flag">(edited)</span>{/if}
							</p>
							{@const previewUrl = firstUrl(message.content)}
							{#if previewUrl && linkPreviews[previewUrl] && !dismissedPreviews.has(message.id)}
								{@const preview = linkPreviews[previewUrl]}
								<div class="link-preview">
									<button class="link-preview-close" onclick={() => dismissedPreviews.add(message.id)} title="Dismiss preview">
										<X size={12} strokeWidth={2} />
									</button>
									<div class="link-preview-body">
										{#if preview.site_name}<span class="link-preview-site">{preview.site_name}</span>{/if}
										{#if preview.title}
											<a class="link-preview-title" href={preview.url} target="_blank" rel="noreferrer">{preview.title}</a>
										{/if}
										{#if preview.description}<p class="link-preview-desc">{preview.description}</p>{/if}
									</div>
									{#if preview.image}
										<img class="link-preview-image" src={preview.image} alt="" />
									{/if}
								</div>
							{/if}
						{/if}
						{#if message.attachment}
							{@const isSpoiler = isSpoilerFilename(message.attachment.filename)}
							{@const revealed = !isSpoiler || revealedSpoilers.has(message.id)}
							{@const isVideo = message.attachment.mimeType.startsWith("video/")}
							{@const isAudio = message.attachment.mimeType.startsWith("audio/")}
							{#if expiredAttachments.has(message.attachment.id)}
								<div class="attachment-file expired">
									<FileIcon size={20} strokeWidth={2} />
									<span class="attachment-info">
										<span class="attachment-name">{displayFilename(message.attachment.filename)}</span>
										<span class="attachment-size">Expired — no longer stored on the server</span>
									</span>
								</div>
							{:else if message.attachment.mimeType.startsWith("image/") && imageUrls[message.attachment.id]}
								{#if revealed}
									<button
										class="attachment-image"
										onclick={() =>
											(lightbox = {
												src: imageUrls[message.attachment!.id],
												kind: "image",
												alt: displayFilename(message.attachment!.filename)
											})}
									>
										<img src={imageUrls[message.attachment.id]} alt={displayFilename(message.attachment.filename)} />
									</button>
								{:else}
									<button
										class="attachment-image spoiler-hidden"
										onclick={() => revealSpoiler(message.id)}
									>
										<img src={imageUrls[message.attachment.id]} alt="" />
										<span class="spoiler-overlay">
											<EyeOff size={20} strokeWidth={2} />
											Spoiler — click to reveal
										</span>
									</button>
								{/if}
							{:else if isVideo && imageUrls[message.attachment.id]}
								{#if revealed}
									<div class="attachment-video">
										<video src={imageUrls[message.attachment.id]} controls></video>
										<button
											class="video-expand"
											title="Open larger"
											onclick={() =>
												(lightbox = {
													src: imageUrls[message.attachment!.id],
													kind: "video",
													alt: displayFilename(message.attachment!.filename)
												})}
										>
											<Maximize2 size={14} strokeWidth={2} />
										</button>
									</div>
								{:else}
									<button
										class="attachment-image spoiler-hidden"
										onclick={() => revealSpoiler(message.id)}
									>
										<video src={imageUrls[message.attachment.id]} muted></video>
										<span class="spoiler-overlay">
											<EyeOff size={20} strokeWidth={2} />
											Spoiler — click to reveal
										</span>
									</button>
								{/if}
							{:else if isAudio && revealed && imageUrls[message.attachment.id]}
								<AudioPlayer
									src={imageUrls[message.attachment.id]}
									filename={displayFilename(message.attachment.filename)}
									sizeBytes={message.attachment.sizeBytes}
								/>
							{:else}
								<button
									class="attachment-file"
									onclick={() => (revealed ? downloadAttachment(message.attachment!) : revealSpoiler(message.id))}
								>
									{#if revealed}
										<FileIcon size={20} strokeWidth={2} />
									{:else}
										<EyeOff size={20} strokeWidth={2} />
									{/if}
									<span class="attachment-info">
										<span class="attachment-name">
											{revealed ? displayFilename(message.attachment.filename) : "Spoiler — click to reveal"}
										</span>
										{#if revealed}<span class="attachment-size">{formatSize(message.attachment.sizeBytes)}</span>{/if}
									</span>
									{#if revealed}<Download size={16} strokeWidth={2} />{/if}
								</button>
							{/if}
						{/if}
						{#if message.reactions && message.reactions.length > 0}
							<div class="reactions">
								{#each message.reactions as reaction (reaction.emoji)}
									<button
										class="reaction"
										class:reacted={reaction.reacted}
										use:emojify
										onclick={() => toggleReaction(message, reaction.emoji)}
									>
										{reaction.emoji} {reaction.count}
									</button>
								{/each}
							</div>
						{/if}
					</div>

					<div class="hover-actions">
						{#each quickEmoji as emoji (emoji)}
							<button class="icon-button small quick-react" use:emojify title={`React with ${emoji}`} onclick={() => react(message, emoji)}>
								{emoji}
							</button>
						{/each}
						<button class="icon-button small" title="Reply" onclick={() => (replyingTo = message)}>
							<Reply size={15} strokeWidth={2} />
						</button>
						<div class="anchor">
							<button
								class="icon-button small"
								title="More"
								onclick={() => (openMenuId = openMenuId === message.id ? null : message.id)}
							>
								<MoreHorizontal size={15} strokeWidth={2} />
							</button>
							{#if openMenuId === message.id}
								<MessageMenu
									pinned={!!message.pinned}
									canEdit={isMine(message)}
									canDelete={isMine(message) || canManageMessages}
									canReport={!isMine(message)}
									onClose={() => (openMenuId = null)}
									onCopy={() => copyText(message)}
									onTogglePin={() => togglePin(message)}
									onEdit={() => startEdit(message)}
									onDelete={() => deleteMessage(message)}
									onCreateThread={isDm ? undefined : () => createThread(message)}
									onReport={() => (reportTarget = message)}
								/>
							{/if}
						</div>
					</div>
				</div>
			{/each}
	</div>
	{/key}

	{#if replyingTo}
		<div class="reply-banner" transition:fly={{ y: 8, duration: 140 }}>
			<Reply size={14} strokeWidth={2} />
			<span>Replying to <strong>{displayNameFor(replyingTo.author)}</strong></span>
			<button class="cancel-reply" onclick={() => (replyingTo = null)}>
				<X size={14} strokeWidth={2} />
			</button>
		</div>
	{/if}

	{#if uploading && uploadingTotalCount > 0}
		<div class="upload-banner" transition:fly={{ y: 8, duration: 140 }}>
			<FileIcon size={18} strokeWidth={2} />
			<div class="upload-banner-body">
				<span class="upload-banner-title">
					Uploading {uploadingTotalCount === 1 ? "1 File" : `${uploadingTotalCount} Files`}
					{" \u2014 "}{formatSize(uploadingTotalBytes)}
				</span>
				<div class="upload-banner-track">
					<div
						class="upload-banner-fill"
						style:width={`${Math.min(100, (uploadingIndex / uploadingTotalCount) * 100)}%`}
					></div>
				</div>
			</div>
		</div>
	{/if}

	{#if pendingFiles.length > 0}
		<div class="attachment-preview-row" transition:fly={{ y: 8, duration: 140 }}>
			{#each pendingFiles as file, i (i)}
				{@const isSpoiler = isPendingSpoiler(file)}
				{@const previewUrl = pendingPreviewUrls[i]}
				<div class="attachment-card">
					<div class="attachment-thumb" class:spoiler-blur={isSpoiler}>
						{#if previewUrl}
							<img src={previewUrl} alt={file.name} />
						{:else}
							<FileIcon size={22} strokeWidth={1.5} />
						{/if}
						{#if isSpoiler}
							<span class="spoiler-tag">Spoiler</span>
						{/if}
						{#if uploading}
							<div class="attachment-uploading">
								<Loader2 size={22} strokeWidth={2} class="spin" />
							</div>
						{:else}
							<div class="attachment-hover-actions">
								{#if previewUrl}
									<button type="button" title="Edit" onclick={() => openCrop(i)}>
										<Pencil size={16} strokeWidth={2} />
									</button>
									<button
										type="button"
										class:active={isSpoiler}
										title={isSpoiler ? "Unmark spoiler" : "Mark as spoiler"}
										onclick={() => toggleSpoiler(i)}
									>
										<EyeOff size={16} strokeWidth={2} />
									</button>
								{/if}
								<button type="button" title="Remove" onclick={() => clearPendingFile(i)}>
									<Trash2 size={16} strokeWidth={2} />
								</button>
							</div>
						{/if}
					</div>
					<span class="attachment-preview-name">
						{isSpoiler ? file.name.slice(SPOILER_PREFIX.length) : file.name}
					</span>
				</div>
			{/each}
		</div>
	{/if}

	{#if cropModalOpen && cropIndex !== null && pendingFiles[cropIndex] && pendingPreviewUrls[cropIndex]}
		{@const cropSrc = pendingPreviewUrls[cropIndex]}
		<CropAttachmentModal
			src={cropSrc ?? ""}
			filename={pendingFiles[cropIndex].name}
			onCancel={() => {
				cropModalOpen = false;
				cropIndex = null;
			}}
			onConfirm={onCropped}
		/>
	{/if}

	{#if lightbox}
		<Lightbox
			src={lightbox.src}
			kind={lightbox.kind}
			alt={lightbox.alt}
			onClose={() => (lightbox = null)}
			onDownload={() => triggerDownload(lightbox!.src, lightbox!.alt ?? "file")}
		/>
	{/if}

	<form class="composer" onsubmit={send}>
		<input
			type="file"
			multiple
			bind:this={fileInputEl}
			onchange={onFileChosen}
			style="display: none;"
		/>
		<button type="button" class="attach" title="Upload a file" onclick={pickFile}>
			<Paperclip size={18} strokeWidth={2} />
		</button>
		<div class="anchor composer-input-wrap">
			{#if mentionQuery !== null && mentionCandidates.length > 0}
				<div class="mention-popup" transition:fly={{ y: 6, duration: 120 }}>
					{#each mentionCandidates as candidate, i (candidate.name)}
						{@const candidateAvatarUrl = candidate.kind === "user" ? profileStore.forUser(candidate.name)?.avatar_url : null}
						<button
							type="button"
							class="mention-item"
							class:active={i === mentionIndex}
							onmousedown={(e) => {
								e.preventDefault();
								selectMention(candidate.name);
							}}
						>
							{#if candidate.kind === "user"}
								<span
									class="mention-avatar"
									style:background={candidateAvatarUrl ? undefined : colorFor(candidate.name)}
									style:background-image={candidateAvatarUrl ? `url(${resolveUrl(candidateAvatarUrl, session.token)})` : undefined}
								>
									{#if !candidateAvatarUrl}{candidate.name.slice(0, 2).toUpperCase()}{/if}
								</span>
							{:else}
								<span class="mention-avatar mention-avatar-broadcast">
									<AtSign size={14} strokeWidth={2.25} />
								</span>
							{/if}
							@{candidate.name}
						</button>
					{/each}
				</div>
			{/if}
			<input
				type="text"
				bind:this={composerInputEl}
				placeholder={isDm ? `Message ${channel.name}` : `Message #${channel.name}`}
				bind:value={draft}
				onpaste={onComposerPaste}
				oninput={onComposerInput}
				onkeydown={onComposerKeydown}
			/>
		</div>
		<div class="anchor">
			<button type="button" class="emoji-toggle" title="Emoji" onclick={() => (composerEmojiOpen = !composerEmojiOpen)}>
				<Smile size={18} strokeWidth={2} />
			</button>
			{#if composerEmojiOpen}
				<EmojiPicker
					onClose={() => (composerEmojiOpen = false)}
					onPick={insertEmoji}
					customEmoji={customEmojiStore.forServer(serverId ?? null)}
				/>
			{/if}
		</div>
		<button type="submit" disabled={(draft.trim().length === 0 && pendingFiles.length === 0) || uploading}>
			{#if uploading}
				<Loader2 size={16} strokeWidth={2.25} class="spin" />
			{:else}
				<SendHorizontal size={16} strokeWidth={2.25} />
			{/if}
		</button>
	</form>
</section>
{#if threadsOpen && !isDm}
	<ThreadPanel channelId={channel.id} initialThreadId={openThreadId} onClose={() => (threadsOpen = false)} />
{/if}
{#if reportTarget && session.token}
	{@const reportedId = reportedUserIdFor(reportTarget)}
	{#if reportedId}
		<ReportModal
			token={session.token}
			reportedUsername={reportTarget.author}
			reportedUserId={reportedId}
			contextKind={isDm ? "dm" : "channel"}
			contextId={channel.id}
			serverId={isDm ? undefined : serverId}
			candidates={messages.filter((m) => m.author === reportTarget?.author).slice(-30)}
			initialMessageId={reportTarget.id}
			onClose={() => (reportTarget = null)}
		/>
	{/if}
{/if}
{#if profileModalUsername}
	<FullProfileModal
		username={profileModalUsername}
		member={null}
		serverName={isDm ? "" : (channel.name ?? "")}
		onClose={() => (profileModalUsername = null)}
		onMessage={() => (profileModalUsername = null)}
	/>
{/if}
</div>

<style>
	.chat-row {
		flex: 1;
		display: flex;
		height: 100%;
		min-width: 0;
	}

	.chat {
		position: relative;
		flex: 1;
		display: flex;
		flex-direction: column;
		height: 100%;
		min-width: 0;
		background: var(--panel);
	}

	.drop-overlay {
		position: absolute;
		inset: 0;
		z-index: 50;
		display: flex;
		align-items: center;
		justify-content: center;
		background: rgba(0, 0, 0, 0.55);
		pointer-events: none;
	}

	.drop-card {
		position: relative;
		width: min(320px, 80%);
		display: flex;
		flex-direction: column;
		align-items: center;
		text-align: center;
		gap: 4px;
		padding: 36px 28px 28px;
		border-radius: 12px;
		border: 2px dashed color-mix(in srgb, var(--accent-fill) 70%, white);
		background: color-mix(in srgb, var(--accent-fill) 22%, var(--panel));
		overflow: hidden;
	}

	.drop-card :global(.drop-card-bg-icon) {
		position: absolute;
		top: -14px;
		right: -14px;
		width: 90px;
		height: 90px;
		opacity: 0.16;
		color: var(--accent-fill);
	}

	.drop-card h3 {
		margin: 4px 0 0;
		font-size: 16px;
		font-weight: 800;
		color: var(--ink);
	}

	.drop-card p {
		margin: 0;
		font-size: 12px;
		line-height: 1.4;
		color: var(--ink-dim);
	}

	.drop-card-badge {
		display: flex;
		align-items: center;
		justify-content: center;
		width: 40px;
		height: 40px;
		margin-top: 10px;
		border-radius: 9px;
		background: var(--accent-fill);
		color: var(--accent-fill-ink);
	}

	.header {
		height: 48px;
		flex-shrink: 0;
		display: flex;
		align-items: center;
		gap: 8px;
		padding: 0 16px;
		border-bottom: 1px solid var(--hairline);
		font-family: var(--font-mono);
		font-weight: 600;
		font-size: 14px;
		position: relative;
	}

	.header :global(.hash) {
		color: var(--ink-faint);
	}

	.name {
		white-space: nowrap;
		overflow: hidden;
		text-overflow: ellipsis;
		min-width: 0;
	}

	.spacer {
		flex: 1;
	}

	.anchor {
		position: relative;
	}

	.header-icons {
		display: flex;
		align-items: center;
		gap: 2px;
	}

	.header-search {
		display: flex;
		align-items: center;
		gap: 6px;
		margin: 0 4px;
		padding: 0 8px;
		background: var(--void);
		border-radius: 6px;
		color: var(--ink-faint);
	}

	.header-search input {
		width: 110px;
		background: none;
		border: none;
		padding: 6px 0;
		font-family: var(--font-body);
		font-size: 12px;
		font-weight: 400;
		color: var(--ink);
	}

	.header-search input::placeholder {
		color: var(--ink-faint);
	}

	.icon-button {
		display: flex;
		color: var(--ink-dim);
		padding: 6px;
		border-radius: 6px;
		transition: background-color 0.15s ease, color 0.15s ease;
	}

	.icon-button:hover,
	.icon-button.active {
		background: var(--hover);
		color: var(--ink);
	}

	.icon-button.small {
		padding: 4px;
	}

	.icon-button.quick-react {
		font-size: 14px;
		line-height: 1;
		align-items: center;
		justify-content: center;
	}

	.toggle-row {
		display: flex;
		align-items: center;
		justify-content: space-between;
		font-size: 13px;
		color: var(--ink);
	}

	.switch {
		position: relative;
		flex-shrink: 0;
		width: 40px;
		height: 22px;
	}

	.switch input {
		position: absolute;
		opacity: 0;
		width: 100%;
		height: 100%;
		margin: 0;
		cursor: pointer;
	}

	.track {
		display: block;
		width: 100%;
		height: 100%;
		border-radius: 999px;
		background: var(--active);
		transition: background-color 0.15s ease;
	}

	.thumb {
		position: absolute;
		top: 3px;
		left: 3px;
		width: 16px;
		height: 16px;
		border-radius: 50%;
		background: var(--ink-faint);
		transition: transform 0.15s ease, background-color 0.15s ease;
	}

	.switch input:checked + .track {
		background: var(--accent-soft);
	}

	.switch input:checked + .track .thumb {
		transform: translateX(18px);
		background: var(--ink);
	}

	.inbox-empty {
		margin: 0;
		font-size: 13px;
		color: var(--ink-dim);
	}

	.messages {
		flex: 1;
		overflow-y: auto;
		padding: 16px 16px 8px;
		display: flex;
		flex-direction: column;
	}

	.welcome {
		padding: 24px 8px 16px;
	}

	.welcome-icon {
		width: 56px;
		height: 56px;
		border-radius: 50%;
		background: var(--active);
		color: var(--ink-dim);
		display: flex;
		align-items: center;
		justify-content: center;
		margin-bottom: 12px;
	}

	.welcome h2 {
		margin: 0 0 4px;
		font-family: var(--font-display);
		font-size: 22px;
	}

	.welcome p {
		margin: 0;
		color: var(--ink-dim);
		font-size: 14px;
	}

	.message {
		position: relative;
		display: flex;
		gap: 12px;
		padding: 2px 8px;
		border-radius: 6px;
		transition: background-color 0.1s ease;
	}

	.message:hover {
		background: rgba(255, 255, 255, 0.03);
	}

	.message:not(.grouped) {
		margin-top: 12px;
	}

	.message.mentioned {
		background: color-mix(in srgb, var(--idle) 10%, transparent);
		box-shadow: inset 2px 0 0 var(--idle);
	}

	.message.mentioned:hover {
		background: color-mix(in srgb, var(--idle) 16%, transparent);
	}

	.avatar {
		width: 36px;
		height: 36px;
		border-radius: 50%;
		flex-shrink: 0;
		display: flex;
		align-items: center;
		justify-content: center;
		font-family: var(--font-mono);
		font-size: 12px;
		font-weight: 600;
		color: var(--void);
		background-size: cover;
		background-position: center;
	}

	.clickable {
		cursor: pointer;
	}

	.clickable:hover {
		text-decoration: underline;
	}

	.avatar.clickable:hover {
		text-decoration: none;
		opacity: 0.85;
	}

	.avatar-spacer {
		width: 36px;
		flex-shrink: 0;
		display: flex;
		align-items: center;
		justify-content: center;
	}

	.hover-time {
		display: none;
		font-family: var(--font-mono);
		font-size: 10px;
		color: var(--ink-faint);
	}

	.message:hover .hover-time {
		display: block;
	}

	.body {
		min-width: 0;
		flex: 1;
	}

	.meta {
		margin: 0 0 2px;
		display: flex;
		align-items: baseline;
		gap: 8px;
	}

	.meta :global(.pinned-flag) {
		color: var(--ink-faint);
	}

	.author {
		font-family: var(--font-mono);
		font-weight: 600;
		font-size: 13px;
	}

	.time {
		font-family: var(--font-mono);
		font-size: 11px;
		color: var(--ink-faint);
	}

	.content {
		margin: 0;
		font-family: var(--font-body);
		font-size: 14px;
		line-height: 1.4;
		color: var(--ink);
		word-break: break-word;
	}

	.edited-flag {
		margin-left: 4px;
		font-size: 10px;
		color: var(--ink-faint);
	}

	.content :global(strong) {
		font-weight: 700;
	}

	.content :global(em) {
		font-style: italic;
	}

	.content :global(u) {
		text-decoration: underline;
	}

	.content :global(del) {
		text-decoration: line-through;
		opacity: 0.7;
	}

	.content :global(code.md-inline) {
		background: var(--sidebar);
		border-radius: 4px;
		padding: 1px 5px;
		font-family: var(--font-mono);
		font-size: 0.9em;
	}

	.content :global(pre.md-block) {
		background: var(--sidebar);
		border-radius: 6px;
		padding: 10px 12px;
		margin: 4px 0;
		overflow-x: auto;
	}

	.content :global(pre.md-block code) {
		font-family: var(--font-mono);
		font-size: 13px;
		white-space: pre;
	}

	.content :global(.md-spoiler) {
		background: var(--ink-faint);
		color: transparent;
		border-radius: 3px;
		cursor: pointer;
		transition: background-color 0.1s ease, color 0.1s ease;
	}

	.content :global(.md-spoiler.revealed) {
		background: var(--active);
		color: var(--ink);
	}

	.content :global(.md-emoji) {
		width: 22px;
		height: 22px;
		object-fit: contain;
		vertical-align: bottom;
	}

	.content :global(a.md-link) {
		color: var(--accent-fill);
		text-decoration: none;
	}

	.content :global(a.md-link:hover) {
		text-decoration: underline;
	}

	.content :global(.md-mention) {
		background: color-mix(in srgb, var(--accent-fill) 22%, transparent);
		color: var(--accent-fill);
		border-radius: 4px;
		padding: 0 3px;
		font-weight: 600;
	}

	.content :global(.md-mention-special) {
		background: color-mix(in srgb, var(--idle) 25%, transparent);
		color: var(--idle);
	}

	.content :global(.md-mention-self) {
		background: color-mix(in srgb, var(--danger) 25%, transparent);
		color: var(--danger);
	}

	.reply-quote {
		display: flex;
		align-items: center;
		gap: 6px;
		margin: 0 0 2px;
		font-size: 12px;
		color: var(--ink-faint);
		overflow: hidden;
	}

	.reply-quote :global(svg) {
		flex-shrink: 0;
		transform: scaleX(-1);
	}

	.reply-author {
		flex-shrink: 0;
		font-weight: 600;
		color: var(--ink-dim);
	}

	.reply-snippet {
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
	}

	.edit-form {
		display: flex;
		flex-direction: column;
		gap: 6px;
	}

	.edit-form input {
		width: 100%;
		background: var(--sidebar);
		border: 1px solid var(--hairline);
		border-radius: 6px;
		padding: 8px 10px;
		font-family: var(--font-body);
		font-size: 14px;
		color: var(--ink);
	}

	.edit-form input:focus {
		outline: none;
		border-color: var(--ink-dim);
	}

	.edit-actions {
		display: flex;
		gap: 8px;
	}

	.ghost-small,
	.primary-small {
		padding: 5px 10px;
		border-radius: 5px;
		font-size: 12px;
		font-weight: 600;
	}

	.ghost-small {
		color: var(--ink-dim);
	}

	.ghost-small:hover {
		color: var(--ink);
	}

	.primary-small {
		background: var(--accent-fill);
		color: var(--accent-fill-ink);
	}

	.primary-small:disabled {
		background: var(--active);
		color: var(--ink-faint);
	}

	.attachment-image {
		display: block;
		max-width: 320px;
		margin-top: 4px;
		padding: 0;
		border-radius: 8px;
		overflow: hidden;
	}

	.attachment-video {
		position: relative;
		max-width: 320px;
		margin-top: 4px;
	}

	.attachment-video video {
		display: block;
		max-width: 100%;
		max-height: 300px;
		border-radius: 8px;
	}

	.video-expand {
		position: absolute;
		top: 8px;
		right: 8px;
		display: flex;
		align-items: center;
		justify-content: center;
		width: 26px;
		height: 26px;
		border-radius: 50%;
		background: rgba(0, 0, 0, 0.6);
		color: white;
	}

	.video-expand:hover {
		background: rgba(0, 0, 0, 0.8);
	}

	.attachment-image img {
		display: block;
		max-width: 100%;
		max-height: 300px;
		object-fit: contain;
	}

	.attachment-image.spoiler-hidden {
		position: relative;
		padding: 0;
		cursor: pointer;
	}

	.attachment-image.spoiler-hidden img,
	.attachment-image.spoiler-hidden video {
		filter: blur(24px);
	}

	.attachment-image.spoiler-hidden video {
		display: block;
		max-width: 100%;
		max-height: 300px;
	}

	.spoiler-overlay {
		position: absolute;
		inset: 0;
		display: flex;
		align-items: center;
		justify-content: center;
		gap: 6px;
		background: rgba(0, 0, 0, 0.45);
		color: white;
		font-size: 13px;
		font-weight: 700;
		text-align: center;
		padding: 8px;
	}

	.attachment-file {
		display: flex;
		align-items: center;
		gap: 10px;
		margin-top: 4px;
		padding: 10px 12px;
		max-width: 320px;
		background: var(--sidebar);
		border-radius: 8px;
		color: var(--ink);
		transition: background-color 0.15s ease;
	}

	.attachment-file:hover {
		background: var(--hover);
	}

	.attachment-file.expired {
		opacity: 0.6;
		cursor: default;
	}

	.attachment-file.expired:hover {
		background: var(--sidebar);
	}

	.attachment-file :global(svg:first-child) {
		flex-shrink: 0;
		color: var(--ink-faint);
	}

	.attachment-info {
		flex: 1;
		min-width: 0;
		display: flex;
		flex-direction: column;
		align-items: flex-start;
	}

	.attachment-name {
		font-size: 13px;
		font-weight: 600;
		color: var(--ink);
		max-width: 100%;
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
	}

	.attachment-size {
		font-size: 11px;
		color: var(--ink-faint);
	}

	.link-preview {
		position: relative;
		display: flex;
		gap: 12px;
		margin-top: 6px;
		padding: 10px 28px 10px 12px;
		max-width: 420px;
		background: var(--sidebar);
		border-left: 3px solid var(--accent-fill);
		border-radius: 4px;
	}

	.link-preview-close {
		position: absolute;
		top: 6px;
		right: 6px;
		padding: 3px;
		border-radius: 999px;
		color: var(--ink-faint);
		opacity: 0;
		transition: opacity 0.15s ease, background-color 0.15s ease, color 0.15s ease;
	}

	.link-preview:hover .link-preview-close {
		opacity: 1;
	}

	.link-preview-close:hover {
		background: var(--hover);
		color: var(--ink);
	}

	.link-preview-body {
		flex: 1;
		min-width: 0;
		display: flex;
		flex-direction: column;
		gap: 4px;
	}

	.link-preview-site {
		font-size: 11px;
		font-weight: 600;
		color: var(--ink-faint);
	}

	.link-preview-title {
		font-size: 14px;
		font-weight: 700;
		color: var(--accent-fill);
	}

	.link-preview-title:hover {
		text-decoration: underline;
	}

	.link-preview-desc {
		margin: 0;
		font-size: 12px;
		line-height: 1.4;
		color: var(--ink-dim);
		display: -webkit-box;
		-webkit-line-clamp: 3;
		line-clamp: 3;
		-webkit-box-orient: vertical;
		overflow: hidden;
	}

	.link-preview-image {
		flex-shrink: 0;
		width: 64px;
		height: 64px;
		border-radius: 4px;
		object-fit: cover;
	}

	.attachment-preview-row {
		flex-shrink: 0;
		display: flex;
		flex-wrap: wrap;
		gap: 12px;
		margin: 0 16px;
		padding-top: 10px;
	}

	.upload-banner {
		flex-shrink: 0;
		display: flex;
		align-items: center;
		gap: 10px;
		margin: 10px 16px 0;
		padding: 10px 14px;
		border-radius: 8px;
		background: var(--panel);
		border: 1px solid var(--hairline);
		color: var(--ink-dim);
	}

	.upload-banner-body {
		flex: 1;
		min-width: 0;
		display: flex;
		flex-direction: column;
		gap: 6px;
	}

	.upload-banner-title {
		font-size: 12px;
		font-weight: 600;
		color: var(--ink);
	}

	.upload-banner-track {
		height: 4px;
		border-radius: 999px;
		background: var(--hover);
		overflow: hidden;
	}

	.upload-banner-fill {
		height: 100%;
		background: var(--accent-fill);
		transition: width 0.2s ease;
	}

	.attachment-card {
		display: flex;
		flex-direction: column;
		gap: 6px;
		width: 140px;
	}

	.attachment-thumb {
		position: relative;
		width: 140px;
		height: 100px;
		border-radius: 10px;
		background: var(--active);
		display: flex;
		align-items: center;
		justify-content: center;
		color: var(--ink-faint);
		overflow: hidden;
	}

	.attachment-thumb img {
		width: 100%;
		height: 100%;
		object-fit: cover;
	}

	.attachment-hover-actions {
		position: absolute;
		inset: 0;
		display: flex;
		align-items: center;
		justify-content: center;
		gap: 8px;
		background: rgba(0, 0, 0, 0.55);
		opacity: 0;
		transition: opacity 0.15s ease;
	}

	.attachment-thumb:hover .attachment-hover-actions {
		opacity: 1;
	}

	.attachment-hover-actions button {
		display: flex;
		align-items: center;
		justify-content: center;
		width: 28px;
		height: 28px;
		border-radius: 50%;
		background: rgba(255, 255, 255, 0.12);
		color: white;
		transition: background-color 0.15s ease;
	}

	.attachment-hover-actions button:hover {
		background: rgba(255, 255, 255, 0.24);
	}

	.attachment-hover-actions button.active {
		background: var(--danger);
		color: white;
	}

	:global(.spin) {
		animation: spin 0.7s linear infinite;
	}

	@keyframes spin {
		from {
			transform: rotate(0deg);
		}
		to {
			transform: rotate(360deg);
		}
	}

	.attachment-uploading {
		position: absolute;
		inset: 0;
		display: flex;
		align-items: center;
		justify-content: center;
		background: rgba(0, 0, 0, 0.55);
		color: white;
	}

	.attachment-preview-name {
		font-size: 12px;
		color: var(--ink-faint);
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
	}

	.spoiler-tag {
		position: absolute;
		top: 6px;
		left: 6px;
		padding: 2px 6px;
		border-radius: 4px;
		background: rgba(0, 0, 0, 0.6);
		color: white;
		font-size: 10px;
		font-weight: 700;
		text-transform: uppercase;
		letter-spacing: 0.03em;
	}

	.attachment-thumb.spoiler-blur img {
		filter: blur(16px);
	}

	.reactions {
		display: flex;
		flex-wrap: wrap;
		gap: 6px;
		margin-top: 6px;
	}

	.reaction {
		display: flex;
		align-items: center;
		gap: 4px;
		padding: 2px 8px;
		border-radius: 999px;
		background: var(--active);
		border: 1px solid transparent;
		font-size: 12px;
		color: var(--ink-dim);
		transition: background-color 0.15s ease, border-color 0.15s ease;
	}

	.reaction.reacted {
		background: var(--accent-soft);
		border-color: var(--ink-dim);
		color: var(--ink);
	}

	.hover-actions {
		position: absolute;
		top: -14px;
		right: 8px;
		display: none;
		background: var(--active);
		border-radius: 6px;
		padding: 2px;
		box-shadow: 0 2px 6px rgba(0, 0, 0, 0.3);
	}

	.message:hover .hover-actions {
		display: flex;
	}

	.reply-banner {
		flex-shrink: 0;
		display: flex;
		align-items: center;
		gap: 8px;
		margin: 0 16px;
		padding: 8px 12px;
		background: var(--active);
		border-radius: 8px 8px 0 0;
		font-size: 12px;
		color: var(--ink-dim);
	}

	.reply-banner strong {
		color: var(--ink);
	}

	.cancel-reply {
		margin-left: auto;
		display: flex;
		color: var(--ink-faint);
		padding: 2px;
		border-radius: 4px;
	}

	.cancel-reply:hover {
		color: var(--ink);
	}

	.composer {
		flex-shrink: 0;
		display: flex;
		align-items: center;
		gap: 8px;
		padding: 0 16px 16px;
	}

	.attach,
	.emoji-toggle {
		flex-shrink: 0;
		display: flex;
		padding: 8px;
		border-radius: 8px;
		color: var(--ink-dim);
		transition: background-color 0.15s ease, color 0.15s ease;
	}

	.attach:hover,
	.emoji-toggle:hover {
		background: var(--hover);
		color: var(--ink);
	}

	.composer-input-wrap {
		flex: 1;
	}

	.mention-popup {
		position: absolute;
		bottom: calc(100% + 8px);
		left: 0;
		right: 0;
		background: var(--panel);
		border-radius: 8px;
		padding: 6px;
		max-height: 220px;
		overflow-y: auto;
		box-shadow: 0 12px 32px rgba(0, 0, 0, 0.4);
		z-index: 60;
	}

	.mention-item {
		display: flex;
		align-items: center;
		gap: 8px;
		width: 100%;
		text-align: left;
		padding: 6px 10px;
		border-radius: 6px;
		font-size: 13px;
		font-weight: 600;
		color: var(--ink-dim);
	}

	.mention-item:hover,
	.mention-item.active {
		background: var(--hover);
		color: var(--ink);
	}

	.mention-avatar {
		flex-shrink: 0;
		width: 22px;
		height: 22px;
		border-radius: 50%;
		background-size: cover;
		background-position: center;
		display: flex;
		align-items: center;
		justify-content: center;
		font-family: var(--font-mono);
		font-size: 9px;
		font-weight: 700;
		color: var(--accent-fill-ink);
	}

	.mention-avatar-broadcast {
		background: var(--active);
		color: var(--ink-dim);
	}

	.composer input {
		width: 100%;
		background: var(--active);
		border-radius: 8px;
		padding: 10px 12px;
		color: var(--ink);
		border: none;
		font-family: var(--font-body);
		font-size: 14px;
	}

	.composer input::placeholder {
		color: var(--ink-faint);
	}

	.composer button[type="submit"] {
		flex-shrink: 0;
		display: flex;
		align-items: center;
		justify-content: center;
		width: 40px;
		height: 38px;
		border-radius: 8px;
		background: var(--accent-fill);
		color: var(--accent-fill-ink);
	}

	.composer button[type="submit"]:disabled {
		background: var(--active);
		color: var(--ink-faint);
		cursor: default;
	}
</style>

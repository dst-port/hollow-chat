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
	import Download from "@lucide/svelte/icons/download";
	import MessagesSquare from "@lucide/svelte/icons/messages-square";
	import Phone from "@lucide/svelte/icons/phone";
	import PinnedPopover from "$lib/components/PinnedPopover.svelte";
	import InfoPopover from "$lib/components/InfoPopover.svelte";
	import MessageMenu from "$lib/components/MessageMenu.svelte";
	import EmojiPicker from "$lib/components/EmojiPicker.svelte";
	import ThreadPanel from "$lib/components/ThreadPanel.svelte";
	import { emojify } from "$lib/actions/emojify";
	import { toast } from "$lib/stores/toast.svelte";
	import { session } from "$lib/stores/session.svelte";
	import { profileStore } from "$lib/stores/profile.svelte";
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
		PERMISSIONS,
		ApiError,
		type ApiMessage,
		type ApiReplyPreview,
		type MessageScope
	} from "$lib/api/client";
	import { colorForName } from "$lib/utils/color";
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
	import { call } from "$lib/webrtc/call.svelte";
	import { loadAttachmentBlobUrl, triggerDownload } from "$lib/utils/attachment";
	import { renderMarkdown } from "$lib/utils/markdown";
	import type { Channel, Message, MessageAttachment } from "$lib/data/mock";

	const DEFAULT_QUICK_EMOJI = ["👍", "❤️", "😂", "🔥", "🎉"];
	const POLL_INTERVAL_MS = 3000;

	let { channel, isDm = false, serverId, onToggleMembers }: {
		channel: Channel;
		isDm?: boolean;
		serverId?: string;
		onToggleMembers?: () => void;
	} = $props();

	const scope: MessageScope = isDm ? "dm" : "channel";

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

	function ensureProfileLoaded(username: string) {
		const token = session.token;
		if (token && !profileStore.forUser(username)) profileStore.load(token, username);
	}

	function displayNameFor(username: string): string {
		return profileStore.forUser(username)?.display_name || username;
	}

	async function toReplyPreview(reply: ApiReplyPreview | null) {
		if (!reply) return undefined;
		const content = reply.content
			? await decryptStoredContent(reply.author, reply.id, reply.content)
			: reply.has_attachment
				? "📎 Attachment"
				: "";
		return { id: reply.id, author: reply.author, content, hasAttachment: reply.has_attachment };
	}

	async function toMessage(apiMsg: ApiMessage): Promise<Message> {
		const content = apiMsg.content
			? await decryptStoredContent(apiMsg.author, apiMsg.id, apiMsg.content)
			: "";
		ensureProfileLoaded(apiMsg.author);

		return {
			id: apiMsg.id,
			author: apiMsg.author,
			color: colorForName(apiMsg.author),
			content,
			attachment: apiMsg.attachment
				? {
						id: apiMsg.attachment.id,
						filename: apiMsg.attachment.filename,
						mimeType: apiMsg.attachment.mime_type,
						sizeBytes: apiMsg.attachment.size_bytes
					}
				: undefined,
			reactions: apiMsg.reactions.map((r) => ({ emoji: r.emoji, count: r.count, reacted: r.reacted })),
			pinned: apiMsg.pinned,
			replyTo: await toReplyPreview(apiMsg.reply_to),
			edited: !!apiMsg.edited_at,
			time: new Date(apiMsg.timestamp).toLocaleTimeString([], { hour: "2-digit", minute: "2-digit" })
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
						if (!known.has(row.id)) messages.push(await toMessage(row));
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
	let pendingFile = $state<File | null>(null);
	let uploading = $state(false);
	let fileInputEl = $state<HTMLInputElement | undefined>();
	let replyingTo = $state<Message | null>(null);
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

	$effect(() => {
		const token = session.token;
		if (!token) return;
		for (const message of messages) {
			const attachment = message.attachment;
			if (attachment && attachment.mimeType.startsWith("image/") && !imageUrls[attachment.id]) {
				loadAttachmentBlobUrl(token, attachment.id, attachment.filename)
					.then((url) => {
						imageUrls[attachment.id] = url;
					})
					.catch(() => {});
			}
		}
	});

	async function downloadAttachment(attachment: MessageAttachment) {
		const token = session.token;
		if (!token) return;
		try {
			const url = await loadAttachmentBlobUrl(token, attachment.id, attachment.filename);
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
		pendingFile = input.files?.[0] ?? null;
		input.value = "";
	}

	function clearPendingFile() {
		pendingFile = null;
	}

	function formatSize(bytes: number): string {
		if (bytes < 1024) return `${bytes} B`;
		if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`;
		if (bytes < 1024 * 1024 * 1024) return `${(bytes / (1024 * 1024)).toFixed(1)} MB`;
		return `${(bytes / (1024 * 1024 * 1024)).toFixed(2)} GB`;
	}

	async function send(event: SubmitEvent) {
		event.preventDefault();
		const content = draft.trim();
		const token = session.token;
		const myUsername = session.username;
		const file = pendingFile;
		const replyToId = replyingTo?.id;
		if ((!content && !file) || !token) return;

		draft = "";
		pendingFile = null;
		replyingTo = null;

		try {
			let attachmentId: string | undefined;
			if (file) {
				uploading = true;
				const uploaded = await uploadFile(token, file);
				attachmentId = uploaded.id;
			}

			let payload: string | null = content || null;
			if (myUsername && content) {
				payload = await encryptOutgoing(myUsername, token, content);
			}

			const apiMsg = await postMessage(token, channel.id, payload, attachmentId, replyToId);
			if (content) rememberDecrypted(apiMsg.id, content);
			messages.push(await toMessage(apiMsg));
			lastId = apiMsg.id;
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
		}
	}

	function isGrouped(index: number) {
		if (index === 0) return false;
		return (
			messages[index - 1].author === messages[index].author &&
			!messages[index].replyTo
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
			let payload = content;
			if (session.username) {
				payload = await encryptOutgoing(session.username, token, content);
			}
			await apiEditMessage(token, scope, channel.id, message.id, payload);
			rememberDecrypted(message.id, content);
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
<section class="chat">
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
						<label class="toggle-row">
							<span>Mute channel</span>
							<input type="checkbox" bind:checked={muted} />
						</label>
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
				<div class="message" class:grouped={isGrouped(index)} in:fly={{ y: 6, duration: 180, delay: index * 20 }}>
					{#if !isGrouped(index)}
						{@const authorAvatarUrl = profileStore.forUser(message.author)?.avatar_url}
						<div
							class="avatar"
							style:background={authorAvatarUrl ? undefined : message.color}
							style:background-image={authorAvatarUrl ? `url(${resolveUrl(authorAvatarUrl)})` : undefined}
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
								<span class="author" style:color={message.color}>{displayNameFor(message.author)}</span>
								<span class="time">{message.time}</span>
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
								{@html renderMarkdown(message.content)}
								{#if message.edited && isGrouped(index)}<span class="edited-flag">(edited)</span>{/if}
							</p>
						{/if}
						{#if message.attachment}
							{#if message.attachment.mimeType.startsWith("image/") && imageUrls[message.attachment.id]}
								<a
									class="attachment-image"
									href={imageUrls[message.attachment.id]}
									target="_blank"
									rel="noreferrer"
								>
									<img src={imageUrls[message.attachment.id]} alt={message.attachment.filename} />
								</a>
							{:else}
								<button class="attachment-file" onclick={() => downloadAttachment(message.attachment!)}>
									<FileIcon size={20} strokeWidth={2} />
									<span class="attachment-info">
										<span class="attachment-name">{message.attachment.filename}</span>
										<span class="attachment-size">{formatSize(message.attachment.sizeBytes)}</span>
									</span>
									<Download size={16} strokeWidth={2} />
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
									onClose={() => (openMenuId = null)}
									onCopy={() => copyText(message)}
									onTogglePin={() => togglePin(message)}
									onEdit={() => startEdit(message)}
									onDelete={() => deleteMessage(message)}
									onCreateThread={isDm ? undefined : () => createThread(message)}
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

	{#if pendingFile}
		<div class="pending-file" transition:fly={{ y: 8, duration: 140 }}>
			<FileIcon size={16} strokeWidth={2} />
			<span class="pending-name">{pendingFile.name}</span>
			<span class="pending-size">{formatSize(pendingFile.size)}</span>
			<button type="button" class="cancel-reply" onclick={clearPendingFile} title="Remove file">
				<X size={14} strokeWidth={2} />
			</button>
		</div>
	{/if}

	<form class="composer" onsubmit={send}>
		<input
			type="file"
			bind:this={fileInputEl}
			onchange={onFileChosen}
			style="display: none;"
		/>
		<button type="button" class="attach" title="Upload a file" onclick={pickFile}>
			<Paperclip size={18} strokeWidth={2} />
		</button>
		<input
			type="text"
			placeholder={isDm ? `Message ${channel.name}` : `Message #${channel.name}`}
			bind:value={draft}
		/>
		<div class="anchor">
			<button type="button" class="emoji-toggle" title="Emoji" onclick={() => (composerEmojiOpen = !composerEmojiOpen)}>
				<Smile size={18} strokeWidth={2} />
			</button>
			{#if composerEmojiOpen}
				<EmojiPicker onClose={() => (composerEmojiOpen = false)} onPick={insertEmoji} />
			{/if}
		</div>
		<button type="submit" disabled={(draft.trim().length === 0 && !pendingFile) || uploading}>
			<SendHorizontal size={16} strokeWidth={2.25} />
		</button>
	</form>
</section>
{#if threadsOpen && !isDm}
	<ThreadPanel channelId={channel.id} initialThreadId={openThreadId} onClose={() => (threadsOpen = false)} />
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
		flex: 1;
		display: flex;
		flex-direction: column;
		height: 100%;
		min-width: 0;
		background: var(--panel);
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
		border-radius: 8px;
		overflow: hidden;
	}

	.attachment-image img {
		display: block;
		max-width: 100%;
		max-height: 300px;
		object-fit: contain;
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

	.pending-file {
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

	.pending-name {
		flex: 1;
		min-width: 0;
		color: var(--ink);
		font-weight: 600;
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
	}

	.pending-size {
		color: var(--ink-faint);
		flex-shrink: 0;
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

	.composer input {
		flex: 1;
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

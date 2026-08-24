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
	import PinnedPopover from "$lib/components/PinnedPopover.svelte";
	import InfoPopover from "$lib/components/InfoPopover.svelte";
	import MessageMenu from "$lib/components/MessageMenu.svelte";
	import EmojiPicker from "$lib/components/EmojiPicker.svelte";
	import { emojify } from "$lib/actions/emojify";
	import { toast } from "$lib/stores/toast.svelte";
	import { session } from "$lib/stores/session.svelte";
	import UserRound from "@lucide/svelte/icons/user-round";
	import {
		recordEmojiUse,
		frequentEmoji,
		listMessages,
		sendMessage,
		listDmMessages,
		sendDmMessage,
		type ApiMessage
	} from "$lib/api/client";
	import { colorForName } from "$lib/utils/color";
	import { encryptForPeer, decryptFromPeer } from "$lib/crypto/dm";
	import { rememberSent, recallSent } from "$lib/crypto/sent-cache";
	import type { Channel, Message } from "$lib/data/mock";

	const DEFAULT_QUICK_EMOJI = ["👍", "❤️", "😂", "🔥", "🎉"];
	const POLL_INTERVAL_MS = 3000;

	let { channel, isDm = false, onToggleMembers }: {
		channel: Channel;
		isDm?: boolean;
		onToggleMembers?: () => void;
	} = $props();

	const fetchMessages = $derived(isDm ? listDmMessages : listMessages);
	const postMessage = $derived(isDm ? sendDmMessage : sendMessage);

	async function toMessage(apiMsg: ApiMessage): Promise<Message> {
		let content = apiMsg.content;
		const myUsername = session.username;

		if (isDm && myUsername) {
			if (apiMsg.author === myUsername) {
				content = recallSent(apiMsg.id) ?? "[sent from another device]";
			} else {
				try {
					content = await decryptFromPeer(myUsername, channel.name, apiMsg.content);
				} catch {
					content = "[unable to decrypt message]";
				}
			}
		}

		return {
			id: apiMsg.id,
			author: apiMsg.author,
			color: colorForName(apiMsg.author),
			content,
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

	$effect(() => {
		const token = session.token;
		const channelId = channel.id;
		const fetcher = fetchMessages;
		if (!token) return;

		messages = [];
		lastId = null;
		let cancelled = false;

		fetcher(token, channelId)
			.then(async (rows) => {
				if (cancelled) return;
				const converted = await toMessages(rows);
				if (cancelled) return;
				messages = converted;
				lastId = rows.at(-1)?.id ?? lastId;
			})
			.catch(() => {});

		const interval = setInterval(() => {
			if (!lastId) return;
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
				.catch(() => {});
		}, POLL_INTERVAL_MS);

		return () => {
			cancelled = true;
			clearInterval(interval);
		};
	});

	let draft = $state("");
	let replyingTo = $state<Message | null>(null);
	let openMenuId = $state<string | null>(null);
	let composerEmojiOpen = $state(false);
	let pinnedOpen = $state(false);
	let notificationsOpen = $state(false);
	let inboxOpen = $state(false);
	let muted = $state(false);
	let emojiCounts = $state<Record<string, number>>({});

	const pinnedMessages = $derived(messages.filter((m) => m.pinned));
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

	async function send(event: SubmitEvent) {
		event.preventDefault();
		const content = draft.trim();
		const token = session.token;
		const myUsername = session.username;
		if (!content || !token) return;
		draft = "";
		replyingTo = null;

		try {
			let payload = content;
			if (isDm && myUsername) {
				payload = await encryptForPeer(token, myUsername, channel.name, content);
			}
			const apiMsg = await postMessage(token, channel.id, payload);
			if (isDm) rememberSent(apiMsg.id, content);
			messages.push(await toMessage(apiMsg));
			lastId = apiMsg.id;
		} catch {
			toast.push("Message failed to send");
		}
	}

	function isGrouped(index: number) {
		if (index === 0) return false;
		return messages[index - 1].author === messages[index].author;
	}

	function copyText(message: Message) {
		navigator.clipboard.writeText(message.content);
		toast.push("Copied");
	}

	function togglePin(message: Message) {
		message.pinned = !message.pinned;
		toast.push(message.pinned ? "Message pinned" : "Message unpinned");
	}

	function deleteMessage(message: Message) {
		messages = messages.filter((m) => m.id !== message.id);
		toast.push("Message deleted");
	}

	function toggleReaction(message: Message, emoji: string) {
		if (!message.reactions) message.reactions = [];
		const existing = message.reactions.find((r) => r.emoji === emoji);
		if (existing) {
			existing.reacted = !existing.reacted;
			existing.count += existing.reacted ? 1 : -1;
			if (existing.count <= 0) {
				message.reactions = message.reactions.filter((r) => r.emoji !== emoji);
			}
		} else {
			message.reactions.push({ emoji, count: 1, reacted: true });
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
</script>

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
			<div class="anchor">
				<button class="icon-button" title="Pinned messages" onclick={() => (pinnedOpen = !pinnedOpen)}>
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

	<div class="messages">
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
		{#key channel.id}
			{#each messages as message, index (message.id)}
				<div class="message" class:grouped={isGrouped(index)} in:fly={{ y: 6, duration: 180, delay: index * 20 }}>
					{#if !isGrouped(index)}
						<div class="avatar" style:background={message.color}>
							{message.author.slice(0, 2).toUpperCase()}
						</div>
					{:else}
						<div class="avatar-spacer">
							<span class="hover-time">{message.time}</span>
						</div>
					{/if}

					<div class="body">
						{#if !isGrouped(index)}
							<p class="meta">
								<span class="author" style:color={message.color}>{message.author}</span>
								<span class="time">{message.time}</span>
								{#if message.pinned}<Pin size={11} strokeWidth={2.5} class="pinned-flag" />{/if}
							</p>
						{/if}
						<p class="content" use:emojify>{message.content}</p>
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
									onClose={() => (openMenuId = null)}
									onCopy={() => copyText(message)}
									onTogglePin={() => togglePin(message)}
									onDelete={() => deleteMessage(message)}
								/>
							{/if}
						</div>
					</div>
				</div>
			{/each}
		{/key}
	</div>

	{#if replyingTo}
		<div class="reply-banner" transition:fly={{ y: 8, duration: 140 }}>
			<Reply size={14} strokeWidth={2} />
			<span>Replying to <strong>{replyingTo.author}</strong></span>
			<button class="cancel-reply" onclick={() => (replyingTo = null)}>
				<X size={14} strokeWidth={2} />
			</button>
		</div>
	{/if}

	<form class="composer" onsubmit={send}>
		<button type="button" class="attach" title="Upload a file" onclick={() => toast.push("File uploads aren't wired up yet")}>
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
		<button type="submit" disabled={draft.trim().length === 0}>
			<SendHorizontal size={16} strokeWidth={2.25} />
		</button>
	</form>
</section>

<style>
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

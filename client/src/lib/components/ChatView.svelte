<script lang="ts">
	import { fade, fly } from "svelte/transition";
	import Hash from "@lucide/svelte/icons/hash";
	import Pin from "@lucide/svelte/icons/pin";
	import Bell from "@lucide/svelte/icons/bell";
	import Users from "@lucide/svelte/icons/users";
	import Inbox from "@lucide/svelte/icons/inbox";
	import Search from "@lucide/svelte/icons/search";
	import SmilePlus from "@lucide/svelte/icons/smile-plus";
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
	import { toast } from "$lib/stores/toast.svelte";
	import type { Channel, Message } from "$lib/data/mock";

	let { channel, messages = $bindable(), onToggleMembers }: {
		channel: Channel;
		messages: Message[];
		onToggleMembers: () => void;
	} = $props();

	let draft = $state("");
	let replyingTo = $state<Message | null>(null);
	let openMenuId = $state<string | null>(null);
	let composerEmojiOpen = $state(false);
	let pinnedOpen = $state(false);
	let notificationsOpen = $state(false);
	let inboxOpen = $state(false);
	let muted = $state(false);

	const pinnedMessages = $derived(messages.filter((m) => m.pinned));

	function send(event: SubmitEvent) {
		event.preventDefault();
		if (!draft.trim()) return;
		messages.push({
			id: crypto.randomUUID(),
			author: "you",
			color: "#9a9ba1",
			content: draft.trim(),
			time: new Date().toLocaleTimeString([], { hour: "2-digit", minute: "2-digit" })
		});
		draft = "";
		replyingTo = null;
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
	}
</script>

<section class="chat">
	<header class="header">
		<Hash size={18} strokeWidth={2.5} class="hash" />
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
			<button class="icon-button" title="Members" onclick={onToggleMembers}>
				<Users size={17} strokeWidth={2} />
			</button>
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
				<div class="welcome-icon"><Hash size={28} strokeWidth={2} /></div>
				<h2>Welcome to #{channel.name}</h2>
				<p>This is the start of the channel.</p>
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
						<p class="content">{message.content}</p>
						{#if message.reactions && message.reactions.length > 0}
							<div class="reactions">
								{#each message.reactions as reaction (reaction.emoji)}
									<button
										class="reaction"
										class:reacted={reaction.reacted}
										onclick={() => toggleReaction(message, reaction.emoji)}
									>
										{reaction.emoji} {reaction.count}
									</button>
								{/each}
							</div>
						{/if}
					</div>

					<div class="hover-actions">
						<button class="icon-button small" title="Add reaction" onclick={() => toggleReaction(message, "👍")}>
							<SmilePlus size={15} strokeWidth={2} />
						</button>
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
			placeholder={`Message #${channel.name}`}
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

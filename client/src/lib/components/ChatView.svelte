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
	import type { Channel, Message } from "$lib/data/mock";

	let { channel, messages, onToggleMembers }: {
		channel: Channel;
		messages: Message[];
		onToggleMembers: () => void;
	} = $props();

	let draft = $state("");

	function send(event: SubmitEvent) {
		event.preventDefault();
		draft = "";
	}

	function isGrouped(index: number) {
		if (index === 0) return false;
		return messages[index - 1].author === messages[index].author;
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
			<button class="icon-button" title="Pinned messages"><Pin size={17} strokeWidth={2} /></button>
			<button class="icon-button" title="Notifications"><Bell size={17} strokeWidth={2} /></button>
			<button class="icon-button" title="Members" onclick={onToggleMembers}>
				<Users size={17} strokeWidth={2} />
			</button>
			<div class="header-search">
				<Search size={13} strokeWidth={2.5} />
				<input type="text" placeholder="Search" />
			</div>
			<button class="icon-button" title="Inbox"><Inbox size={17} strokeWidth={2} /></button>
		</div>
	</header>

	<div class="messages">
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
						</p>
					{/if}
					<p class="content">{message.content}</p>
				</div>

				<div class="hover-actions">
					<button class="icon-button small" title="Add reaction"><SmilePlus size={15} strokeWidth={2} /></button>
					<button class="icon-button small" title="Reply"><Reply size={15} strokeWidth={2} /></button>
					<button class="icon-button small" title="More"><MoreHorizontal size={15} strokeWidth={2} /></button>
				</div>
			</div>
		{/each}
		{/key}
	</div>

	<form class="composer" onsubmit={send}>
		<input
			type="text"
			placeholder={`Message #${channel.name}`}
			bind:value={draft}
		/>
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
	}

	.header :global(.hash) {
		color: var(--ink-faint);
	}

	.spacer {
		flex: 1;
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

	.icon-button:hover {
		background: var(--hover);
		color: var(--ink);
	}

	.icon-button.small {
		padding: 4px;
	}

	.messages {
		flex: 1;
		overflow-y: auto;
		padding: 16px 16px 8px;
		display: flex;
		flex-direction: column;
	}

	.message {
		transition: background-color 0.1s ease;
		position: relative;
		display: flex;
		gap: 12px;
		padding: 2px 8px;
		border-radius: 6px;
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
	}

	.meta {
		margin: 0 0 2px;
		display: flex;
		align-items: baseline;
		gap: 8px;
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

	.composer {
		flex-shrink: 0;
		display: flex;
		gap: 8px;
		padding: 0 16px 16px;
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

	.composer button {
		transition: background-color 0.15s ease, color 0.15s ease, transform 0.05s ease;
		display: flex;
		align-items: center;
		justify-content: center;
		width: 40px;
		border-radius: 8px;
		background: var(--accent-fill);
		color: var(--accent-fill-ink);
	}

	.composer button:disabled {
		background: var(--active);
		color: var(--ink-faint);
		cursor: default;
	}
</style>

<script lang="ts">
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
		<span class="hash">#</span>
		<span class="name">{channel.name}</span>
		<div class="spacer"></div>
		<div class="header-icons">
			<button class="icon-button" title="Pinned messages">📌</button>
			<button class="icon-button" title="Notifications">🔔</button>
			<button class="icon-button" title="Members" onclick={onToggleMembers}>◈</button>
			<div class="header-search">
				<input type="text" placeholder="Search" />
			</div>
			<button class="icon-button" title="Inbox">📥</button>
		</div>
	</header>

	<div class="messages">
		{#each messages as message, index (message.id)}
			<div class="message" class:grouped={isGrouped(index)}>
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
					<button class="icon-button small" title="Add reaction">☺</button>
					<button class="icon-button small" title="Reply">↩</button>
					<button class="icon-button small" title="More">⋯</button>
				</div>
			</div>
		{/each}
	</div>

	<form class="composer" onsubmit={send}>
		<input
			type="text"
			placeholder={`Message #${channel.name}`}
			bind:value={draft}
		/>
		<button type="submit" disabled={draft.trim().length === 0}>Send</button>
	</form>
</section>

<style>
	.chat {
		flex: 1;
		display: flex;
		flex-direction: column;
		height: 100%;
		min-width: 0;
		background: var(--bg-main);
	}

	.header {
		height: 48px;
		flex-shrink: 0;
		display: flex;
		align-items: center;
		gap: 8px;
		padding: 0 16px;
		border-bottom: 1px solid var(--border);
		font-weight: 600;
	}

	.header .hash {
		color: var(--text-faint);
		font-weight: 700;
	}

	.spacer {
		flex: 1;
	}

	.header-icons {
		display: flex;
		align-items: center;
		gap: 4px;
	}

	.header-search {
		margin: 0 4px;
	}

	.header-search input {
		width: 120px;
		background: var(--bg-rail);
		border: none;
		border-radius: 6px;
		padding: 5px 8px;
		font-size: 12px;
		color: var(--text-primary);
	}

	.header-search input::placeholder {
		color: var(--text-faint);
	}

	.icon-button {
		font-size: 15px;
		color: var(--text-muted);
		padding: 6px;
		border-radius: 6px;
	}

	.icon-button:hover {
		background: var(--bg-hover);
		color: var(--text-primary);
	}

	.icon-button.small {
		font-size: 13px;
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
		font-size: 12px;
		font-weight: 700;
		color: white;
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
		font-size: 10px;
		color: var(--text-faint);
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
		font-weight: 600;
		font-size: 14px;
	}

	.time {
		font-size: 11px;
		color: var(--text-faint);
	}

	.content {
		margin: 0;
		font-size: 14px;
		line-height: 1.4;
		color: var(--text-primary);
		word-break: break-word;
	}

	.hover-actions {
		position: absolute;
		top: -14px;
		right: 8px;
		display: none;
		background: var(--bg-active);
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
		background: var(--bg-active);
		border-radius: 8px;
		padding: 10px 12px;
		color: var(--text-primary);
		border: none;
		font-size: 14px;
	}

	.composer input::placeholder {
		color: var(--text-faint);
	}

	.composer button {
		padding: 0 16px;
		border-radius: 8px;
		background: var(--accent);
		color: white;
		font-weight: 600;
		font-size: 13px;
	}

	.composer button:disabled {
		background: var(--bg-active);
		color: var(--text-faint);
		cursor: default;
	}
</style>

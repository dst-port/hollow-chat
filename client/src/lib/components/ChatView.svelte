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
		<button class="members-toggle" title="Members" onclick={onToggleMembers}>◈</button>
	</header>

	<div class="messages">
		{#each messages as message, index (message.id)}
			<div class="message" class:grouped={isGrouped(index)}>
				{#if !isGrouped(index)}
					<div class="avatar" style:background={message.color}>
						{message.author.slice(0, 2).toUpperCase()}
					</div>
				{:else}
					<div class="avatar-spacer"></div>
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

	.members-toggle {
		font-size: 16px;
		color: var(--text-muted);
		padding: 6px;
		border-radius: 6px;
	}

	.members-toggle:hover {
		background: var(--bg-hover);
		color: var(--text-primary);
	}

	.messages {
		flex: 1;
		overflow-y: auto;
		padding: 16px 16px 8px;
		display: flex;
		flex-direction: column;
	}

	.message {
		display: flex;
		gap: 12px;
		padding: 2px 8px;
		border-radius: 6px;
	}

	.message:hover {
		background: rgba(255, 255, 255, 0.02);
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

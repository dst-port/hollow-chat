<script lang="ts">
	import type { ServerEntry } from "$lib/data/mock";

	let { server, activeChannelId, onSelectChannel }: {
		server: ServerEntry;
		activeChannelId: string;
		onSelectChannel: (id: string) => void;
	} = $props();

	const textChannels = $derived(server.channels.filter((c) => c.type === "text"));
	const voiceChannels = $derived(server.channels.filter((c) => c.type === "voice"));
</script>

<aside class="sidebar">
	<button class="header">
		<span>{server.name}</span>
		<span class="chevron">⌄</span>
	</button>

	<div class="channels">
		<div class="section">
			<p class="label">Text Channels</p>
			{#each textChannels as channel (channel.id)}
				<button
					class="channel"
					class:active={channel.id === activeChannelId}
					onclick={() => onSelectChannel(channel.id)}
				>
					<span class="hash">#</span>
					<span class="name">{channel.name}</span>
				</button>
			{/each}
		</div>

		<div class="section">
			<p class="label">Voice Channels</p>
			{#each voiceChannels as channel (channel.id)}
				<button
					class="channel"
					class:active={channel.id === activeChannelId}
					onclick={() => onSelectChannel(channel.id)}
				>
					<span class="mic">))</span>
					<span class="name">{channel.name}</span>
				</button>
			{/each}
		</div>
	</div>

	<div class="user-panel">
		<div class="avatar">U</div>
		<div class="identity">
			<p class="username">you</p>
			<p class="status">online</p>
		</div>
		<button class="settings" title="Settings">⚙</button>
	</div>
</aside>

<style>
	.sidebar {
		width: 240px;
		flex-shrink: 0;
		background: var(--bg-sidebar);
		display: flex;
		flex-direction: column;
		height: 100%;
	}

	.header {
		height: 48px;
		flex-shrink: 0;
		display: flex;
		align-items: center;
		justify-content: space-between;
		padding: 0 16px;
		font-weight: 600;
		border-bottom: 1px solid var(--border);
	}

	.header:hover {
		background: var(--bg-hover);
	}

	.chevron {
		color: var(--text-muted);
	}

	.channels {
		flex: 1;
		overflow-y: auto;
		padding: 12px 8px;
	}

	.section {
		margin-bottom: 16px;
	}

	.label {
		margin: 0 0 4px 8px;
		font-size: 11px;
		font-weight: 700;
		text-transform: uppercase;
		letter-spacing: 0.03em;
		color: var(--text-faint);
	}

	.channel {
		width: 100%;
		display: flex;
		align-items: center;
		gap: 8px;
		padding: 6px 8px;
		border-radius: 6px;
		color: var(--text-muted);
		font-size: 14px;
		font-weight: 500;
	}

	.channel:hover {
		background: var(--bg-hover);
		color: var(--text-primary);
	}

	.channel.active {
		background: var(--bg-active);
		color: var(--text-primary);
	}

	.hash,
	.mic {
		color: var(--text-faint);
		font-weight: 700;
		width: 16px;
		text-align: center;
	}

	.user-panel {
		height: 56px;
		flex-shrink: 0;
		display: flex;
		align-items: center;
		gap: 8px;
		padding: 0 8px;
		background: #131318;
	}

	.avatar {
		width: 32px;
		height: 32px;
		border-radius: 50%;
		background: var(--accent);
		display: flex;
		align-items: center;
		justify-content: center;
		font-weight: 700;
		font-size: 13px;
		flex-shrink: 0;
	}

	.identity {
		flex: 1;
		min-width: 0;
	}

	.username {
		margin: 0;
		font-size: 13px;
		font-weight: 600;
		white-space: nowrap;
		overflow: hidden;
		text-overflow: ellipsis;
	}

	.status {
		margin: 0;
		font-size: 11px;
		color: var(--online);
	}

	.settings {
		font-size: 16px;
		color: var(--text-muted);
		padding: 6px;
		border-radius: 6px;
	}

	.settings:hover {
		background: var(--bg-hover);
		color: var(--text-primary);
	}
</style>

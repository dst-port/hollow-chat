<script lang="ts">
	import type { ServerEntry } from "$lib/data/mock";

	let { server, activeChannelId, onSelectChannel, username, onLogout }: {
		server: ServerEntry;
		activeChannelId: string;
		onSelectChannel: (id: string) => void;
		username: string;
		onLogout: () => void;
	} = $props();

	let search = $state("");
	let textCollapsed = $state(false);
	let voiceCollapsed = $state(false);
	let muted = $state(false);
	let deafened = $state(false);

	const textChannels = $derived(
		server.channels.filter(
			(c) => c.type === "text" && c.name.toLowerCase().includes(search.toLowerCase())
		)
	);
	const voiceChannels = $derived(
		server.channels.filter(
			(c) => c.type === "voice" && c.name.toLowerCase().includes(search.toLowerCase())
		)
	);
</script>

<aside class="sidebar">
	<button class="header">
		<span>{server.name}</span>
		<span class="chevron">⌄</span>
	</button>

	<div class="search-bar">
		<input type="text" placeholder="Search channels" bind:value={search} />
	</div>

	<div class="channels">
		<div class="section">
			<button class="label" onclick={() => (textCollapsed = !textCollapsed)}>
				<span class="caret" class:collapsed={textCollapsed}>⌄</span>
				Text Channels
			</button>
			{#if !textCollapsed}
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
			{/if}
		</div>

		<div class="section">
			<button class="label" onclick={() => (voiceCollapsed = !voiceCollapsed)}>
				<span class="caret" class:collapsed={voiceCollapsed}>⌄</span>
				Voice Channels
			</button>
			{#if !voiceCollapsed}
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
			{/if}
		</div>
	</div>

	<div class="user-panel">
		<div class="avatar">{username.slice(0, 2).toUpperCase()}</div>
		<div class="identity">
			<p class="username">{username}</p>
			<p class="status">online</p>
		</div>
		<div class="controls">
			<button
				class="icon-button"
				class:muted-active={muted}
				title={muted ? "Unmute" : "Mute"}
				onclick={() => (muted = !muted)}
			>
				{muted ? "🔇" : "🎙"}
			</button>
			<button
				class="icon-button"
				class:muted-active={deafened}
				title={deafened ? "Undeafen" : "Deafen"}
				onclick={() => (deafened = !deafened)}
			>
				{deafened ? "🔕" : "🔊"}
			</button>
			<button class="icon-button" title="Log out" onclick={onLogout}>⏻</button>
		</div>
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

	.search-bar {
		padding: 8px;
		flex-shrink: 0;
	}

	.search-bar input {
		width: 100%;
		background: var(--bg-main);
		border: none;
		border-radius: 6px;
		padding: 6px 8px;
		font-size: 12px;
		color: var(--text-primary);
	}

	.search-bar input::placeholder {
		color: var(--text-faint);
	}

	.channels {
		flex: 1;
		overflow-y: auto;
		padding: 4px 8px;
	}

	.section {
		margin-bottom: 16px;
	}

	.label {
		width: 100%;
		display: flex;
		align-items: center;
		gap: 4px;
		margin: 0 0 4px;
		padding: 4px 8px;
		border-radius: 4px;
		font-size: 11px;
		font-weight: 700;
		text-transform: uppercase;
		letter-spacing: 0.03em;
		color: var(--text-faint);
	}

	.label:hover {
		color: var(--text-muted);
	}

	.caret {
		display: inline-block;
		font-size: 10px;
		transition: transform 0.15s ease;
	}

	.caret.collapsed {
		transform: rotate(-90deg);
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
		background: #101015;
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

	.controls {
		display: flex;
		gap: 2px;
	}

	.icon-button {
		font-size: 14px;
		padding: 6px;
		border-radius: 6px;
		color: var(--text-muted);
	}

	.icon-button:hover {
		background: var(--bg-hover);
		color: var(--text-primary);
	}

	.icon-button.muted-active {
		color: var(--danger);
	}
</style>

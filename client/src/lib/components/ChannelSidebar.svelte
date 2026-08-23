<script lang="ts">
	import ChevronDown from "@lucide/svelte/icons/chevron-down";
	import Hash from "@lucide/svelte/icons/hash";
	import Volume2 from "@lucide/svelte/icons/volume-2";
	import Search from "@lucide/svelte/icons/search";
	import Mic from "@lucide/svelte/icons/mic";
	import MicOff from "@lucide/svelte/icons/mic-off";
	import Headphones from "@lucide/svelte/icons/headphones";
	import HeadphoneOff from "@lucide/svelte/icons/headphone-off";
	import LogOut from "@lucide/svelte/icons/log-out";
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
		<ChevronDown size={16} strokeWidth={2.5} />
	</button>

	<div class="search-bar">
		<Search size={13} strokeWidth={2.5} />
		<input type="text" placeholder="Search channels" bind:value={search} />
	</div>

	<div class="channels">
		<div class="section">
			<button class="label" onclick={() => (textCollapsed = !textCollapsed)}>
				<ChevronDown class={`caret ${textCollapsed ? "collapsed" : ""}`} size={12} strokeWidth={3} />
				Text Channels
			</button>
			{#if !textCollapsed}
				{#each textChannels as channel (channel.id)}
					<button
						class="channel"
						class:active={channel.id === activeChannelId}
						onclick={() => onSelectChannel(channel.id)}
					>
						<Hash size={16} strokeWidth={2} class="channel-icon" />
						<span class="name">{channel.name}</span>
					</button>
				{/each}
			{/if}
		</div>

		<div class="section">
			<button class="label" onclick={() => (voiceCollapsed = !voiceCollapsed)}>
				<ChevronDown class={`caret ${voiceCollapsed ? "collapsed" : ""}`} size={12} strokeWidth={3} />
				Voice Channels
			</button>
			{#if !voiceCollapsed}
				{#each voiceChannels as channel (channel.id)}
					<button
						class="channel"
						class:active={channel.id === activeChannelId}
						onclick={() => onSelectChannel(channel.id)}
					>
						<Volume2 size={16} strokeWidth={2} class="channel-icon" />
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
				{#if muted}<MicOff size={16} strokeWidth={2} />{:else}<Mic size={16} strokeWidth={2} />{/if}
			</button>
			<button
				class="icon-button"
				class:muted-active={deafened}
				title={deafened ? "Undeafen" : "Deafen"}
				onclick={() => (deafened = !deafened)}
			>
				{#if deafened}<HeadphoneOff size={16} strokeWidth={2} />{:else}<Headphones size={16} strokeWidth={2} />{/if}
			</button>
			<button class="icon-button" title="Log out" onclick={onLogout}>
				<LogOut size={16} strokeWidth={2} />
			</button>
		</div>
	</div>
</aside>

<style>
	.sidebar {
		width: 240px;
		flex-shrink: 0;
		background: var(--sidebar);
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
		font-family: var(--font-display);
		font-weight: 700;
		font-size: 13px;
		letter-spacing: 0.01em;
		border-bottom: 1px solid var(--hairline);
		color: var(--ink-dim);
	}

	.header:hover {
		background: var(--hover);
		color: var(--ink);
	}

	.header {
		transition: background-color 0.15s ease, color 0.15s ease;
	}

	.search-bar {
		display: flex;
		align-items: center;
		gap: 6px;
		margin: 8px;
		padding: 0 8px;
		background: var(--panel);
		border-radius: 6px;
		color: var(--ink-faint);
		flex-shrink: 0;
	}

	.search-bar input {
		flex: 1;
		background: none;
		border: none;
		padding: 6px 0;
		font-size: 12px;
		color: var(--ink);
	}

	.search-bar input::placeholder {
		color: var(--ink-faint);
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
		font-family: var(--font-mono);
		font-size: 11px;
		font-weight: 600;
		text-transform: uppercase;
		letter-spacing: 0.04em;
		color: var(--ink-faint);
	}

	.label:hover {
		color: var(--ink-dim);
	}

	.label :global(.caret) {
		transition: transform 0.15s ease;
	}

	.label :global(.caret.collapsed) {
		transform: rotate(-90deg);
	}

	.channel {
		transition: background-color 0.15s ease, color 0.15s ease;
		width: 100%;
		display: flex;
		align-items: center;
		gap: 8px;
		padding: 6px 8px;
		border-radius: 6px;
		color: var(--ink-dim);
		font-family: var(--font-mono);
		font-size: 13px;
		font-weight: 500;
	}

	.channel:hover {
		background: var(--hover);
		color: var(--ink);
	}

	.channel.active {
		background: var(--active);
		color: var(--ink);
	}

	.channel :global(.channel-icon) {
		color: var(--ink-faint);
		flex-shrink: 0;
	}

	.channel.active :global(.channel-icon),
	.channel:hover :global(.channel-icon) {
		color: var(--ink);
	}

	.user-panel {
		height: 56px;
		flex-shrink: 0;
		display: flex;
		align-items: center;
		gap: 8px;
		padding: 0 8px;
		background: var(--void);
	}

	.avatar {
		width: 32px;
		height: 32px;
		border-radius: 50%;
		background: var(--accent-fill);
		color: var(--accent-fill-ink);
		display: flex;
		align-items: center;
		justify-content: center;
		font-family: var(--font-display);
		font-weight: 700;
		font-size: 12px;
		flex-shrink: 0;
	}

	.identity {
		flex: 1;
		min-width: 0;
	}

	.username {
		margin: 0;
		font-family: var(--font-mono);
		font-size: 13px;
		font-weight: 600;
		white-space: nowrap;
		overflow: hidden;
		text-overflow: ellipsis;
		color: var(--ink);
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
		padding: 6px;
		border-radius: 6px;
		color: var(--ink-dim);
		display: flex;
		transition: background-color 0.15s ease, color 0.15s ease;
	}

	.icon-button:hover {
		background: var(--hover);
		color: var(--ink);
	}

	.icon-button.muted-active {
		color: var(--danger);
	}
</style>

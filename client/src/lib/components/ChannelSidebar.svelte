<script lang="ts">
	import { slide } from "svelte/transition";
	import ChevronDown from "@lucide/svelte/icons/chevron-down";
	import Hash from "@lucide/svelte/icons/hash";
	import Volume2 from "@lucide/svelte/icons/volume-2";
	import Search from "@lucide/svelte/icons/search";
	import UserBar from "$lib/components/UserBar.svelte";
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
				<div transition:slide={{ duration: 160 }}>
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
				</div>
			{/if}
		</div>

		<div class="section">
			<button class="label" onclick={() => (voiceCollapsed = !voiceCollapsed)}>
				<ChevronDown class={`caret ${voiceCollapsed ? "collapsed" : ""}`} size={12} strokeWidth={3} />
				Voice Channels
			</button>
			{#if !voiceCollapsed}
				<div transition:slide={{ duration: 160 }}>
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
				</div>
			{/if}
		</div>
	</div>

	<UserBar {username} {onLogout} />
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
		transition: background-color 0.15s ease, color 0.15s ease;
	}

	.header:hover {
		background: var(--hover);
		color: var(--ink);
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
</style>

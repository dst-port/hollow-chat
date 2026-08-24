<script lang="ts">
	import { slide } from "svelte/transition";
	import ChevronDown from "@lucide/svelte/icons/chevron-down";
	import Hash from "@lucide/svelte/icons/hash";
	import Volume2 from "@lucide/svelte/icons/volume-2";
	import Search from "@lucide/svelte/icons/search";
	import UserBar from "$lib/components/UserBar.svelte";
	import ServerMenu from "$lib/components/ServerMenu.svelte";
	import InviteModal from "$lib/components/InviteModal.svelte";
	import CreateChannelModal from "$lib/components/CreateChannelModal.svelte";
	import ServerSettingsModal from "$lib/components/ServerSettingsModal.svelte";
	import { toast } from "$lib/stores/toast.svelte";
	import type { ChannelType, ServerEntry } from "$lib/data/mock";

	let { server, activeChannelId, onSelectChannel, onCreateChannel, onLeaveServer, username, onLogout }: {
		server: ServerEntry;
		activeChannelId: string;
		onSelectChannel: (id: string) => void;
		onCreateChannel: (name: string, type: ChannelType) => void;
		onLeaveServer: () => void;
		username: string;
		onLogout: () => void;
	} = $props();

	let search = $state("");
	let collapsedCategories = $state<Record<string, boolean>>({});
	let menuOpen = $state(false);
	let inviteOpen = $state(false);
	let createChannelOpen = $state(false);
	let serverSettingsOpen = $state(false);

	const categories = $derived.by(() => {
		const groups = new Map<string, typeof server.channels>();
		for (const channel of server.channels) {
			if (!channel.name.toLowerCase().includes(search.toLowerCase())) continue;
			const key = channel.category ?? (channel.type === "text" ? "Text Channels" : "Voice Channels");
			if (!groups.has(key)) groups.set(key, []);
			groups.get(key)!.push(channel);
		}
		return Array.from(groups.entries()).map(([name, channels]) => ({ name, channels }));
	});

	function selectChannel(id: string) {
		onSelectChannel(id);
	}

	function handleCreateChannel(name: string, type: ChannelType) {
		onCreateChannel(name, type);
		createChannelOpen = false;
		toast.push(`#${name} created`);
	}
</script>

<aside class="sidebar">
	<div class="header-wrap">
		<button class="header" onclick={() => (menuOpen = !menuOpen)}>
			<span>{server.name}</span>
			<ChevronDown class={menuOpen ? "flipped" : ""} size={16} strokeWidth={2.5} />
		</button>
		{#if menuOpen}
			<ServerMenu
				serverId={server.id}
				onClose={() => (menuOpen = false)}
				onInvite={() => {
					menuOpen = false;
					inviteOpen = true;
				}}
				onCreateChannel={() => {
					menuOpen = false;
					createChannelOpen = true;
				}}
				onSettings={() => {
					menuOpen = false;
					serverSettingsOpen = true;
				}}
				onLeave={() => {
					menuOpen = false;
					onLeaveServer();
				}}
			/>
		{/if}
	</div>

	<div class="search-bar">
		<Search size={13} strokeWidth={2.5} />
		<input type="text" placeholder="Search channels" bind:value={search} />
	</div>

	<div class="channels">
		{#each categories as category (category.name)}
			<div class="section">
				<button
					class="label"
					onclick={() => (collapsedCategories[category.name] = !collapsedCategories[category.name])}
				>
					<ChevronDown
						class={`caret ${collapsedCategories[category.name] ? "collapsed" : ""}`}
						size={12}
						strokeWidth={3}
					/>
					{category.name}
				</button>
				{#if !collapsedCategories[category.name]}
					<div transition:slide={{ duration: 160 }}>
						{#each category.channels as channel (channel.id)}
							<button
								class="channel"
								class:active={channel.id === activeChannelId}
								onclick={() => selectChannel(channel.id)}
							>
								{#if channel.type === "text"}
									<Hash size={16} strokeWidth={2} class="channel-icon" />
								{:else}
									<Volume2 size={16} strokeWidth={2} class="channel-icon" />
								{/if}
								<span class="name" class:unread={channel.unread}>{channel.name}</span>
								{#if channel.unread}<span class="unread-dot"></span>{/if}
							</button>
						{/each}
					</div>
				{/if}
			</div>
		{/each}
	</div>

	<UserBar {username} {onLogout} />
</aside>

{#if inviteOpen}
	<InviteModal serverName={server.name} serverId={server.id} onClose={() => (inviteOpen = false)} />
{/if}

{#if createChannelOpen}
	<CreateChannelModal onClose={() => (createChannelOpen = false)} onCreate={handleCreateChannel} />
{/if}

{#if serverSettingsOpen}
	<ServerSettingsModal server={server} onClose={() => (serverSettingsOpen = false)} onLeave={onLeaveServer} />
{/if}

<style>
	.sidebar {
		width: 240px;
		flex-shrink: 0;
		background: var(--sidebar);
		display: flex;
		flex-direction: column;
		height: 100%;
	}

	.header-wrap {
		position: relative;
		flex-shrink: 0;
	}

	.header {
		width: 100%;
		height: 48px;
		display: flex;
		align-items: center;
		justify-content: space-between;
		padding: 0 16px;
		font-family: var(--font-body);
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

	.header :global(svg) {
		transition: transform 0.15s ease;
	}

	.header :global(svg.flipped) {
		transform: rotate(180deg);
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
		color: var(--ink);
	}

	.label:hover {
		color: var(--ink);
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

	.name.unread {
		color: var(--ink);
		font-weight: 700;
	}

	.unread-dot {
		width: 6px;
		height: 6px;
		border-radius: 50%;
		background: var(--ink);
		margin-left: auto;
	}
</style>

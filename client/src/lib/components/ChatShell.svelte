<script lang="ts">
	import { fade } from "svelte/transition";
	import ServerRail from "$lib/components/ServerRail.svelte";
	import ChannelSidebar from "$lib/components/ChannelSidebar.svelte";
	import ChatView from "$lib/components/ChatView.svelte";
	import MemberList from "$lib/components/MemberList.svelte";
	import HomeView from "$lib/components/HomeView.svelte";
	import { servers, messages, members } from "$lib/data/mock";
	import { session } from "$lib/stores/session.svelte";

	let activeServerId = $state<string | null>(servers[0].id);
	let activeChannelId = $state(servers[0].channels[0].id);
	let showMembers = $state(true);

	const activeServer = $derived(servers.find((s) => s.id === activeServerId) ?? null);
	const activeChannel = $derived(
		activeServer
			? (activeServer.channels.find((c) => c.id === activeChannelId) ?? activeServer.channels[0])
			: null
	);

	function selectServer(id: string) {
		activeServerId = id;
		const server = servers.find((s) => s.id === id)!;
		activeChannelId = server.channels[0].id;
	}

	function selectHome() {
		activeServerId = null;
	}

	function selectChannel(id: string) {
		activeChannelId = id;
	}
</script>

<div class="window-frame app">
	<ServerRail servers={servers} activeId={activeServerId} onSelect={selectServer} onSelectHome={selectHome} />
	{#key activeServerId}
		<div class="content" in:fade={{ duration: 140 }}>
			{#if activeServer && activeChannel}
				<ChannelSidebar
					server={activeServer}
					activeChannelId={activeChannelId}
					onSelectChannel={selectChannel}
					username={session.username ?? ""}
					onLogout={() => session.clear()}
				/>
				<ChatView
					channel={activeChannel}
					messages={messages}
					onToggleMembers={() => (showMembers = !showMembers)}
				/>
				{#if showMembers}
					<MemberList members={members} />
				{/if}
			{:else}
				<HomeView username={session.username ?? ""} onLogout={() => session.clear()} />
			{/if}
		</div>
	{/key}
</div>

<style>
	.app {
		display: flex;
	}

	.content {
		display: flex;
		flex: 1;
		min-width: 0;
		height: 100%;
	}
</style>

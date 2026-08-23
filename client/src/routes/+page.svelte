<script lang="ts">
	import ServerRail from "$lib/components/ServerRail.svelte";
	import ChannelSidebar from "$lib/components/ChannelSidebar.svelte";
	import ChatView from "$lib/components/ChatView.svelte";
	import MemberList from "$lib/components/MemberList.svelte";
	import { servers, messages, members } from "$lib/data/mock";

	let activeServerId = $state(servers[0].id);
	let activeChannelId = $state(servers[0].channels[0].id);
	let showMembers = $state(true);

	const activeServer = $derived(servers.find((s) => s.id === activeServerId)!);
	const activeChannel = $derived(
		activeServer.channels.find((c) => c.id === activeChannelId) ?? activeServer.channels[0]
	);

	function selectServer(id: string) {
		activeServerId = id;
		const server = servers.find((s) => s.id === id)!;
		activeChannelId = server.channels[0].id;
	}

	function selectChannel(id: string) {
		activeChannelId = id;
	}
</script>

<div class="app">
	<ServerRail servers={servers} activeId={activeServerId} onSelect={selectServer} />
	<ChannelSidebar
		server={activeServer}
		activeChannelId={activeChannelId}
		onSelectChannel={selectChannel}
	/>
	<ChatView
		channel={activeChannel}
		messages={messages}
		onToggleMembers={() => (showMembers = !showMembers)}
	/>
	{#if showMembers}
		<MemberList members={members} />
	{/if}
</div>

<style>
	.app {
		display: flex;
		height: 100vh;
		width: 100vw;
		overflow: hidden;
	}
</style>

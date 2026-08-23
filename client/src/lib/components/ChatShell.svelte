<script lang="ts">
	import { fade } from "svelte/transition";
	import ServerRail from "$lib/components/ServerRail.svelte";
	import ChannelSidebar from "$lib/components/ChannelSidebar.svelte";
	import ChatView from "$lib/components/ChatView.svelte";
	import MemberList from "$lib/components/MemberList.svelte";
	import HomeView from "$lib/components/HomeView.svelte";
	import CreateServerModal from "$lib/components/CreateServerModal.svelte";
	import { createServers, createMessages, members, type ChannelType } from "$lib/data/mock";
	import { session } from "$lib/stores/session.svelte";
	import { toast } from "$lib/stores/toast.svelte";

	const initialServers = createServers();

	let serverList = $state(initialServers);
	let messageList = $state(createMessages());
	let activeServerId = $state<string | null>(initialServers[0].id);
	let activeChannelId = $state(initialServers[0].channels[0].id);
	let showMembers = $state(true);
	let createServerOpen = $state(false);

	const activeServer = $derived(serverList.find((s) => s.id === activeServerId) ?? null);
	const activeChannel = $derived(
		activeServer
			? (activeServer.channels.find((c) => c.id === activeChannelId) ?? activeServer.channels[0])
			: null
	);

	function selectServer(id: string) {
		activeServerId = id;
		const server = serverList.find((s) => s.id === id)!;
		activeChannelId = server.channels[0].id;
		server.unread = 0;
	}

	function selectHome() {
		activeServerId = null;
	}

	function selectChannel(id: string) {
		activeChannelId = id;
		if (activeServer) {
			const channel = activeServer.channels.find((c) => c.id === id);
			if (channel) channel.unread = false;
		}
	}

	function createServer(name: string) {
		const id = name.toLowerCase().replace(/\s+/g, "-") + "-" + Date.now().toString().slice(-4);
		const entry = {
			id,
			name,
			initials: name.slice(0, 2).toUpperCase(),
			channels: [
				{ id: "general", name: "general", type: "text" as ChannelType },
				{ id: "voice", name: "General Voice", type: "voice" as ChannelType }
			]
		};
		serverList.push(entry);
		createServerOpen = false;
		selectServer(id);
		toast.push(`${name} created`);
	}

	function createChannel(name: string, type: ChannelType) {
		if (!activeServer) return;
		const id = name.toLowerCase().replace(/\s+/g, "-") + "-" + Date.now().toString().slice(-4);
		activeServer.channels.push({ id, name, type });
	}

	function leaveServer() {
		if (!activeServer) return;
		const name = activeServer.name;
		serverList = serverList.filter((s) => s.id !== activeServer.id);
		activeServerId = serverList[0]?.id ?? null;
		if (activeServerId) {
			activeChannelId = serverList.find((s) => s.id === activeServerId)!.channels[0].id;
		}
		toast.push(`Left ${name}`);
	}
</script>

<div class="window-frame app">
	<ServerRail
		servers={serverList}
		activeId={activeServerId}
		onSelect={selectServer}
		onSelectHome={selectHome}
		onAddServer={() => (createServerOpen = true)}
	/>
	{#key activeServerId}
		<div class="content" in:fade={{ duration: 140 }}>
			{#if activeServer && activeChannel}
				<ChannelSidebar
					server={activeServer}
					activeChannelId={activeChannelId}
					onSelectChannel={selectChannel}
					onCreateChannel={createChannel}
					onLeaveServer={leaveServer}
					username={session.username ?? ""}
					onLogout={() => session.clear()}
				/>
				<ChatView
					channel={activeChannel}
					bind:messages={messageList}
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

{#if createServerOpen}
	<CreateServerModal onClose={() => (createServerOpen = false)} onCreate={createServer} />
{/if}

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

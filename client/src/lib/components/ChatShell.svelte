<script lang="ts">
	import { fade } from "svelte/transition";
	import ServerRail from "$lib/components/ServerRail.svelte";
	import ChannelSidebar from "$lib/components/ChannelSidebar.svelte";
	import CallBar from "$lib/components/CallBar.svelte";
	import UserBar from "$lib/components/UserBar.svelte";
	import ChatView from "$lib/components/ChatView.svelte";
	import CallStage from "$lib/components/CallStage.svelte";
	import MemberList from "$lib/components/MemberList.svelte";
	import HomeView from "$lib/components/HomeView.svelte";
	import CreateServerModal from "$lib/components/CreateServerModal.svelte";
	import { type ChannelType, type Member, type ServerEntry } from "$lib/data/mock";
	import { session } from "$lib/stores/session.svelte";
	import { toast } from "$lib/stores/toast.svelte";
	import { pendingDm } from "$lib/stores/pendingDm.svelte";
	import { initRichPresenceBridge } from "$lib/stores/richPresence.svelte";
	import { initGatewayBridge } from "$lib/stores/gateway.svelte";
	import { colorForName } from "$lib/utils/color";
	import { call } from "$lib/webrtc/call.svelte";
	import * as api from "$lib/api/client";

	initRichPresenceBridge();

	$effect(() => {
		if (session.token) initGatewayBridge(session.token);
	});

	function toServerEntry(server: api.ApiServer): ServerEntry {
		return {
			id: server.id,
			name: server.name,
			initials: server.name.slice(0, 2).toUpperCase(),
			iconUrl: server.icon_url,
			ownerId: server.owner_id,
			channels: server.channels.map((c) => ({
				id: c.id,
				name: c.name,
				type: c.type,
				category: c.category ?? undefined,
				slowmodeSeconds: c.slowmode_seconds
			}))
		};
	}

	function defaultChannelId(server: ServerEntry) {
		return (
			server.channels.find((c) => c.name === "general") ??
			server.channels.find((c) => c.type === "text") ??
			server.channels[0]
		)?.id;
	}

	let serverList = $state<ServerEntry[]>([]);
	let loaded = $state(false);
	let activeServerId = $state<string | null>(null);
	let activeChannelId = $state<string | null>(null);
	let showMembers = $state(true);
	let createServerOpen = $state(false);
	let memberList = $state<Member[]>([]);

	$effect(() => {
		const token = session.token;
		if (!token) return;
		api
			.listServers(token)
			.then((servers) => {
				serverList = servers.map(toServerEntry);
				loaded = true;
				if (serverList.length > 0) {
					activeServerId = serverList[0].id;
					activeChannelId = defaultChannelId(serverList[0]) ?? null;
				}
			})
			.catch(() => {
				loaded = true;
				toast.push("Couldn't load servers");
			});
	});

	$effect(() => {
		const token = session.token;
		const serverId = activeServerId;
		if (!token || !serverId) {
			memberList = [];
			return;
		}
		api
			.listMembers(token, serverId)
			.then((rows) => {
				memberList = rows.map((m) => ({
					id: m.id,
					name: m.username,
					color: colorForName(m.username)
				}));
			})
			.catch(() => {
				memberList = [];
			});
	});

	const activeServer = $derived(serverList.find((s) => s.id === activeServerId) ?? null);
	const activeChannel = $derived(
		activeServer
			? (activeServer.channels.find((c) => c.id === activeChannelId) ?? activeServer.channels[0] ?? null)
			: null
	);

	function selectServer(id: string) {
		activeServerId = id;
		const server = serverList.find((s) => s.id === id)!;
		activeChannelId = defaultChannelId(server) ?? null;
		server.unread = 0;
	}

	function selectHome() {
		activeServerId = null;
	}

	function messageUser(username: string) {
		pendingDm.request(username);
		activeServerId = null;
	}

	function selectChannel(id: string) {
		activeChannelId = id;
		if (activeServer) {
			const channel = activeServer.channels.find((c) => c.id === id);
			if (channel) channel.unread = false;
		}
	}

	async function joinVoiceChannel() {
		const token = session.token;
		if (!token || !activeServer || !activeChannel) return;
		try {
			await call.join(token, activeChannel.id, `${activeServer.name} / ${activeChannel.name}`);
		} catch {
			toast.push("Couldn't join voice — check microphone permissions");
		}
	}

	function createServer(name: string) {
		const token = session.token;
		if (!token) return;
		api
			.createServer(token, name)
			.then((server) => {
				const entry = toServerEntry(server);
				serverList.push(entry);
				createServerOpen = false;
				selectServer(entry.id);
				toast.push(`${name} created`);
			})
			.catch(() => toast.push("Couldn't create server"));
	}

	function joinServer(server: api.ApiServer) {
		const entry = toServerEntry(server);
		if (!serverList.some((s) => s.id === entry.id)) serverList.push(entry);
		createServerOpen = false;
		selectServer(entry.id);
		toast.push(`Joined ${entry.name}`);
	}

	function createChannel(name: string, type: ChannelType) {
		const token = session.token;
		if (!token || !activeServer) return;
		api
			.createChannel(token, activeServer.id, name, type)
			.then((channel) => {
				activeServer.channels.push({
					id: channel.id,
					name: channel.name,
					type: channel.type,
					category: channel.category ?? undefined,
					slowmodeSeconds: channel.slowmode_seconds
				});
			})
			.catch(() => toast.push("Couldn't create channel"));
	}

	function leaveServer() {
		const token = session.token;
		if (!token || !activeServer) return;
		const name = activeServer.name;
		const id = activeServer.id;
		api
			.leaveServer(token, id)
			.then(() => {
				serverList = serverList.filter((s) => s.id !== id);
				activeServerId = serverList[0]?.id ?? null;
				activeChannelId = activeServerId ? (defaultChannelId(serverList[0]) ?? null) : null;
				toast.push(`Left ${name}`);
			})
			.catch(() => toast.push("Couldn't leave server"));
	}
</script>

<div class="window-frame app">
	<div class="left-column">
		<div class="upper">
			<ServerRail
				servers={serverList}
				activeId={activeServerId}
				onSelect={selectServer}
				onSelectHome={selectHome}
				onAddServer={() => (createServerOpen = true)}
			/>
			{#if activeServer && activeChannel}
				<ChannelSidebar
					server={activeServer}
					activeChannelId={activeChannelId ?? ""}
					onSelectChannel={selectChannel}
					onCreateChannel={createChannel}
					onLeaveServer={leaveServer}
					username={session.username ?? ""}
				/>
			{/if}
		</div>
		{#if activeServer && activeChannel}
			<div class="bottom-panel">
				<CallBar />
				<UserBar username={session.username ?? ""} onLogout={() => session.clear()} />
			</div>
		{/if}
	</div>
	{#key activeServerId}
		<div class="content" in:fade={{ duration: 140 }}>
			{#if activeServer && activeChannel && activeChannel.type === "voice"}
				<CallStage server={activeServer} channel={activeChannel} onJoin={joinVoiceChannel} />
				{#if showMembers}
					<MemberList members={memberList} serverName={activeServer.name} onMessage={messageUser} />
				{/if}
			{:else if activeServer && activeChannel}
				<ChatView
					channel={activeChannel}
					serverId={activeServer.id}
					onToggleMembers={() => (showMembers = !showMembers)}
				/>
				{#if showMembers}
					<MemberList members={memberList} serverName={activeServer.name} onMessage={messageUser} />
				{/if}
			{:else}
				<HomeView username={session.username ?? ""} onLogout={() => session.clear()} />
			{/if}
		</div>
	{/key}
</div>

{#if createServerOpen}
	<CreateServerModal onClose={() => (createServerOpen = false)} onCreate={createServer} onJoin={joinServer} />
{/if}

<style>
	.app {
		display: flex;
		height: 100%;
		align-items: stretch;
	}

	.left-column {
		display: flex;
		flex-direction: column;
		align-self: stretch;
		width: 312px;
		flex-shrink: 0;
		min-height: 0;
	}

	.upper {
		display: flex;
		flex: 1;
		min-height: 0;
	}

	.bottom-panel {
		margin: 8px;
		background: var(--sidebar);
		border-radius: 10px;
	}

	.bottom-panel:has(:global(.call-bar)) :global(.user-panel) {
		border-top: 1px solid var(--hover);
	}

	.content {
		display: flex;
		flex: 1;
		min-width: 0;
		min-height: 0;
		align-self: stretch;
	}
</style>

<script lang="ts">
	import { fade } from "svelte/transition";
	import UserPlus from "@lucide/svelte/icons/user-plus";
	import Users from "@lucide/svelte/icons/users";
	import MessageCircle from "@lucide/svelte/icons/message-circle";
	import Search from "@lucide/svelte/icons/search";
	import ChevronRight from "@lucide/svelte/icons/chevron-right";
	import ShieldOff from "@lucide/svelte/icons/shield-off";
	import Check from "@lucide/svelte/icons/check";
	import X from "@lucide/svelte/icons/x";
	import UserBar from "$lib/components/UserBar.svelte";
	import CallBar from "$lib/components/CallBar.svelte";
	import ChatView from "$lib/components/ChatView.svelte";
	import type { Member, Channel } from "$lib/data/mock";
	import { toast } from "$lib/stores/toast.svelte";
	import { session } from "$lib/stores/session.svelte";
	import { pendingDm } from "$lib/stores/pendingDm.svelte";
	import { colorForName } from "$lib/utils/color";
	import * as api from "$lib/api/client";
	import { presenceStore } from "$lib/stores/gateway.svelte";

	let { username, onLogout }: {
		username: string;
		onLogout: () => void;
	} = $props();

	let rawFriends = $state<api.ApiFriend[]>([]);
	let requests = $state<api.ApiFriendRequest[]>([]);
	let dmChannels = $state<api.ApiDmChannel[]>([]);
	let activeDmId = $state<string | null>(null);

	function toMember(f: api.ApiFriend): Member {
		const live = presenceStore.forUser(f.id);
		const status = live?.presence ?? f.presence;
		const statusText = live ? live.status_text : f.status_text;
		const activityLabels: string[] = [];
		if (live?.activity_application) activityLabels.push(`Playing ${live.activity_application}`);
		if (live?.media_details) activityLabels.push(live.media_details);
		const activityLabel = activityLabels.length > 0 ? activityLabels.join(" · ") : statusText;
		return {
			id: f.id,
			name: f.username,
			color: colorForName(f.username),
			status: status as Member["status"],
			activity: activityLabel ?? undefined
		};
	}

	const friends = $derived(rawFriends.map(toMember));

	function refreshFriends() {
		const token = session.token;
		if (!token) return;
		api
			.listFriends(token)
			.then((rows) => (rawFriends = rows))
			.catch(() => {});
	}

	function refreshRequests() {
		const token = session.token;
		if (!token) return;
		api
			.listFriendRequests(token)
			.then((rows) => (requests = rows))
			.catch(() => {});
	}

	function refreshDms() {
		const token = session.token;
		if (!token) return;
		api
			.listDms(token)
			.then((rows) => (dmChannels = rows))
			.catch(() => {});
	}

	$effect(() => {
		if (!session.token) return;
		refreshFriends();
		refreshRequests();
		refreshDms();
	});

	const activeDm = $derived(dmChannels.find((d) => d.id === activeDmId) ?? null);
	const activeDmChannel = $derived<Channel | null>(
		activeDm ? { id: activeDm.id, name: activeDm.peer_username, type: "text" } : null
	);

	const incomingRequests = $derived(requests.filter((r) => r.direction === "incoming"));
	const outgoingRequests = $derived(requests.filter((r) => r.direction === "outgoing"));

	type Tab = "online" | "all" | "pending" | "add";
	let tab = $state<Tab>("online");
	let addFriendDraft = $state("");

	let titleEl = $state<HTMLElement | undefined>();
	let formEl = $state<HTMLElement | undefined>();
	let mascotTop = $state(0);
	let mascotHeight = $state(68);

	$effect(() => {
		if (tab !== "add" || !titleEl || !formEl) return;

		function measure() {
			if (!titleEl || !formEl) return;
			mascotTop = titleEl.offsetTop;
			mascotHeight = Math.max(40, formEl.offsetTop - titleEl.offsetTop);
		}

		measure();
		window.addEventListener("resize", measure);
		return () => window.removeEventListener("resize", measure);
	});

	const onlineFriends = $derived(friends.filter((m) => m.status !== "offline"));
	const visibleFriends = $derived(tab === "online" ? onlineFriends : friends);

	function openDmWith(targetUsername: string) {
		const token = session.token;
		if (!token) return;
		api
			.openDm(token, targetUsername)
			.then((dm) => {
				if (!dmChannels.some((d) => d.id === dm.id)) dmChannels = [dm, ...dmChannels];
				activeDmId = dm.id;
			})
			.catch((err: api.ApiError) => {
				if (err.status === 401) {
					toast.push("You can only message friends — send a friend request first");
				} else {
					toast.push("Couldn't open conversation");
				}
			});
	}

	function messageFriend(friend: Member) {
		openDmWith(friend.name);
	}

	$effect(() => {
		const target = pendingDm.username;
		if (!target || !session.token) return;
		pendingDm.consume();
		openDmWith(target);
	});

	function selectDm(id: string) {
		activeDmId = id;
	}

	function backToFriends() {
		activeDmId = null;
	}

	function sendRequest(event: SubmitEvent) {
		event.preventDefault();
		const username = addFriendDraft.trim();
		const token = session.token;
		if (!username || !token) return;
		addFriendDraft = "";
		api
			.sendFriendRequest(token, username)
			.then((res) => {
				if (res.result === "accepted") {
					toast.push(`You're now friends with ${username}`);
					refreshFriends();
					refreshRequests();
				} else {
					toast.push("Friend request sent");
					refreshRequests();
				}
			})
			.catch((err: api.ApiError) => {
				if (err.status === 404) toast.push("No HollowChat user with that username");
				else if (err.status === 409) toast.push("Already friends or request already sent");
				else toast.push("Couldn't send friend request");
			});
	}

	function acceptRequest(request: api.ApiFriendRequest) {
		const token = session.token;
		if (!token) return;
		api
			.acceptFriendRequest(token, request.id)
			.then(() => {
				toast.push(`You're now friends with ${request.username}`);
				refreshFriends();
				refreshRequests();
			})
			.catch(() => toast.push("Couldn't accept request"));
	}

	function declineRequest(request: api.ApiFriendRequest) {
		const token = session.token;
		if (!token) return;
		api
			.declineFriendRequest(token, request.id)
			.then(() => refreshRequests())
			.catch(() => toast.push("Couldn't remove request"));
	}

	function noDiscovery() {
		toast.push("HollowChat has no public server directory, by design");
	}

	function hideBrokenImage(event: Event) {
		(event.currentTarget as HTMLElement).style.display = "none";
	}
</script>

<div class="home">
	<aside class="dm-list">
		<div class="search-bar">
			<input type="text" placeholder="Find or start a conversation" />
		</div>
		<button class="nav-item" class:active={!activeDmId} onclick={backToFriends}>
			<Users size={16} strokeWidth={2} />
			Friends
		</button>
		<p class="label">Direct Messages</p>
		{#if dmChannels.length === 0}
			<p class="dm-empty">No conversations yet. Message a friend to start one.</p>
		{:else}
			{#each dmChannels as dm (dm.id)}
				<button class="nav-item dm-item" class:active={activeDmId === dm.id} onclick={() => selectDm(dm.id)}>
					<div class="dm-avatar" style:background={colorForName(dm.peer_username)}>
						{dm.peer_username.slice(0, 2).toUpperCase()}
					</div>
					{dm.peer_username}
				</button>
			{/each}
		{/if}
		<div class="spacer"></div>
		<div class="bottom-panel">
			<CallBar />
			<UserBar {username} {onLogout} />
		</div>
	</aside>

	{#if activeDmChannel}
		<ChatView channel={activeDmChannel} isDm={true} peerId={activeDm?.peer_id} />
	{:else}
	<div class="main">
		<div class="tabs">
			<span class="tabs-title">
				<Users size={20} strokeWidth={2} />
				Friends
			</span>
			<span class="tabs-divider"></span>
			<button class="tab" class:active={tab === "online"} onclick={() => (tab = "online")}>Online</button>
			<button class="tab" class:active={tab === "all"} onclick={() => (tab = "all")}>All</button>
			<button class="tab" class:active={tab === "pending"} onclick={() => (tab = "pending")}>
				Pending{#if incomingRequests.length > 0} — {incomingRequests.length}{/if}
			</button>
			<button class="tab pill" class:active={tab === "add"} onclick={() => (tab = "add")}>Add Friend</button>
		</div>

		<div class="content">
			{#if tab === "add"}
				<div class="add-friend" in:fade={{ duration: 150 }}>
					<img
						class="mascot"
						src="/mascot/add-friend.png"
						alt=""
						onerror={hideBrokenImage}
						style:top={`${mascotTop}px`}
						style:height={`${mascotHeight}px`}
					/>
					<h2 bind:this={titleEl}>Add Friend</h2>
					<p class="hint">You can add a friend by their HollowChat username.</p>
					<form bind:this={formEl} onsubmit={sendRequest}>
						<input type="text" bind:value={addFriendDraft} placeholder="Enter a username" maxlength="32" />
						<button type="submit" class="send" disabled={!addFriendDraft.trim()}>Send Friend Request</button>
					</form>

					<div class="divider"></div>

					<h3>Other Places to Make Friends</h3>
					<p class="hint">
						Don't have a username on hand? HollowChat has no public server directory —
						ask them to share their username or an invite link directly.
					</p>
					<button class="discovery-row" onclick={noDiscovery}>
						<span class="discovery-icon"><ShieldOff size={18} strokeWidth={2} /></span>
						<span class="discovery-label">No Public Discovery, By Design</span>
						<ChevronRight size={16} strokeWidth={2} />
					</button>
				</div>
			{:else if tab === "pending"}
				{#if incomingRequests.length === 0 && outgoingRequests.length === 0}
					<div class="empty" in:fade={{ duration: 150 }}>
						<UserPlus size={40} strokeWidth={1.5} />
						<h2>No pending requests</h2>
						<p>Sent and received friend requests will show up here.</p>
					</div>
				{:else}
					<div in:fade={{ duration: 150 }}>
						{#if incomingRequests.length > 0}
							<p class="section-label">Incoming — {incomingRequests.length}</p>
							{#each incomingRequests as request (request.id)}
								<div class="request-row">
									<div class="request-identity">
										<div class="avatar small" style:background={colorForName(request.username)}>
											{request.username.slice(0, 2).toUpperCase()}
										</div>
										<span class="request-name">{request.username}</span>
									</div>
									<div class="request-actions">
										<button class="icon-round accept" title="Accept" onclick={() => acceptRequest(request)}>
											<Check size={16} strokeWidth={2.5} />
										</button>
										<button class="icon-round decline" title="Decline" onclick={() => declineRequest(request)}>
											<X size={16} strokeWidth={2.5} />
										</button>
									</div>
								</div>
							{/each}
						{/if}
						{#if outgoingRequests.length > 0}
							<p class="section-label">Outgoing — {outgoingRequests.length}</p>
							{#each outgoingRequests as request (request.id)}
								<div class="request-row">
									<div class="request-identity">
										<div class="avatar small" style:background={colorForName(request.username)}>
											{request.username.slice(0, 2).toUpperCase()}
										</div>
										<span class="request-name">{request.username}</span>
									</div>
									<div class="request-actions">
										<button class="icon-round decline" title="Cancel" onclick={() => declineRequest(request)}>
											<X size={16} strokeWidth={2.5} />
										</button>
									</div>
								</div>
							{/each}
						{/if}
					</div>
				{/if}
			{:else if visibleFriends.length > 0}
				{#key tab}
					<div in:fade={{ duration: 150 }}>
						<p class="section-label">{tab === "online" ? "Online" : "All Friends"} — {visibleFriends.length}</p>
						<div class="friend-grid">
							{#each visibleFriends as friend (friend.id)}
								{@const accent = friend.roles?.[0]?.color ?? friend.color}
								<div class="friend-card">
									<div class="card-banner" style:background={`linear-gradient(135deg, ${accent}, color-mix(in srgb, ${accent} 40%, black))`}></div>
									<button class="card-message" title="Message" onclick={() => messageFriend(friend)}>
										<MessageCircle size={15} strokeWidth={2} />
									</button>
									<div class="card-body">
										<div class="status-avatar card-avatar">
											<div class="avatar" style:background={friend.color}>
												{friend.name.slice(0, 2).toUpperCase()}
											</div>
											{#if friend.status}<span class="status-dot on-panel {friend.status}"></span>{/if}
										</div>
										<p class="name">{friend.name}</p>
										<p class="username">{friend.name.toLowerCase()}</p>
										{#if friend.activity ?? friend.status}
											<p class="activity">{friend.activity ?? friend.status}</p>
										{/if}
									</div>
								</div>
							{/each}
						</div>
					</div>
				{/key}
			{:else if tab === "all"}
				<div class="empty" in:fade={{ duration: 150 }}>
					<Users size={40} strokeWidth={1.5} />
					<h2>No friends yet</h2>
					<p>Add someone by their HollowChat username to get started.</p>
				</div>
			{:else}
				<div class="empty" in:fade={{ duration: 150 }}>
					<Users size={40} strokeWidth={1.5} />
					<h2>No one's around</h2>
					<p>None of your friends are online right now.</p>
				</div>
			{/if}
		</div>
	</div>

	<aside class="active-now">
		<p class="label">Active Now</p>
		<div class="active-empty">
			<p class="active-title">It's quiet for now</p>
			<p class="active-hint">
				When a friend starts talking in a voice channel, we'll show it here.
			</p>
		</div>
	</aside>
	{/if}
</div>

<style>
	.home {
		flex: 1;
		display: flex;
		height: 100%;
		min-width: 0;
	}

	.dm-list {
		width: 280px;
		flex-shrink: 0;
		background: var(--sidebar);
		padding-top: 12px;
		display: flex;
		flex-direction: column;
	}

	.dm-list .search-bar {
		margin-left: 8px;
		margin-right: 8px;
	}

	.spacer {
		flex: 1;
	}

	.bottom-panel {
		margin: 8px;
		background: var(--panel);
		border-radius: 10px;
	}

	.bottom-panel:has(:global(.call-bar)) :global(.user-panel) {
		border-top: 1px solid var(--hover);
	}

	.search-bar {
		margin-bottom: 8px;
	}

	.search-bar input {
		width: 100%;
		background: var(--panel);
		border: none;
		border-radius: 6px;
		padding: 8px 10px;
		font-size: 13px;
		color: var(--ink);
	}

	.search-bar input::placeholder {
		color: var(--ink-faint);
	}

	.nav-item {
		width: 100%;
		display: flex;
		align-items: center;
		gap: 10px;
		margin: 0 8px 8px;
		padding: 8px;
		border-radius: 6px;
		font-size: 14px;
		font-weight: 600;
		color: var(--ink-dim);
		transition: background-color 0.15s ease, color 0.15s ease;
	}

	.nav-item:hover {
		background: var(--hover);
		color: var(--ink);
	}

	.nav-item.active {
		background: var(--active);
		color: var(--ink);
	}

	.dm-item {
		font-weight: 500;
		overflow: hidden;
		white-space: nowrap;
		text-overflow: ellipsis;
	}

	.dm-avatar {
		flex-shrink: 0;
		width: 28px;
		height: 28px;
		border-radius: 50%;
		display: flex;
		align-items: center;
		justify-content: center;
		font-family: var(--font-mono);
		font-size: 10px;
		font-weight: 600;
		color: var(--void);
	}

	.label {
		margin: 8px 8px 4px;
		padding: 0 8px;
		font-family: var(--font-mono);
		font-size: 11px;
		font-weight: 600;
		text-transform: uppercase;
		letter-spacing: 0.04em;
		color: var(--ink-faint);
	}

	.dm-empty {
		margin: 0 16px;
		font-size: 12px;
		line-height: 1.5;
		color: var(--ink-faint);
	}

	.main {
		flex: 1;
		display: flex;
		flex-direction: column;
		min-width: 0;
		background: var(--panel);
	}

	.tabs {
		flex-shrink: 0;
		height: 48px;
		display: flex;
		align-items: center;
		gap: 16px;
		padding: 0 16px;
		border-bottom: 1px solid var(--hairline);
	}

	.tabs-title {
		display: flex;
		align-items: center;
		gap: 8px;
		font-weight: 700;
		font-size: 14px;
		color: var(--ink);
	}

	.tabs-divider {
		width: 1px;
		height: 24px;
		background: var(--hairline);
	}

	.tab {
		padding: 6px 10px;
		border-radius: 6px;
		font-size: 14px;
		font-weight: 600;
		color: var(--ink-dim);
		transition: background-color 0.15s ease, color 0.15s ease;
	}

	.tab:hover {
		color: var(--ink);
	}

	.tab.active {
		background: var(--active);
		color: var(--ink);
	}

	.tab.pill {
		background: var(--accent-fill);
		color: var(--accent-fill-ink);
	}

	.tab.pill:hover {
		background: var(--accent-fill);
		opacity: 0.9;
	}

	.tab.pill.active {
		background: var(--accent-fill);
		color: var(--accent-fill-ink);
	}

	.content {
		flex: 1;
		overflow-y: auto;
		padding: 16px 20px;
	}

	.section-label {
		margin: 0 8px 8px;
		font-family: var(--font-mono);
		font-size: 11px;
		font-weight: 600;
		text-transform: uppercase;
		letter-spacing: 0.04em;
		color: var(--ink-faint);
		border-bottom: 1px solid var(--hairline);
		padding-bottom: 8px;
	}

	.request-row {
		display: flex;
		align-items: center;
		justify-content: space-between;
		padding: 8px;
		margin-bottom: 4px;
		border-radius: 6px;
		transition: background-color 0.15s ease;
	}

	.request-row:hover {
		background: var(--hover);
	}

	.request-identity {
		display: flex;
		align-items: center;
		gap: 12px;
	}

	.avatar.small {
		width: 36px;
		height: 36px;
		font-size: 12px;
	}

	.request-name {
		font-size: 14px;
		font-weight: 600;
		color: var(--ink);
	}

	.request-actions {
		display: flex;
		gap: 8px;
	}

	.icon-round {
		width: 32px;
		height: 32px;
		border-radius: 50%;
		background: var(--sidebar);
		display: flex;
		align-items: center;
		justify-content: center;
		color: var(--ink-dim);
		transition: background-color 0.15s ease, color 0.15s ease;
	}

	.icon-round.accept:hover {
		background: var(--online);
		color: var(--void);
	}

	.icon-round.decline:hover {
		background: var(--danger);
		color: white;
	}

	.friend-grid {
		display: grid;
		grid-template-columns: repeat(auto-fill, minmax(280px, 1fr));
		gap: 16px;
	}

	.friend-card {
		position: relative;
		background: var(--sidebar);
		border-radius: 10px;
		overflow: hidden;
		transition: transform 0.15s ease, box-shadow 0.15s ease;
	}

	.friend-card:hover {
		transform: translateY(-2px);
		box-shadow: 0 8px 20px rgba(0, 0, 0, 0.35);
	}

	.card-banner {
		height: 64px;
	}

	.card-message {
		position: absolute;
		top: 12px;
		right: 12px;
		padding: 8px;
		border-radius: 50%;
		background: rgba(0, 0, 0, 0.35);
		color: white;
		display: flex;
		transition: background-color 0.15s ease;
	}

	.card-message:hover {
		background: rgba(0, 0, 0, 0.55);
	}

	.card-body {
		padding: 0 18px 18px;
	}

	.card-avatar {
		margin-top: -30px;
		margin-bottom: 10px;
	}

	.card-avatar .avatar {
		border: 4px solid var(--sidebar);
	}

	.avatar {
		width: 60px;
		height: 60px;
		border-radius: 50%;
		display: flex;
		align-items: center;
		justify-content: center;
		font-family: var(--font-mono);
		font-size: 16px;
		font-weight: 600;
		color: var(--void);
	}

	.name {
		margin: 0;
		font-size: 16px;
		font-weight: 700;
		color: var(--ink);
		white-space: nowrap;
		overflow: hidden;
		text-overflow: ellipsis;
	}

	.username {
		margin: 2px 0 10px;
		font-size: 13px;
		color: var(--ink-faint);
		white-space: nowrap;
		overflow: hidden;
		text-overflow: ellipsis;
	}

	.activity {
		margin: 0;
		font-size: 12px;
		color: var(--ink-dim);
		text-transform: capitalize;
		white-space: nowrap;
		overflow: hidden;
		text-overflow: ellipsis;
	}

	.add-friend {
		position: relative;
	}

	.mascot {
		position: absolute;
		right: 16px;
		width: auto;
		object-fit: contain;
		pointer-events: none;
		z-index: 1;
	}

	.add-friend h2 {
		margin: 0 0 8px;
		font-family: var(--font-display);
		font-size: 24px;
	}

	.add-friend h3 {
		margin: 0 0 8px;
		font-family: var(--font-body);
		font-size: 16px;
		color: var(--ink);
	}

	.add-friend .hint {
		margin: 0 0 20px;
		font-size: 14px;
		color: var(--ink-dim);
		line-height: 1.5;
	}

	.add-friend form {
		position: relative;
	}

	.divider {
		height: 1px;
		background: var(--hairline);
		margin: 32px 0 24px;
	}

	.discovery-row {
		width: 100%;
		max-width: 528px;
		display: flex;
		align-items: center;
		gap: 12px;
		padding: 6px 16px;
		border-radius: 12px;
		background: var(--sidebar);
		color: var(--ink);
		transition: background-color 0.15s ease;
	}

	.discovery-row:hover {
		background: var(--hover);
	}

	.discovery-icon {
		flex-shrink: 0;
		width: 40px;
		height: 40px;
		border-radius: 10px;
		background: var(--active);
		color: var(--ink-dim);
		display: flex;
		align-items: center;
		justify-content: center;
	}

	.discovery-label {
		flex: 1;
		text-align: left;
		font-size: 14px;
		font-weight: 600;
	}

	.add-friend input {
		width: 100%;
		background: var(--sidebar);
		border: 1px solid var(--hairline);
		border-radius: 8px;
		padding: 13px 170px 13px 14px;
		color: var(--ink);
		font-family: var(--font-body);
		font-size: 14px;
	}

	.add-friend input:focus {
		outline: none;
		border-color: var(--ink-dim);
	}

	.add-friend form .send {
		position: absolute;
		top: 4px;
		right: 4px;
		bottom: 4px;
		padding: 0 16px;
		border-radius: 10px;
		background: var(--active);
		color: var(--ink);
		font-weight: 600;
		font-size: 13px;
		transition: background-color 0.15s ease;
	}

	.add-friend form .send:hover:not(:disabled) {
		background: var(--hover);
	}

	.add-friend form .send:disabled {
		background: var(--active);
		color: var(--ink-faint);
		opacity: 0.6;
	}

	.empty {
		height: 100%;
		display: flex;
		flex-direction: column;
		align-items: center;
		justify-content: center;
		gap: 8px;
		color: var(--ink-faint);
		text-align: center;
	}

	.empty h2 {
		margin: 8px 0 0;
		font-family: var(--font-body);
		font-size: 16px;
		color: var(--ink-dim);
	}

	.empty p {
		margin: 0;
		font-size: 13px;
	}

	.active-now {
		width: 360px;
		flex-shrink: 0;
		background: var(--sidebar);
		padding: 16px;
	}

	.active-now .label {
		margin: 0 0 16px;
		padding: 0;
		font-family: var(--font-body);
		font-weight: 700;
		font-size: 16px;
		text-transform: none;
		letter-spacing: normal;
		color: var(--ink);
	}

	.active-empty {
		background: var(--panel);
		border-radius: 8px;
		padding: 16px;
	}

	.active-title {
		margin: 0 0 6px;
		font-size: 14px;
		font-weight: 700;
		color: var(--ink);
	}

	.active-hint {
		margin: 0;
		font-size: 12px;
		line-height: 1.5;
		color: var(--ink-faint);
	}
</style>

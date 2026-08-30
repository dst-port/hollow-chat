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
	import ArrowLeft from "@lucide/svelte/icons/arrow-left";
	import UserBar from "$lib/components/UserBar.svelte";
	import CallBar from "$lib/components/CallBar.svelte";
	import ChatView from "$lib/components/ChatView.svelte";
	import CallStage from "$lib/components/CallStage.svelte";
	import DmProfilePanel from "$lib/components/DmProfilePanel.svelte";
	import FullProfileModal from "$lib/components/FullProfileModal.svelte";
	import CreateGroupDmModal from "$lib/components/CreateGroupDmModal.svelte";
	import GroupDmMembersPanel from "$lib/components/GroupDmMembersPanel.svelte";
	import { call } from "$lib/webrtc/call.svelte";
	import type { Member, Channel } from "$lib/data/mock";
	import { toast } from "$lib/stores/toast.svelte";
	import { session } from "$lib/stores/session.svelte";
	import { pendingDm } from "$lib/stores/pendingDm.svelte";
	import { colorForName } from "$lib/utils/color";
	import { t } from "$lib/i18n/index.svelte";
	import * as api from "$lib/api/client";
	import { presenceStore } from "$lib/stores/gateway.svelte";
	import { viewport } from "$lib/stores/viewport.svelte";
	import { base } from "$app/paths";

	let { username, onLogout }: {
		username: string;
		onLogout: () => void;
	} = $props();

	let rawFriends = $state<api.ApiFriend[]>([]);
	let requests = $state<api.ApiFriendRequest[]>([]);
	let dmChannels = $state<api.ApiDmChannel[]>([]);
	let activeDmId = $state<string | null>(null);
	let mobileDetailOpen = $state(false);
	let groupModalOpen = $state(false);
	let showDmProfile = $state(false);
	let viewingProfile = $state<string | null>(null);

	function toMember(f: api.ApiFriend): Member {
		const live = presenceStore.forUser(f.id);
		const status = live?.presence ?? f.presence;
		const statusText = live ? live.status_text : f.status_text;
		const activityLabels: string[] = [];
		if (live?.activity_application)
			activityLabels.push(t("presence.playing", { app: live.activity_application }));
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

	const HOME_POLL_INTERVAL_MS = 5000;

	$effect(() => {
		if (!session.token) return;
		refreshFriends();
		refreshRequests();
		refreshDms();

		// Incoming friend requests and new DM conversations don't have a
		// gateway push yet (unlike presence), so this is the only way they
		// show up without a manual reload.
		const interval = setInterval(() => {
			refreshFriends();
			refreshRequests();
			refreshDms();
		}, HOME_POLL_INTERVAL_MS);

		return () => clearInterval(interval);
	});

	function dmDisplayName(dm: api.ApiDmChannel): string {
		if (!dm.is_group) return dm.peer_username ?? t("home.dmUnknownPeer");
		return dm.name || dm.members.map((m) => m.username).filter((u) => u !== username).join(", ") || t("home.groupFallback");
	}

	function dmMemberKey(dm: api.ApiDmChannel): string {
		return dm.is_group
			? `g:${dm.name ?? ""}:${dm.members.map((m) => m.username).sort().join(",")}`
			: (dm.peer_username ?? "");
	}

	async function startDmCallFromCallBar() {
		const token = session.token;
		const dm = activeDm;
		if (!token || !dm) return;
		try {
			await call.join(token, dm.id, dmDisplayName(dm));
		} catch {
			toast.push(t("toast.callStartFailed"));
		}
	}

	const activeDm = $derived(dmChannels.find((d) => d.id === activeDmId) ?? null);

	// Kept referentially stable across the background poll above - a new
	// object here on every refresh (even with identical id/name) makes
	// ChatView see it as a different channel prop and reload the
	// conversation from scratch every few seconds.
	let memoDmChannel: Channel | null = null;
	let memoDmKey: string | null = null;
	const activeDmChannel = $derived.by<Channel | null>(() => {
		if (!activeDm) {
			memoDmChannel = null;
			memoDmKey = null;
			return null;
		}
		const key = dmMemberKey(activeDm);
		if (memoDmChannel && memoDmChannel.id === activeDm.id && memoDmKey === key) {
			return memoDmChannel;
		}
		memoDmChannel = {
			id: activeDm.id,
			name: dmDisplayName(activeDm),
			type: "text",
			isGroupDm: activeDm.is_group,
			dmMembers: activeDm.members.filter((m) => m.username !== username)
		};
		memoDmKey = key;
		return memoDmChannel;
	});

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
					toast.push(t("toast.friendsOnlyMessage"));
				} else {
					toast.push(t("toast.openConversationFailed"));
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
		if (viewport.isMobile) mobileDetailOpen = true;
	}

	function backToFriends() {
		activeDmId = null;
		if (viewport.isMobile) mobileDetailOpen = true;
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
					toast.push(t("toast.nowFriendsWith", { username }));
					refreshFriends();
					refreshRequests();
				} else {
					toast.push(t("toast.friendRequestSent"));
					refreshRequests();
				}
			})
			.catch((err: api.ApiError) => {
				if (err.status === 404) toast.push(t("toast.noSuchUser"));
				else if (err.status === 409) toast.push(t("toast.alreadyFriendsOrPending"));
				else toast.push(t("toast.friendRequestFailed"));
			});
	}

	function acceptRequest(request: api.ApiFriendRequest) {
		const token = session.token;
		if (!token) return;
		api
			.acceptFriendRequest(token, request.id)
			.then(() => {
				toast.push(t("toast.nowFriendsWith", { username: request.username }));
				refreshFriends();
				refreshRequests();
			})
			.catch(() => toast.push(t("toast.acceptRequestFailed")));
	}

	function declineRequest(request: api.ApiFriendRequest) {
		const token = session.token;
		if (!token) return;
		api
			.declineFriendRequest(token, request.id)
			.then(() => refreshRequests())
			.catch(() => toast.push(t("toast.removeRequestFailed")));
	}

	function noDiscovery() {
		toast.push(t("toast.noPublicDirectory"));
	}

	function hideBrokenImage(event: Event) {
		(event.currentTarget as HTMLElement).style.display = "none";
	}
</script>

<div class="home">
	<aside class="dm-list" class:mobile-open={!mobileDetailOpen}>
		<div class="search-bar">
			<input type="text" placeholder={t("home.search")} />
		</div>
		<button class="nav-item" class:active={!activeDmId} onclick={backToFriends}>
			<Users size={16} strokeWidth={2} />
			{t("home.friends")}
		</button>
		<div class="label-row">
			<p class="label">{t("nav.directMessages")}</p>
			<button class="new-group" title={t("groupDm.create.title")} onclick={() => (groupModalOpen = true)}>
				<UserPlus size={13} strokeWidth={2.25} />
			</button>
		</div>
		{#if dmChannels.length === 0}
			<p class="dm-empty">{t("home.noConversations")}</p>
		{:else}
			{#each dmChannels as dm (dm.id)}
				<button class="nav-item dm-item" class:active={activeDmId === dm.id} onclick={() => selectDm(dm.id)}>
					{#if dm.is_group}
						<div class="dm-avatar group">
							<Users size={14} strokeWidth={2} />
						</div>
					{:else}
						<div class="dm-avatar" style:background={colorForName(dm.peer_username ?? "")}>
							{(dm.peer_username ?? "?").slice(0, 2).toUpperCase()}
						</div>
					{/if}
					{dmDisplayName(dm)}
				</button>
			{/each}
		{/if}
		<div class="spacer"></div>
		<div class="bottom-panel">
			<CallBar />
			<UserBar {username} {onLogout} />
		</div>
	</aside>

	<div class="detail" class:mobile-open={mobileDetailOpen}>
	{#if viewport.isMobile && mobileDetailOpen}
		<button class="mobile-back" onclick={() => (mobileDetailOpen = false)} aria-label={t("common.back")}>
			<ArrowLeft size={18} strokeWidth={2.25} />
		</button>
	{/if}
	{#if activeDmChannel}
		<div class="dm-main">
			{#if call.roomId === activeDmChannel.id}
				<div class="dm-call">
					<CallStage channel={activeDmChannel} onJoin={startDmCallFromCallBar} />
				</div>
			{/if}
			<ChatView
				channel={activeDmChannel}
				isDm={true}
				peerId={activeDm?.peer_id ?? undefined}
				onToggleMembers={() => (showDmProfile = !showDmProfile)}
			/>
		</div>
		{#if showDmProfile && activeDm}
			<div class="dm-profile-wrap" class:mobile-overlay={viewport.isMobile}>
				{#if activeDm.is_group}
					<GroupDmMembersPanel
						dm={activeDm}
						onChanged={(updated) => (dmChannels = dmChannels.map((d) => (d.id === updated.id ? updated : d)))}
						onLeft={() => {
							dmChannels = dmChannels.filter((d) => d.id !== activeDm!.id);
							activeDmId = null;
							showDmProfile = false;
						}}
					/>
				{:else}
					<DmProfilePanel username={activeDmChannel.name} onViewFullProfile={() => (viewingProfile = activeDmChannel!.name)} />
				{/if}
			</div>
		{/if}
	{:else}
	<div class="main">
		<div class="tabs">
			<span class="tabs-title">
				<Users size={20} strokeWidth={2} />
				{t("home.friends")}
			</span>
			<span class="tabs-divider"></span>
			<button class="tab" class:active={tab === "online"} onclick={() => (tab = "online")}>{t("home.tab.online")}</button>
			<button class="tab" class:active={tab === "all"} onclick={() => (tab = "all")}>{t("home.tab.all")}</button>
			<button class="tab" class:active={tab === "pending"} onclick={() => (tab = "pending")}>
				{t("home.tab.pending")}{#if incomingRequests.length > 0} — {incomingRequests.length}{/if}
			</button>
			<button class="tab pill" class:active={tab === "add"} onclick={() => (tab = "add")}>{t("home.tab.addFriend")}</button>
		</div>

		<div class="content">
			{#if tab === "add"}
				<div class="add-friend" in:fade={{ duration: 150 }}>
					<img
						class="mascot"
						src={`${base}/mascot/add-friend.png`}
						alt=""
						onerror={hideBrokenImage}
						style:top={`${mascotTop}px`}
						style:height={`${mascotHeight}px`}
					/>
					<h2 bind:this={titleEl}>{t("home.tab.addFriend")}</h2>
					<p class="hint">{t("home.addFriend.hint")}</p>
					<form bind:this={formEl} onsubmit={sendRequest}>
						<input type="text" bind:value={addFriendDraft} placeholder={t("home.addFriend.placeholder")} maxlength="32" />
						<button type="submit" class="send" disabled={!addFriendDraft.trim()}>{t("home.addFriend.submit")}</button>
					</form>

					<div class="divider"></div>

					<h3>{t("home.addFriend.otherPlaces")}</h3>
					<p class="hint">
						{t("home.addFriend.otherHint")}
					</p>
					<button class="discovery-row" onclick={noDiscovery}>
						<span class="discovery-icon"><ShieldOff size={18} strokeWidth={2} /></span>
						<span class="discovery-label">{t("home.addFriend.noDiscovery")}</span>
						<ChevronRight size={16} strokeWidth={2} />
					</button>
				</div>
			{:else if tab === "pending"}
				{#if incomingRequests.length === 0 && outgoingRequests.length === 0}
					<div class="empty" in:fade={{ duration: 150 }}>
						<UserPlus size={40} strokeWidth={1.5} />
						<h2>{t("home.pending.emptyTitle")}</h2>
						<p>{t("home.pending.emptyBody")}</p>
					</div>
				{:else}
					<div in:fade={{ duration: 150 }}>
						{#if incomingRequests.length > 0}
							<p class="section-label">{t("home.pending.incoming", { count: incomingRequests.length })}</p>
							{#each incomingRequests as request (request.id)}
								<div class="request-row">
									<div class="request-identity">
										<div class="avatar small" style:background={colorForName(request.username)}>
											{request.username.slice(0, 2).toUpperCase()}
										</div>
										<span class="request-name">{request.username}</span>
									</div>
									<div class="request-actions">
										<button class="icon-round accept" title={t("common.accept")} onclick={() => acceptRequest(request)}>
											<Check size={16} strokeWidth={2.5} />
										</button>
										<button class="icon-round decline" title={t("common.decline")} onclick={() => declineRequest(request)}>
											<X size={16} strokeWidth={2.5} />
										</button>
									</div>
								</div>
							{/each}
						{/if}
						{#if outgoingRequests.length > 0}
							<p class="section-label">{t("home.pending.outgoing", { count: outgoingRequests.length })}</p>
							{#each outgoingRequests as request (request.id)}
								<div class="request-row">
									<div class="request-identity">
										<div class="avatar small" style:background={colorForName(request.username)}>
											{request.username.slice(0, 2).toUpperCase()}
										</div>
										<span class="request-name">{request.username}</span>
									</div>
									<div class="request-actions">
										<button class="icon-round decline" title={t("common.cancel")} onclick={() => declineRequest(request)}>
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
						<p class="section-label">{t("home.friends.sectionCount", { label: tab === "online" ? t("home.friends.sectionOnline") : t("home.friends.sectionAll"), count: visibleFriends.length })}</p>
						<div class="friend-grid">
							{#each visibleFriends as friend (friend.id)}
								{@const accent = friend.roles?.[0]?.color ?? friend.color}
								<div class="friend-card">
									<div class="card-banner" style:background={`linear-gradient(135deg, ${accent}, color-mix(in srgb, ${accent} 40%, black))`}></div>
									<button class="card-message" title={t("home.friends.messageTitle")} onclick={() => messageFriend(friend)}>
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
					<h2>{t("home.friends.noneTitle")}</h2>
					<p>{t("home.friends.noneBody")}</p>
				</div>
			{:else}
				<div class="empty" in:fade={{ duration: 150 }}>
					<Users size={40} strokeWidth={1.5} />
					<h2>{t("home.friends.noOnlineTitle")}</h2>
					<p>{t("home.friends.noOnlineBody")}</p>
				</div>
			{/if}
		</div>
	</div>

	<aside class="active-now">
		<p class="label">{t("home.activeNow")}</p>
		<div class="active-empty">
			<p class="active-title">{t("home.activeQuietTitle")}</p>
			<p class="active-hint">
				{t("home.activeQuietBody")}
			</p>
		</div>
	</aside>
	{/if}
	</div>
</div>

{#if viewingProfile}
	<FullProfileModal
		username={viewingProfile}
		member={null}
		serverName=""
		onClose={() => (viewingProfile = null)}
		onMessage={() => (viewingProfile = null)}
	/>
{/if}

{#if groupModalOpen && session.token}
	<CreateGroupDmModal
		token={session.token}
		friends={rawFriends}
		onClose={() => (groupModalOpen = false)}
		onCreated={(dm) => {
			dmChannels = [dm, ...dmChannels.filter((d) => d.id !== dm.id)];
			activeDmId = dm.id;
			if (viewport.isMobile) mobileDetailOpen = true;
		}}
	/>
{/if}

<style>
	.home {
		flex: 1;
		display: flex;
		height: 100%;
		min-width: 0;
	}

	.dm-main {
		flex: 1;
		display: flex;
		flex-direction: column;
		min-width: 0;
		min-height: 0;
	}

	.dm-profile-wrap {
		display: flex;
		flex-shrink: 0;
		min-height: 0;
	}

	.dm-call {
		flex-shrink: 0;
		height: min(38vh, 300px);
		display: flex;
		background: #000;
		border-bottom: 1px solid var(--hairline);
	}

	.dm-call :global(.stage),
	.dm-call :global(.prejoin) {
		height: 100%;
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

	.label-row {
		display: flex;
		align-items: center;
		justify-content: space-between;
	}

	.label-row .label {
		margin: 8px 0 4px;
	}

	.new-group {
		margin-right: 8px;
		width: 20px;
		height: 20px;
		border-radius: 5px;
		display: flex;
		align-items: center;
		justify-content: center;
		color: var(--ink-faint);
		background: var(--panel);
	}

	.new-group:hover {
		color: var(--ink);
		background: var(--hover);
	}

	.dm-avatar.group {
		background: var(--active);
		color: var(--ink-dim);
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

	.detail {
		display: flex;
		flex: 1;
		min-width: 0;
		min-height: 0;
	}

	.mobile-back {
		display: none;
	}

	@media (max-width: 860px) {
		.home {
			position: relative;
			overflow: hidden;
		}

		.dm-list {
			position: absolute;
			inset: 0;
			z-index: 30;
			width: 100%;
			transform: translateX(-100%);
			transition: transform 0.18s ease;
		}

		.dm-list.mobile-open {
			transform: translateX(0);
		}

		.detail {
			width: 100%;
			position: relative;
		}

		.active-now {
			display: none;
		}

		.dm-profile-wrap.mobile-overlay {
			position: absolute;
			inset: 0;
			z-index: 45;
			width: 100%;
			background: var(--panel);
		}

		.mobile-back {
			display: flex;
			align-items: center;
			justify-content: center;
			position: absolute;
			top: 10px;
			left: 10px;
			z-index: 50;
			width: 32px;
			height: 32px;
			border-radius: 50%;
			background: var(--sidebar);
			color: var(--ink);
			box-shadow: 0 4px 12px rgba(0, 0, 0, 0.35);
		}
	}
</style>

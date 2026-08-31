<script lang="ts">
	import { fade } from "svelte/transition";
	import X from "@lucide/svelte/icons/x";
	import MessageSquare from "@lucide/svelte/icons/message-square";
	import UserPlus from "@lucide/svelte/icons/user-plus";
	import MoreHorizontal from "@lucide/svelte/icons/more-horizontal";
	import CalendarDays from "@lucide/svelte/icons/calendar-days";
	import Users from "@lucide/svelte/icons/users";
	import Globe from "@lucide/svelte/icons/globe";
	import ExternalLink from "@lucide/svelte/icons/external-link";
	import Gamepad2 from "@lucide/svelte/icons/gamepad-2";
	import Pin from "@lucide/svelte/icons/pin";
	import Badges from "$lib/components/Badges.svelte";
	import ActivityCard from "$lib/components/ActivityCard.svelte";
	import BrandIcon from "$lib/components/BrandIcon.svelte";
	import ProfileActionsMenu from "$lib/components/ProfileActionsMenu.svelte";
	import { BRAND_ICONS } from "$lib/data/brandIcons";
	import { extractConnectionHandle } from "$lib/utils/connectionHandle";
	import { session } from "$lib/stores/session.svelte";
	import { profileStore } from "$lib/stores/profile.svelte";
	import { badgeStore } from "$lib/stores/badges.svelte";
	import { toast } from "$lib/stores/toast.svelte";
	import * as api from "$lib/api/client";
	import { isVideoMedia } from "$lib/utils/media";
	import { t, tp } from "$lib/i18n/index.svelte";
	import { nameFontStack } from "$lib/stores/font.svelte";
	import type { Member } from "$lib/data/mock";

	let { username, member, serverName, onClose, onMessage }: {
		username: string;
		member: Member | null;
		serverName: string;
		onClose: () => void;
		onMessage: (username: string) => void;
	} = $props();

	const NOTE_KEY = `hollowchat_note_${member?.id ?? username}`;

	$effect(() => {
		const token = session.token;
		if (!token) return;
		badgeStore.loadForUser(token, username);
		profileStore.load(token, username);
	});

	const profile = $derived(profileStore.forUser(username));
	const badges = $derived(badgeStore.forUser(username));
	const isSelf = $derived(username === session.username);
	const accent = $derived(profile?.accent_color || member?.roles?.[0]?.color || member?.color || "#5865f2");
	// Discord-style profile theme: tint the left column with the two profile
	// colors, kept low-opacity so text stays legible on any theme.
	const themeStart = $derived(profile?.banner_color || accent);
	const themeEnd = $derived(profile?.banner_gradient_end || accent);
	const themeBg = $derived(
		`linear-gradient(180deg, color-mix(in srgb, ${themeStart} 26%, var(--sidebar)), color-mix(in srgb, ${themeEnd} 26%, var(--sidebar)))`
	);
	const displayName = $derived(profile?.display_name || username);
	const avatarIsVideo = $derived(isVideoMedia(profile?.avatar_url));
	const bannerIsVideo = $derived(isVideoMedia(profile?.banner_url));
	const avatarSrc = $derived(profile?.avatar_url ? api.resolveUrl(profile.avatar_url, session.token) : "");
	const bannerSrc = $derived(profile?.banner_url ? api.resolveUrl(profile.banner_url, session.token) : "");
	const bioLines = $derived((profile?.bio ?? "").split("\n").filter((line) => line.trim().length > 0));
	const memberSince = $derived(
		profile?.member_since
			? new Date(profile.member_since).toLocaleDateString(undefined, { month: "long", day: "numeric", year: "numeric" })
			: member?.memberSince
	);

	let mutualFriends = $state<api.ApiFriend[]>([]);
	async function loadMutualFriends() {
		const token = session.token;
		if (!token || isSelf) return;
		try {
			mutualFriends = await api.listMutualFriends(token, username);
		} catch {
			mutualFriends = [];
		}
	}
	$effect(() => {
		if (session.token) loadMutualFriends();
	});

	let connections = $state<api.ApiConnection[]>([]);
	async function loadConnections() {
		const token = session.token;
		if (!token) return;
		try {
			connections = await api.listConnections(token, username);
		} catch {
			connections = [];
		}
	}
	$effect(() => {
		if (session.token) loadConnections();
	});

	function connectionLabel(connection: api.ApiConnection): string {
		return extractConnectionHandle(connection.service, connection.url) ?? connection.label;
	}

	let widgets = $state<api.ApiWidget[]>([]);
	async function loadWidgets() {
		const token = session.token;
		if (!token) return;
		try {
			widgets = await api.listWidgets(token, username);
		} catch {
			widgets = [];
		}
	}
	$effect(() => {
		if (session.token) loadWidgets();
	});

	const WIDGET_KIND_LABELS = $derived<Record<api.WidgetKind, string>>({
		favorite_game: t("profile.edit.widgetKind.favoriteGame"),
		want_to_play: t("profile.edit.widgetKind.wantToPlay"),
		games_i_like: t("profile.edit.widgetKind.gamesILike"),
		games_in_rotation: t("profile.edit.widgetKind.gamesInRotation")
	});

	type BoardTab = "board" | "activity" | "wishlist";
	let boardTab = $state<BoardTab>("board");

	let note = $state(localStorage.getItem(NOTE_KEY) ?? "");
	function saveNote() {
		if (note.trim()) {
			localStorage.setItem(NOTE_KEY, note);
		} else {
			localStorage.removeItem(NOTE_KEY);
		}
	}

	let friendRequested = $state(false);
	async function addFriend() {
		const token = session.token;
		if (!token) return;
		try {
			await api.sendFriendRequest(token, username);
			friendRequested = true;
			toast.push(t("profile.full.friendRequestSentTo", { name: username }));
		} catch (err) {
			toast.push(err instanceof api.ApiError ? err.message : t("toast.friendRequestFailed"));
		}
	}

	function message() {
		onMessage(username);
		onClose();
	}

	let moreButtonEl: HTMLElement | undefined;
	let moreOpen = $state(false);
	let morePosition = $state({ top: 0, left: 0 });

	function toggleMore() {
		if (!moreOpen && moreButtonEl) {
			const rect = moreButtonEl.getBoundingClientRect();
			morePosition = { top: rect.bottom + 4, left: Math.max(8, rect.right - 210) };
		}
		moreOpen = !moreOpen;
	}

	function onKeydown(event: KeyboardEvent) {
		if (event.key === "Escape") onClose();
	}
</script>

<svelte:window onkeydown={onKeydown} />

<div class="overlay" role="presentation" onclick={onClose} transition:fade={{ duration: 140 }}>
<div class="modal" role="dialog" aria-modal="true" aria-label={t("chat.header.profile")} tabindex="-1" onclick={(e) => e.stopPropagation()}>
	<button class="close" onclick={onClose} title={t("common.close")}>
		<X size={20} strokeWidth={2} />
	</button>

	<div class="col-main" style:background={themeBg}>
		<div class="banner" style:background={bannerIsVideo ? "#000" : api.bannerBackground(profile, session.token)}>
			{#if bannerIsVideo}
				<video class="banner-media" src={bannerSrc} autoplay loop muted playsinline></video>
			{/if}
		</div>
		<div class="body">
			<div class="top-row">
				<div
					class="avatar avatar-ring on-panel {profile?.presence ?? 'online'}"
					style:background={avatarSrc && !avatarIsVideo ? undefined : member?.color}
					style:background-image={avatarSrc && !avatarIsVideo ? `url(${avatarSrc})` : undefined}
				>
					{#if avatarSrc && avatarIsVideo}
						<video class="avatar-media" src={avatarSrc} autoplay loop muted playsinline></video>
					{:else if !avatarSrc}{username.slice(0, 2).toUpperCase()}{/if}
				</div>

				{#if !isSelf}
					<div class="actions">
						<button class="action primary" onclick={addFriend} disabled={friendRequested} title={t("profile.full.addFriend")}>
							<UserPlus size={14} strokeWidth={2} />
							{friendRequested ? t("profile.full.requested") : t("profile.full.addFriend")}
						</button>
						<button class="action" onclick={message} title={t("profile.full.message")}>
							<MessageSquare size={14} strokeWidth={2} />
							{t("profile.full.message")}
						</button>
						<button bind:this={moreButtonEl} class="action icon-only" onclick={toggleMore} title={t("profile.full.more")}>
							<MoreHorizontal size={14} strokeWidth={2} />
						</button>
					</div>
				{/if}
			</div>

			<p class="name" style:color={accent} style:font-family={nameFontStack(profile?.name_font)}>{displayName}</p>
			<p class="handle">
				<span>@{username}</span>
				{#if profile?.pronouns}<span>· {profile.pronouns}</span>{/if}
				{#if badges.length > 0}<Badges {badges} />{/if}
			</p>

			{#if profile?.status_text}<p class="status">{profile.status_text}</p>{/if}

			<div class="divider"></div>

			{#if !isSelf}
				<p class="mutual">
					<Users size={12} strokeWidth={2} />
					{tp("profile.full.mutualFriends", mutualFriends.length)}
				</p>
				{#if serverName}
					<p class="mutual">
						<Users size={12} strokeWidth={2} />
						{t("profile.full.mutualServer", { server: serverName })}
					</p>
				{/if}
			{/if}

			{#if bioLines.length > 0}
				<div class="info-block">
					<p class="info-label">{t("profile.full.aboutMe")}</p>
					{#each bioLines as line}<p class="bio-line">{line}</p>{/each}
				</div>
			{/if}

			{#if profile?.activity_application}
				<ActivityCard
					label={t("profile.full.playing")}
					application={profile.activity_application}
					details={profile.activity_details}
					activityState={profile.activity_state}
					image={profile.activity_image}
					smallImage={profile.activity_small_image}
					smallText={profile.activity_small_text}
					startedAt={profile.activity_started_at}
					partySize={profile.activity_party_size}
					partyMax={profile.activity_party_max}
				/>
			{/if}
			{#if profile?.media_details}
				<p class="activity-line">
					{profile.media_details}
					{#if profile.media_application}<br /><strong>{profile.media_application}</strong>{/if}
					{#if profile.media_state}<br />{profile.media_state}{/if}
				</p>
			{/if}

			{#if memberSince}
				<div class="info-block">
					<p class="info-label">{t("profile.full.memberSince")}</p>
					<p class="info-value"><CalendarDays size={13} strokeWidth={2} /> {memberSince}</p>
				</div>
			{/if}

			{#if connections.length > 0}
				<div class="info-block">
					<p class="info-label">{t("profile.full.connections")}</p>
					<div class="connection-list">
						{#each connections as connection (connection.id)}
							<a
								href={connection.url}
								target="_blank"
								rel="noreferrer"
								class="connection-row"
							>
								{#if BRAND_ICONS[connection.service]}
									<BrandIcon service={connection.service} size={13} chip />
								{:else}
									<Globe size={13} strokeWidth={2} />
								{/if}
								<span class="connection-label">{connectionLabel(connection)}</span>
								<ExternalLink size={12} strokeWidth={2} class="connection-arrow" />
							</a>
						{/each}
					</div>
				</div>
			{/if}

			<div class="info-block">
				<p class="info-label">{t("profile.full.noteLabel")}</p>
				<textarea
					class="note"
					placeholder={t("profile.full.notePlaceholder")}
					bind:value={note}
					onblur={saveNote}
				></textarea>
			</div>
		</div>
	</div>

	<div class="col-right">
		<div class="tabs">
			<button class="tab" class:active={boardTab === "board"} onclick={() => (boardTab = "board")}>{t("profile.full.tabBoard")}</button>
			<button class="tab" class:active={boardTab === "activity"} onclick={() => (boardTab = "activity")}>{t("profile.full.tabActivity")}</button>
			<button class="tab" class:active={boardTab === "wishlist"} onclick={() => (boardTab = "wishlist")}>{t("profile.full.tabWishlist")}</button>
		</div>

		{#if boardTab === "board"}
			{#if widgets.length === 0}
				<div class="empty">
					<h3>{t("profile.full.widgetsEmptyTitle")}</h3>
					<p>{t("profile.full.widgetsEmptyBody", { name: displayName })}</p>
				</div>
			{:else}
				<div class="widgets-list">
					{#each widgets as widget (widget.id)}
						<div class="widget-card" class:pinned={widget.pinned}>
							<div class="widget-cover" style:background-image={widget.image_url ? `url(${api.resolveUrl(widget.image_url, session.token)})` : undefined}>
								{#if !widget.image_url}<Gamepad2 size={22} strokeWidth={1.5} />{/if}
							</div>
							<div class="widget-body">
								<span class="widget-kind">{WIDGET_KIND_LABELS[widget.kind]}</span>
								<span class="widget-title">{widget.title}</span>
								{#if widget.description}<p class="widget-desc">{widget.description}</p>{/if}
								{#if widget.tags.length > 0}
									<div class="widget-tags">
										{#each widget.tags as tag (tag)}<span class="widget-tag">{tag}</span>{/each}
									</div>
								{/if}
							</div>
							{#if widget.pinned}
								<Pin size={13} strokeWidth={2} fill="currentColor" class="pin-icon" />
							{/if}
						</div>
					{/each}
				</div>
			{/if}
		{:else if boardTab === "activity"}
			<div class="empty">
				<h3>{t("profile.full.activityEmptyTitle")}</h3>
				<p>{t("profile.full.activityEmptyBody")}</p>
			</div>
		{:else}
			<div class="empty">
				<h3>{t("profile.full.wishlistEmptyTitle")}</h3>
				<p>{t("profile.full.wishlistEmptyBody")}</p>
			</div>
		{/if}
	</div>
</div>
</div>

{#if moreOpen && member}
	<ProfileActionsMenu
		{member}
		position={morePosition}
		onClose={() => (moreOpen = false)}
		onViewFullProfile={() => {}}
	/>
{/if}

<style>
	.overlay {
		position: absolute;
		inset: 0;
		z-index: 300;
		display: flex;
		align-items: center;
		justify-content: center;
		padding: 40px;
		background: rgba(0, 0, 0, 0.6);
		backdrop-filter: blur(3px);
		-webkit-backdrop-filter: blur(3px);
	}

	.modal {
		position: relative;
		width: min(760px, 100%);
		height: min(640px, 100%);
		display: grid;
		grid-template-columns: 1.1fr 1fr;
		background: var(--void);
		border-radius: 12px;
		overflow: hidden auto;
		box-shadow: 0 24px 60px rgba(0, 0, 0, 0.5);
		font-family: var(--font-body);
	}

	.close {
		position: absolute;
		top: 16px;
		right: 16px;
		z-index: 10;
		padding: 8px;
		border-radius: 999px;
		color: var(--ink-dim);
		background: rgba(0, 0, 0, 0.25);
		transition: background-color 0.15s ease, color 0.15s ease;
	}

	.close:hover {
		background: var(--hover);
		color: var(--ink);
	}

	.col-main {
		min-width: 0;
		background: var(--sidebar);
		border-right: 1px solid var(--hairline);
		overflow-y: auto;
	}

	.banner {
		position: relative;
		overflow: hidden;
		height: 90px;
		background: var(--panel);
	}

	.body {
		padding: 0 20px 24px;
		margin-top: -32px;
		display: flex;
		flex-direction: column;
		gap: 10px;
	}

	.top-row {
		display: flex;
		align-items: flex-end;
		justify-content: space-between;
		gap: 10px;
	}

	.avatar {
		position: relative;
		overflow: hidden;
		width: 64px;
		height: 64px;
		border-radius: 50%;
		background-position: center;
		background-size: cover;
		display: flex;
		align-items: center;
		justify-content: center;
		font-family: var(--font-body);
		font-weight: 700;
		font-size: 18px;
		color: var(--void);
	}

	.actions {
		display: flex;
		align-items: center;
		gap: 6px;
		margin-bottom: 6px;
	}

	.action {
		display: flex;
		align-items: center;
		gap: 6px;
		padding: 7px 12px;
		border-radius: 6px;
		background: var(--panel);
		border: 1px solid var(--hairline);
		font-size: 12px;
		font-weight: 600;
		color: var(--ink-dim);
		transition: border-color 0.15s ease, color 0.15s ease;
	}

	.action:hover:not(:disabled) {
		border-color: var(--ink-dim);
		color: var(--ink);
	}

	.action:disabled {
		opacity: 0.6;
		cursor: default;
	}

	.action.primary {
		background: var(--accent-fill);
		border-color: var(--accent-fill);
		color: var(--accent-fill-ink);
	}

	.action.icon-only {
		padding: 7px;
	}

	.name {
		margin: 6px 0 0;
		font-family: var(--font-mono);
		font-weight: 700;
		font-size: 18px;
	}

	.handle {
		margin: 1px 0 0;
		display: flex;
		align-items: center;
		flex-wrap: wrap;
		gap: 6px;
		font-size: 12px;
		color: var(--ink-faint);
	}

	.status {
		margin: 2px 0 0;
		font-size: 12px;
		color: var(--ink-faint);
	}

	.activity-line {
		margin: 2px 0 0;
		font-size: 12px;
		line-height: 1.4;
		color: var(--ink-dim);
	}

	.activity-line strong {
		color: var(--ink);
	}

	.divider {
		height: 1px;
		background: var(--hairline);
		margin: 4px 0;
	}

	.mutual {
		margin: 0;
		display: flex;
		align-items: center;
		gap: 5px;
		font-size: 11px;
		color: var(--ink-faint);
	}

	.info-block {
		display: flex;
		flex-direction: column;
		gap: 5px;
	}

	.info-label {
		margin: 0;
		font-size: 13px;
		font-weight: 700;
		color: var(--ink-faint);
	}

	.info-value {
		margin: 0;
		display: flex;
		align-items: center;
		gap: 6px;
		font-size: 13px;
		color: var(--ink);
	}

	.bio-line {
		margin: 0;
		font-size: 13px;
		line-height: 1.5;
		color: var(--ink);
	}

	.connection-list {
		display: flex;
		flex-direction: column;
		gap: 4px;
	}

	.connection-row {
		display: flex;
		align-items: center;
		gap: 8px;
		padding: 6px 8px;
		border-radius: 7px;
		color: var(--ink-dim);
		transition: background-color 0.12s ease;
	}

	.connection-row:hover {
		background: var(--hover);
	}

	.connection-label {
		flex: 1;
		min-width: 0;
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
		font-size: 13px;
		color: var(--ink);
		font-weight: 600;
	}

	.connection-row :global(.connection-arrow) {
		flex-shrink: 0;
		color: var(--ink-faint);
	}

	.note {
		width: 100%;
		min-height: 44px;
		resize: none;
		background: var(--panel);
		border: 1px solid var(--hairline);
		border-radius: 6px;
		padding: 8px 10px;
		color: var(--ink);
		font-family: var(--font-body);
		font-size: 12px;
		line-height: 1.4;
	}

	.note::placeholder {
		color: var(--ink-faint);
	}

	.note:focus {
		outline: none;
		border-color: var(--ink-dim);
	}

	.col-right {
		min-width: 0;
		background: var(--sidebar);
		overflow-y: auto;
		padding: 20px;
	}

	.tabs {
		display: flex;
		gap: 20px;
		border-bottom: 1px solid var(--hairline);
		padding-bottom: 10px;
		margin-bottom: 24px;
	}

	.tab {
		font-size: 13px;
		font-weight: 700;
		color: var(--ink-faint);
		padding-bottom: 10px;
		margin-bottom: -11px;
		border-bottom: 2px solid transparent;
	}

	.tab.active {
		color: var(--ink);
		border-bottom-color: var(--ink);
	}

	.empty {
		display: flex;
		flex-direction: column;
		align-items: center;
		text-align: center;
		gap: 4px;
		margin-top: 48px;
	}

	.empty h3 {
		margin: 0;
		font-size: 15px;
		font-weight: 700;
		color: var(--ink);
	}

	.empty p {
		margin: 0;
		font-size: 12px;
		color: var(--ink-faint);
		max-width: 280px;
	}

	.widgets-list {
		display: flex;
		flex-direction: column;
		gap: 10px;
	}

	.widget-card {
		position: relative;
		display: flex;
		gap: 14px;
		padding: 14px;
		border-radius: 8px;
		border: 1px solid rgba(255, 255, 255, 0.08);
	}

	.widget-card.pinned {
		border-color: var(--accent-fill);
	}

	.widget-cover {
		flex-shrink: 0;
		width: 72px;
		height: 100px;
		border-radius: 6px;
		background: var(--void) center/cover;
		display: flex;
		align-items: center;
		justify-content: center;
		color: var(--ink-faint);
		overflow: hidden;
	}

	.widget-body {
		flex: 1;
		min-width: 0;
		display: flex;
		flex-direction: column;
		gap: 6px;
	}

	.widget-kind {
		font-size: 11px;
		font-weight: 700;
		text-transform: uppercase;
		letter-spacing: 0.03em;
		color: var(--ink-faint);
	}

	.widget-title {
		font-size: 14px;
		font-weight: 700;
		color: var(--ink);
	}

	.widget-desc {
		margin: 0;
		font-size: 12px;
		color: var(--ink-dim);
	}

	.widget-tags {
		display: flex;
		flex-wrap: wrap;
		gap: 6px;
	}

	.widget-tag {
		padding: 3px 9px;
		border-radius: 999px;
		background: var(--void);
		border: 1px solid var(--hairline);
		color: var(--ink-dim);
		font-size: 11px;
		font-weight: 600;
	}

	.widget-card :global(.pin-icon) {
		position: absolute;
		top: 10px;
		right: 10px;
		color: var(--accent-fill);
	}
</style>

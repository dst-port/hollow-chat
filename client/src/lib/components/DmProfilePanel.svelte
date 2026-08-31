<script lang="ts">
	import CalendarDays from "@lucide/svelte/icons/calendar-days";
	import Users from "@lucide/svelte/icons/users";
	import Badges from "$lib/components/Badges.svelte";
	import ActivityCard from "$lib/components/ActivityCard.svelte";
	import { session } from "$lib/stores/session.svelte";
	import { profileStore } from "$lib/stores/profile.svelte";
	import { badgeStore } from "$lib/stores/badges.svelte";
	import { t, tp } from "$lib/i18n/index.svelte";
	import * as api from "$lib/api/client";
	import { isVideoMedia } from "$lib/utils/media";

	let { username, onViewFullProfile }: {
		username: string;
		onViewFullProfile: () => void;
	} = $props();

	$effect(() => {
		const token = session.token;
		if (!token) return;
		badgeStore.loadForUser(token, username);
		profileStore.load(token, username);
	});

	const profile = $derived(profileStore.forUser(username));
	const badges = $derived(badgeStore.forUser(username));
	const presence = $derived(profile?.presence ?? "online");
	const accent = $derived(profile?.accent_color || "#5865f2");
	const displayName = $derived(profile?.display_name || username);
	const avatarIsVideo = $derived(isVideoMedia(profile?.avatar_url));
	const bannerIsVideo = $derived(isVideoMedia(profile?.banner_url));
	const avatarSrc = $derived(profile?.avatar_url ? api.resolveUrl(profile.avatar_url, session.token) : "");
	const bannerSrc = $derived(profile?.banner_url ? api.resolveUrl(profile.banner_url, session.token) : "");
	const memberSince = $derived(
		profile?.member_since
			? new Date(profile.member_since).toLocaleDateString(undefined, { month: "long", day: "numeric", year: "numeric" })
			: null
	);

	let mutualFriends = $state<api.ApiFriend[]>([]);
	$effect(() => {
		const token = session.token;
		if (!token) return;
		api
			.listMutualFriends(token, username)
			.then((rows) => (mutualFriends = rows))
			.catch(() => (mutualFriends = []));
	});
</script>

<aside class="panel">
	<div class="banner" style:background={bannerIsVideo ? "#000" : api.bannerBackground(profile, session.token)}>
		{#if bannerIsVideo}
			<video class="banner-media" src={bannerSrc} autoplay loop muted playsinline></video>
		{/if}
	</div>
	<div class="body">
		<div
			class="avatar avatar-ring on-panel {presence}"
			style:background={avatarSrc && !avatarIsVideo ? undefined : accent}
			style:background-image={avatarSrc && !avatarIsVideo ? `url(${avatarSrc})` : undefined}
		>
			{#if avatarSrc && avatarIsVideo}
				<video class="avatar-media" src={avatarSrc} autoplay loop muted playsinline></video>
			{:else if !avatarSrc}{username.slice(0, 2).toUpperCase()}{/if}
		</div>

		<p class="name" style:color={accent}>{displayName}</p>
		<p class="handle">
			<span>@{username}</span>
			{#if profile?.pronouns}<span>· {profile.pronouns}</span>{/if}
		</p>
		{#if badges.length > 0}<Badges {badges} />{/if}

		{#if profile?.status_text}<p class="status">{profile.status_text}</p>{/if}

		<div class="divider"></div>

		<p class="mutual">
			<Users size={12} strokeWidth={2} />
			{tp("profile.full.mutualFriends", mutualFriends.length)}
		</p>

		{#if profile?.bio}
			<div class="section">
				<p class="section-label">{t("profile.full.aboutMe")}</p>
				<p class="bio">{profile.bio}</p>
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
			<div class="section">
				<p class="section-label">{t("profile.full.memberSince")}</p>
				<p class="member-since"><CalendarDays size={13} strokeWidth={2} /> {memberSince}</p>
			</div>
		{/if}
	</div>

	<button class="view-full" onclick={onViewFullProfile}>{t("profile.dmPanel.viewFullProfile")}</button>
</aside>

<style>
	.panel {
		width: 240px;
		flex-shrink: 0;
		display: flex;
		flex-direction: column;
		background: var(--sidebar);
		border-left: 1px solid var(--hairline);
		overflow-y: auto;
	}

	.banner {
		position: relative;
		overflow: hidden;
		height: 60px;
		flex-shrink: 0;
		background: var(--panel);
	}

	.body {
		padding: 0 16px;
		margin-top: -26px;
		display: flex;
		flex-direction: column;
		gap: 8px;
	}

	.avatar {
		position: relative;
		overflow: hidden;
		width: 56px;
		height: 56px;
		border-radius: 50%;
		background-position: center;
		background-size: cover;
		display: flex;
		align-items: center;
		justify-content: center;
		font-family: var(--font-body);
		font-weight: 700;
		font-size: 16px;
		color: var(--void);
	}

	.name {
		margin: 4px 0 0;
		font-family: var(--font-mono);
		font-weight: 700;
		font-size: 16px;
	}

	.handle {
		margin: 0;
		display: flex;
		align-items: center;
		flex-wrap: wrap;
		gap: 5px;
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

	.section {
		display: flex;
		flex-direction: column;
		gap: 4px;
	}

	.section-label {
		margin: 0;
		font-size: 12px;
		font-weight: 700;
		color: var(--ink-faint);
	}

	.bio {
		margin: 0;
		font-size: 12px;
		line-height: 1.5;
		color: var(--ink);
		white-space: pre-wrap;
	}

	.member-since {
		margin: 0;
		display: flex;
		align-items: center;
		gap: 6px;
		font-size: 12px;
		color: var(--ink);
	}

	.view-full {
		margin: 16px;
		padding: 10px;
		border-radius: 999px;
		background: var(--panel);
		border: 1px solid var(--hairline);
		font-size: 12px;
		font-weight: 700;
		color: var(--ink);
		text-align: center;
		transition: border-color 0.15s ease;
	}

	.view-full:hover {
		border-color: var(--ink-dim);
	}
</style>

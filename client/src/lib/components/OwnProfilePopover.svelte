<script lang="ts">
	import { fly } from "svelte/transition";
	import Pencil from "@lucide/svelte/icons/pencil";
	import IdCard from "@lucide/svelte/icons/id-card";
	import { clickOutside } from "$lib/actions/clickOutside";
	import { toast } from "$lib/stores/toast.svelte";
	import { session } from "$lib/stores/session.svelte";
	import { badgeStore } from "$lib/stores/badges.svelte";
	import { profileStore } from "$lib/stores/profile.svelte";
	import Badges from "$lib/components/Badges.svelte";
	import * as api from "$lib/api/client";

	let { username, anchor, onEditProfile, onClose }: {
		username: string;
		anchor: HTMLElement;
		onEditProfile: () => void;
		onClose: () => void;
	} = $props();

	$effect(() => {
		const token = session.token;
		if (!token) return;
		badgeStore.loadForUser(token, username);
		profileStore.load(token, username);
	});

	const profile = $derived(profileStore.forUser(username));

	const POPOVER_WIDTH = 260;

	function computePosition() {
		const frame = document.querySelector(".window-frame");
		const frameRect = frame ? frame.getBoundingClientRect() : { top: 0, left: 0, bottom: window.innerHeight };
		const anchorRect = anchor.getBoundingClientRect();

		return {
			bottom: frameRect.bottom - anchorRect.top + 8,
			left: anchorRect.left - frameRect.left
		};
	}

	const position = computePosition();

	function copyId() {
		navigator.clipboard.writeText(username);
		toast.push("User ID copied");
	}

	function editProfile() {
		onEditProfile();
		onClose();
	}
</script>

<div
	class="popover"
	use:clickOutside={onClose}
	style:bottom={`${position.bottom}px`}
	style:left={`${position.left}px`}
	style:width={`${POPOVER_WIDTH}px`}
	transition:fly={{ y: 6, duration: 140 }}
>
	<div
		class="banner"
		style:background={profile?.banner_url ? `url(${api.resolveUrl(profile.banner_url)}) center/cover` : (profile?.banner_color ?? undefined)}
	></div>
	<div class="avatar-row status-avatar">
		<div class="avatar" style:background-image={profile?.avatar_url ? `url(${api.resolveUrl(profile.avatar_url)})` : undefined}>
			{#if !profile?.avatar_url}{username.slice(0, 2).toUpperCase()}{/if}
		</div>
		<span class="status-dot on-panel online"></span>
	</div>
	<div class="body">
		<p class="name-row">
			<span class="name" style:color={profile?.accent_color || undefined}>{username}</span>
			<Badges badges={badgeStore.forUser(username)} />
		</p>
		{#if profile?.pronouns}<p class="pronouns">{profile.pronouns}</p>{/if}
		<p class="status">{profile?.status_text || "online"}</p>

		<div class="bio">
			<p class="bio-label">About me</p>
			<p class="bio-text">{profile?.bio || "No bio yet."}</p>
		</div>

		<button class="action" onclick={editProfile}>
			<Pencil size={15} strokeWidth={2} />
			Edit Profile
		</button>
		<button class="action ghost" onclick={copyId}>
			<IdCard size={15} strokeWidth={2} />
			Copy User ID
		</button>
	</div>
</div>

<style>
	.popover {
		position: fixed;
		background: var(--panel);
		border-radius: 10px;
		overflow: hidden;
		box-shadow: 0 16px 40px rgba(0, 0, 0, 0.45);
		z-index: 100;
	}

	.banner {
		height: 60px;
		background: var(--active);
	}

	.avatar-row {
		margin: -26px 0 0 16px;
		width: fit-content;
	}

	.avatar {
		width: 56px;
		height: 56px;
		border: 4px solid var(--panel);
		border-radius: 50%;
		background: var(--accent-fill) center/cover;
		color: var(--accent-fill-ink);
		display: flex;
		align-items: center;
		justify-content: center;
		font-family: var(--font-body);
		font-weight: 700;
		font-size: 16px;
	}

	.pronouns {
		margin: 1px 0 0;
		font-size: 11px;
		color: var(--ink-faint);
	}

	.status-dot {
		width: 13px;
		height: 13px;
	}

	.body {
		padding: 12px 16px 16px;
	}

	.name-row {
		margin: 0;
		display: flex;
		align-items: center;
		flex-wrap: wrap;
		gap: 6px;
	}

	.name {
		font-family: var(--font-mono);
		font-weight: 700;
		font-size: 15px;
		color: var(--ink);
	}

	.status {
		margin: 2px 0 12px;
		font-size: 12px;
		color: var(--online);
	}

	.bio {
		background: var(--sidebar);
		border-radius: 6px;
		padding: 8px 10px;
		margin-bottom: 12px;
	}

	.bio-label {
		margin: 0 0 4px;
		font-family: var(--font-mono);
		font-size: 10px;
		font-weight: 600;
		text-transform: uppercase;
		letter-spacing: 0.03em;
		color: var(--ink-faint);
	}

	.bio-text {
		margin: 0;
		font-size: 12px;
		color: var(--ink-dim);
	}

	.action {
		width: 100%;
		display: flex;
		align-items: center;
		justify-content: center;
		gap: 8px;
		padding: 8px;
		border-radius: 6px;
		background: var(--accent-fill);
		color: var(--accent-fill-ink);
		font-weight: 600;
		font-size: 13px;
		margin-bottom: 6px;
	}

	.action.ghost {
		background: var(--hover);
		color: var(--ink-dim);
	}

	.action.ghost:hover {
		background: var(--active);
		color: var(--ink);
	}
</style>

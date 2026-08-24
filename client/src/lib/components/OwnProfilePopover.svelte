<script lang="ts">
	import { fly } from "svelte/transition";
	import Pencil from "@lucide/svelte/icons/pencil";
	import Trash2 from "@lucide/svelte/icons/trash-2";
	import IdCard from "@lucide/svelte/icons/id-card";
	import SmilePlus from "@lucide/svelte/icons/smile-plus";
	import { clickOutside } from "$lib/actions/clickOutside";
	import { toast } from "$lib/stores/toast.svelte";
	import { session } from "$lib/stores/session.svelte";
	import { badgeStore } from "$lib/stores/badges.svelte";
	import { profileStore } from "$lib/stores/profile.svelte";
	import Badges from "$lib/components/Badges.svelte";
	import StatusModal from "$lib/components/StatusModal.svelte";
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

	let statusModalOpen = $state(false);
	let presenceMenuOpen = $state(false);

	const PRESENCE_OPTIONS: { value: api.PresenceState; label: string }[] = [
		{ value: "online", label: "Online" },
		{ value: "idle", label: "Idle" },
		{ value: "dnd", label: "Do Not Disturb" },
		{ value: "invisible", label: "Invisible" }
	];

	async function pickPresence(value: api.PresenceState) {
		const token = session.token;
		presenceMenuOpen = false;
		if (!token) return;
		try {
			profileStore.set(await api.setPresence(token, value));
		} catch {
			toast.push("Couldn't change status");
		}
	}

	async function quickClearStatus() {
		const token = session.token;
		if (!token) return;
		try {
			const updated = await api.updateProfile(token, { status_text: "", status_clear_minutes: 0 });
			profileStore.set(updated);
		} catch {
			toast.push("Couldn't clear status");
		}
	}

	const POPOVER_WIDTH = 280;

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
		<button
			class="status-dot on-panel {profile?.presence ?? 'online'} status-dot-trigger"
			title="Set status"
			onclick={() => (presenceMenuOpen = !presenceMenuOpen)}
		></button>
		{#if presenceMenuOpen}
			<div class="presence-menu" use:clickOutside={() => (presenceMenuOpen = false)} transition:fly={{ y: -4, duration: 120 }}>
				{#each PRESENCE_OPTIONS as option (option.value)}
					<button class="presence-option" onclick={() => pickPresence(option.value)}>
						<span class="status-dot static {option.value}"></span>
						{option.label}
					</button>
				{/each}
			</div>
		{/if}
		{#if profile?.status_text}
			<div class="status-bubble" transition:fly={{ y: 4, duration: 140 }}>
				{profile.status_text}
				<div class="status-bubble-actions">
					<button class="bubble-action" title="Edit status" onclick={() => (statusModalOpen = true)}>
						<Pencil size={11} strokeWidth={2.5} />
					</button>
					<button class="bubble-action" title="Clear status" onclick={quickClearStatus}>
						<Trash2 size={11} strokeWidth={2.5} />
					</button>
				</div>
			</div>
		{/if}
	</div>
	<div class="body">
		<p class="name">{profile?.display_name || username}</p>
		<p class="meta-row">
			<span class="meta-username" style:color={profile?.accent_color || undefined}>{username}</span>
			{#if profile?.pronouns}<span class="dot">•</span><span class="meta-item">{profile.pronouns}</span>{/if}
			<Badges badges={badgeStore.forUser(username)} />
		</p>

		<div class="bio">
			<p class="bio-label">About me</p>
			<p class="bio-text">{profile?.bio || "No bio yet."}</p>
		</div>

		<button class="action" onclick={editProfile}>
			<Pencil size={15} strokeWidth={2} />
			Edit Profile
		</button>
		<button class="action ghost" onclick={() => (statusModalOpen = true)}>
			<SmilePlus size={15} strokeWidth={2} />
			{profile?.status_text ? "Edit Status" : "Set Status"}
		</button>
		<button class="action ghost" onclick={copyId}>
			<IdCard size={15} strokeWidth={2} />
			Copy User ID
		</button>
	</div>
</div>

{#if statusModalOpen}
	<StatusModal {username} onClose={() => (statusModalOpen = false)} />
{/if}

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
		position: relative;
		margin: -26px 0 0 16px;
		width: fit-content;
	}

	.status-bubble {
		position: absolute;
		left: 64px;
		bottom: 28px;
		max-width: 170px;
		background: var(--active);
		color: var(--ink);
		border-radius: 10px;
		border-bottom-left-radius: 4px;
		padding: 7px 26px 7px 10px;
		font-size: 12px;
		line-height: 1.35;
		box-shadow: 0 8px 20px rgba(0, 0, 0, 0.35);
	}

	.status-bubble-actions {
		position: absolute;
		top: 4px;
		right: 4px;
		display: flex;
		gap: 2px;
		opacity: 0;
		transition: opacity 0.12s ease;
	}

	.status-bubble:hover .status-bubble-actions {
		opacity: 1;
	}

	.bubble-action {
		width: 18px;
		height: 18px;
		border-radius: 50%;
		display: flex;
		align-items: center;
		justify-content: center;
		background: var(--sidebar);
		color: var(--ink-dim);
	}

	.bubble-action:hover {
		background: var(--hover);
		color: var(--ink);
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

	.status-dot {
		width: 13px;
		height: 13px;
	}

	.status-dot-trigger {
		padding: 0;
		cursor: pointer;
		transition: transform 0.1s ease;
	}

	.status-dot-trigger:hover {
		transform: scale(1.15);
	}

	.status-dot.static {
		position: static;
		width: 9px;
		height: 9px;
		border: none;
		flex-shrink: 0;
	}

	.presence-menu {
		position: absolute;
		left: 40px;
		top: 40px;
		background: var(--panel);
		border-radius: 8px;
		padding: 6px;
		min-width: 170px;
		box-shadow: 0 12px 32px rgba(0, 0, 0, 0.4);
		z-index: 20;
	}

	.presence-option {
		width: 100%;
		display: flex;
		align-items: center;
		gap: 8px;
		padding: 8px 10px;
		border-radius: 6px;
		font-size: 12px;
		font-weight: 500;
		color: var(--ink-dim);
	}

	.presence-option:hover {
		background: var(--hover);
		color: var(--ink);
	}

	.body {
		padding: 12px 16px 16px;
	}

	.name {
		margin: 0;
		font-family: var(--font-mono);
		font-weight: 700;
		font-size: 16px;
		color: var(--ink);
	}

	.meta-row {
		margin: 3px 0 12px;
		display: flex;
		align-items: center;
		flex-wrap: wrap;
		gap: 5px;
	}

	.meta-username,
	.meta-item {
		font-size: 12px;
		font-weight: 500;
		color: var(--ink-faint);
	}

	.dot {
		font-size: 10px;
		color: var(--ink-faint);
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

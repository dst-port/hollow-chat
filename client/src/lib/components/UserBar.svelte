<script lang="ts">
	import Mic from "@lucide/svelte/icons/mic";
	import MicOff from "@lucide/svelte/icons/mic-off";
	import Headphones from "@lucide/svelte/icons/headphones";
	import HeadphoneOff from "@lucide/svelte/icons/headphone-off";
	import Settings from "@lucide/svelte/icons/settings";
	import SettingsModal from "$lib/components/SettingsModal.svelte";
	import OwnProfilePopover from "$lib/components/OwnProfilePopover.svelte";
	import { session } from "$lib/stores/session.svelte";
	import { profileStore } from "$lib/stores/profile.svelte";
	import * as api from "$lib/api/client";

	let { username, onLogout }: {
		username: string;
		onLogout: () => void;
	} = $props();

	let muted = $state(false);
	let deafened = $state(false);
	let settingsOpen = $state(false);
	let settingsInitialSection = $state<"account" | "profile">("account");
	let profileAnchor = $state<HTMLElement | null>(null);

	function openEditProfile() {
		settingsInitialSection = "profile";
		settingsOpen = true;
	}

	const profile = $derived(profileStore.forUser(username));

	const PRESENCE_LABELS: Record<string, string> = {
		online: "Online",
		idle: "Idle",
		dnd: "Do Not Disturb",
		invisible: "Invisible"
	};

	$effect(() => {
		const token = session.token;
		if (token) profileStore.load(token, username);
	});
</script>

<div class="user-panel">
	<button class="identity-trigger" onclick={(e) => (profileAnchor = profileAnchor ? null : (e.currentTarget as HTMLElement))}>
		<div class="status-avatar">
			<div class="avatar" style:background-image={profile?.avatar_url ? `url(${api.resolveUrl(profile.avatar_url)})` : undefined}>
				{#if !profile?.avatar_url}{username.slice(0, 2).toUpperCase()}{/if}
			</div>
			<span class="status-dot on-void {profile?.presence ?? 'online'}"></span>
		</div>
		<div class="identity">
			<p class="username" style:color={profile?.accent_color || undefined}>{profile?.display_name || username}</p>
			<p class="status">{profile?.status_text || PRESENCE_LABELS[profile?.presence ?? "online"]}</p>
		</div>
	</button>
	<div class="controls">
		<button
			class="icon-button"
			class:muted-active={muted}
			title={muted ? "Unmute" : "Mute"}
			onclick={() => (muted = !muted)}
		>
			{#if muted}<MicOff size={15} strokeWidth={2} />{:else}<Mic size={15} strokeWidth={2} />{/if}
		</button>
		<button
			class="icon-button"
			class:muted-active={deafened}
			title={deafened ? "Undeafen" : "Deafen"}
			onclick={() => (deafened = !deafened)}
		>
			{#if deafened}<HeadphoneOff size={15} strokeWidth={2} />{:else}<Headphones size={15} strokeWidth={2} />{/if}
		</button>
		<button class="icon-button" title="User settings" onclick={() => { settingsInitialSection = "account"; settingsOpen = true; }}>
			<Settings size={15} strokeWidth={2} />
		</button>
	</div>
</div>

{#if profileAnchor}
	<OwnProfilePopover
		{username}
		anchor={profileAnchor}
		onEditProfile={openEditProfile}
		onClose={() => (profileAnchor = null)}
	/>
{/if}

{#if settingsOpen}
	<SettingsModal {username} initialSection={settingsInitialSection} onClose={() => (settingsOpen = false)} onLogout={onLogout} />
{/if}

<style>
	.user-panel {
		height: 56px;
		flex-shrink: 0;
		display: flex;
		align-items: center;
		gap: 8px;
		padding: 0 8px;
		background: var(--void);
	}

	.identity-trigger {
		flex: 1;
		min-width: 0;
		display: flex;
		align-items: center;
		gap: 8px;
		padding: 4px;
		border-radius: 8px;
		transition: background-color 0.15s ease;
	}

	.identity-trigger:hover {
		background: var(--hover);
	}

	.avatar {
		width: 36px;
		height: 36px;
		border-radius: 50%;
		background: var(--accent-fill) center/cover;
		color: var(--accent-fill-ink);
		display: flex;
		align-items: center;
		justify-content: center;
		font-family: var(--font-body);
		font-weight: 700;
		font-size: 12px;
	}

	.identity {
		flex: 1;
		min-width: 0;
	}

	.username {
		margin: 0;
		font-family: var(--font-mono);
		font-size: 13px;
		font-weight: 600;
		white-space: nowrap;
		overflow: hidden;
		text-overflow: ellipsis;
		color: var(--ink);
	}

	.status {
		margin: 0;
		font-size: 11px;
		color: var(--online);
	}

	.controls {
		display: flex;
		align-items: center;
		gap: 2px;
		padding: 4px;
		border-radius: 999px;
		background: var(--panel);
	}

	.icon-button {
		padding: 7px;
		border-radius: 999px;
		color: var(--ink-dim);
		display: flex;
		transition: background-color 0.15s ease, color 0.15s ease;
	}

	.icon-button:hover {
		background: var(--hover);
		color: var(--ink);
	}

	.icon-button.muted-active {
		color: var(--danger);
	}
</style>

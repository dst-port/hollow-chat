<script lang="ts">
	import Mic from "@lucide/svelte/icons/mic";
	import MicOff from "@lucide/svelte/icons/mic-off";
	import Headphones from "@lucide/svelte/icons/headphones";
	import HeadphoneOff from "@lucide/svelte/icons/headphone-off";
	import Settings from "@lucide/svelte/icons/settings";
	import SettingsModal from "$lib/components/SettingsModal.svelte";
	import EditProfileModal from "$lib/components/EditProfileModal.svelte";
	import OwnProfilePopover from "$lib/components/OwnProfilePopover.svelte";
	import { session } from "$lib/stores/session.svelte";
	import { profileStore } from "$lib/stores/profile.svelte";
	import { call } from "$lib/webrtc/call.svelte";
	import * as api from "$lib/api/client";

	let { username, onLogout }: {
		username: string;
		onLogout: () => void;
	} = $props();

	let settingsOpen = $state(false);
	let settingsInitialSection = $state<"account" | "profile">("account");
	let profileAnchor = $state<HTMLElement | null>(null);
	let editProfileOpen = $state(false);

	function openEditProfile() {
		editProfileOpen = true;
	}

	const profile = $derived(profileStore.forUser(username));

	const PRESENCE_LABELS: Record<string, string> = {
		online: "Online",
		idle: "Idle",
		dnd: "Do Not Disturb",
		invisible: "Invisible"
	};

	const PRESENCE_COLOR_VAR: Record<string, string> = {
		online: "var(--online)",
		idle: "var(--idle)",
		dnd: "var(--danger)",
		invisible: "var(--ink-faint)"
	};

	const statusColor = $derived(
		profile?.status_text ? "var(--ink-faint)" : PRESENCE_COLOR_VAR[profile?.presence ?? "online"]
	);

	$effect(() => {
		const token = session.token;
		if (token) profileStore.load(token, username);
	});
</script>

<div class="user-panel">
	<div class="panel-pill">
	<button class="identity-trigger" onclick={(e) => (profileAnchor = profileAnchor ? null : (e.currentTarget as HTMLElement))}>
		<div
			class="avatar avatar-ring {profile?.presence ?? 'online'}"
			style:--ring-gap="var(--hover)"
			style:background-image={profile?.avatar_url ? `url(${api.resolveUrl(profile.avatar_url, session.token)})` : undefined}
		>
			{#if !profile?.avatar_url}{username.slice(0, 2).toUpperCase()}{/if}
		</div>
		<div class="identity">
			<p class="username" style:color={profile?.accent_color || undefined}>{profile?.display_name || username}</p>
			<p class="status-row">
				<span class="status" style:color={statusColor}>{profile?.status_text || PRESENCE_LABELS[profile?.presence ?? "online"]}</span>
			</p>
		</div>
	</button>
	<div class="controls">
		<button
			class="icon-button"
			class:muted-active={call.muted}
			aria-label={call.muted ? "Unmute" : "Mute"}
			onclick={() => call.toggleMute()}
		>
			{#if call.muted}<MicOff size={18} strokeWidth={2} />{:else}<Mic size={18} strokeWidth={2} />{/if}
			<span class="tooltip">{call.muted ? "Unmute" : "Mute"}</span>
		</button>
		<button
			class="icon-button"
			class:muted-active={call.deafened}
			aria-label={call.deafened ? "Undeafen" : "Deafen"}
			onclick={() => call.toggleDeafen()}
		>
			{#if call.deafened}<HeadphoneOff size={18} strokeWidth={2} />{:else}<Headphones size={18} strokeWidth={2} />{/if}
			<span class="tooltip">{call.deafened ? "Undeafen" : "Deafen"}</span>
		</button>
		<button class="icon-button" aria-label="User Settings" onclick={() => { settingsInitialSection = "account"; settingsOpen = true; }}>
			<Settings size={18} strokeWidth={2} />
			<span class="tooltip">User Settings</span>
		</button>
	</div>
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

{#if editProfileOpen}
	<EditProfileModal {username} onClose={() => (editProfileOpen = false)} />
{/if}

<style>
	.user-panel {
		flex-shrink: 0;
		display: flex;
		align-items: center;
		padding: 8px;
	}

	.panel-pill {
		flex: 1;
		min-width: 0;
		display: flex;
		align-items: center;
		justify-content: space-between;
		gap: 8px;
		padding: 4px 8px;
		border-radius: 10px;
		background: var(--hover);
	}

	.identity-trigger {
		flex: 1;
		min-width: 0;
		display: flex;
		align-items: center;
		gap: 8px;
		padding: 0 4px;
		border-radius: 8px;
		background: transparent;
		transition: background-color 0.15s ease;
	}

	.identity-trigger:hover {
		background: var(--active);
	}

	.avatar {
		flex-shrink: 0;
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
		text-align: left;
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

	.status-row {
		margin: 0;
		display: flex;
		align-items: center;
		gap: 4px;
		min-width: 0;
	}

	.status {
		font-size: 11px;
		white-space: nowrap;
		overflow: hidden;
		text-overflow: ellipsis;
	}

	.controls {
		flex-shrink: 0;
		display: flex;
		align-items: center;
		gap: 4px;
	}

	.icon-button {
		position: relative;
		padding: 9px;
		border-radius: 999px;
		color: var(--ink-dim);
		display: flex;
		transition: background-color 0.15s ease, color 0.15s ease;
	}

	.tooltip {
		position: absolute;
		bottom: calc(100% + 8px);
		left: 50%;
		transform: translateX(-50%);
		padding: 6px 10px;
		border-radius: 6px;
		background: var(--void);
		color: var(--ink);
		font-size: 12px;
		font-weight: 700;
		white-space: nowrap;
		box-shadow: 0 4px 12px rgba(0, 0, 0, 0.45);
		opacity: 0;
		pointer-events: none;
		transition: opacity 0.1s ease;
		z-index: 30;
	}

	.tooltip::after {
		content: "";
		position: absolute;
		top: 100%;
		left: 50%;
		transform: translateX(-50%);
		border: 5px solid transparent;
		border-top-color: var(--void);
	}

	.icon-button:hover .tooltip {
		opacity: 1;
	}

	.icon-button:hover {
		background: var(--active);
		color: var(--ink);
	}

	.icon-button.muted-active {
		color: var(--danger);
	}
</style>

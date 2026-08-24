<script lang="ts">
	import { fade, scale } from "svelte/transition";
	import { cubicOut } from "svelte/easing";
	import X from "@lucide/svelte/icons/x";
	import UserRound from "@lucide/svelte/icons/user-round";
	import ShieldCheck from "@lucide/svelte/icons/shield-check";
	import Bell from "@lucide/svelte/icons/bell";
	import Palette from "@lucide/svelte/icons/palette";
	import Monitor from "@lucide/svelte/icons/monitor";
	import LogOut from "@lucide/svelte/icons/log-out";
	import { toast } from "$lib/stores/toast.svelte";

	let { username, onClose, onLogout }: {
		username: string;
		onClose: () => void;
		onLogout: () => void;
	} = $props();

	type Section = "account" | "privacy" | "notifications" | "appearance" | "sessions";
	let section = $state<Section>("account");

	const initialUsername = username;

	let editingUsername = $state(false);
	let usernameDraft = $state(initialUsername);
	let editingPassword = $state(false);
	let passwordDraft = $state("");

	let notifyMessages = $state(true);
	let notifyMentions = $state(true);
	let notifySounds = $state(true);

	let reducedMotion = $state(false);
	let compactMode = $state(false);

	const sessions = [
		{ id: "1", label: "This device", detail: "Linux · Wayland · current session", current: true },
		{ id: "2", label: "Unknown device", detail: "Last active 3 days ago", current: false }
	];
	let revokedIds = $state<string[]>([]);

	function onKeydown(event: KeyboardEvent) {
		if (event.key === "Escape") onClose();
	}

	function saveUsername() {
		toast.push("Changing your username isn't wired up yet");
		editingUsername = false;
		usernameDraft = username;
	}

	function savePassword() {
		if (!passwordDraft.trim()) return;
		toast.push("Changing your password isn't wired up yet");
		editingPassword = false;
		passwordDraft = "";
	}

	function revoke(id: string) {
		revokedIds.push(id);
		toast.push("Session revoked");
	}

	function toggle(setter: (v: boolean) => void, current: boolean, label: string) {
		setter(!current);
		toast.push(`${label} ${!current ? "enabled" : "disabled"}`);
	}
</script>

<svelte:window onkeydown={onKeydown} />

<div class="overlay" role="presentation" onclick={onClose} transition:fade={{ duration: 150 }}>
	<div
		class="modal"
		role="dialog"
		aria-modal="true"
		aria-label="User settings"
		tabindex="-1"
		onclick={(e) => e.stopPropagation()}
		transition:scale={{ duration: 180, start: 0.97, easing: cubicOut }}
	>
		<nav class="nav">
			<p class="nav-label">User Settings</p>
			<button class="nav-item" class:active={section === "account"} onclick={() => (section = "account")}>
				<UserRound size={16} strokeWidth={2} />
				My Account
			</button>
			<button class="nav-item" class:active={section === "privacy"} onclick={() => (section = "privacy")}>
				<ShieldCheck size={16} strokeWidth={2} />
				Privacy &amp; Safety
			</button>
			<button class="nav-item" class:active={section === "notifications"} onclick={() => (section = "notifications")}>
				<Bell size={16} strokeWidth={2} />
				Notifications
			</button>
			<button class="nav-item" class:active={section === "appearance"} onclick={() => (section = "appearance")}>
				<Palette size={16} strokeWidth={2} />
				Appearance
			</button>
			<button class="nav-item" class:active={section === "sessions"} onclick={() => (section = "sessions")}>
				<Monitor size={16} strokeWidth={2} />
				Devices
			</button>

			<div class="nav-spacer"></div>

			<button class="nav-item danger" onclick={onLogout}>
				<LogOut size={16} strokeWidth={2} />
				Log Out
			</button>
		</nav>

		<div class="content">
			<button class="close" onclick={onClose} title="Close">
				<X size={20} strokeWidth={2} />
			</button>

			{#if section === "account"}
				<h2>My Account</h2>

				<div class="card">
					<div class="identity">
						<div class="avatar">{username.slice(0, 2).toUpperCase()}</div>
						<div>
							<p class="username">{username}</p>
							<p class="hint">HollowChat account</p>
						</div>
					</div>

					<div class="row">
						<div>
							<p class="row-label">Username</p>
							{#if editingUsername}
								<input class="inline-input" type="text" bind:value={usernameDraft} maxlength="32" />
							{:else}
								<p class="row-value">{username}</p>
							{/if}
						</div>
						{#if editingUsername}
							<div class="row-actions">
								<button class="ghost" onclick={() => ((editingUsername = false), (usernameDraft = username))}>Cancel</button>
								<button class="primary" onclick={saveUsername} disabled={!usernameDraft.trim()}>Save</button>
							</div>
						{:else}
							<button class="edit" onclick={() => (editingUsername = true)}>Edit</button>
						{/if}
					</div>

					<div class="row">
						<div>
							<p class="row-label">Email</p>
							<p class="row-value muted">Not collected — HollowChat never asks for one.</p>
						</div>
					</div>

					<div class="row">
						<div>
							<p class="row-label">Phone number</p>
							<p class="row-value muted">Not collected — HollowChat never asks for one.</p>
						</div>
					</div>
				</div>

				<div class="card">
					<div class="row">
						<div>
							<p class="row-label">Password</p>
							{#if editingPassword}
								<input class="inline-input" type="password" bind:value={passwordDraft} placeholder="New password" />
							{:else}
								<p class="row-value">••••••••••••</p>
							{/if}
						</div>
						{#if editingPassword}
							<div class="row-actions">
								<button class="ghost" onclick={() => ((editingPassword = false), (passwordDraft = ""))}>Cancel</button>
								<button class="primary" onclick={savePassword} disabled={!passwordDraft.trim()}>Save</button>
							</div>
						{:else}
							<button class="edit" onclick={() => (editingPassword = true)}>Change</button>
						{/if}
					</div>

					<div class="row">
						<div>
							<p class="row-label">Password recovery</p>
							<p class="row-value muted">
								There is no email or phone number on file, so there is no password reset. Losing
								your password means losing the account.
							</p>
						</div>
					</div>
				</div>
			{:else if section === "privacy"}
				<h2>Privacy &amp; Safety</h2>

				<div class="card">
					<div class="row">
						<div>
							<p class="row-label">Data collected</p>
							<p class="row-value muted">Username and a password hash. Nothing else.</p>
						</div>
					</div>
					<div class="row">
						<div>
							<p class="row-label">Message storage</p>
							<p class="row-value muted">
								End-to-end encrypted. The server only ever sees ciphertext.
							</p>
						</div>
					</div>
					<div class="row">
						<div>
							<p class="row-label">IP logging</p>
							<p class="row-value muted">Never logged, on any layer of the stack.</p>
						</div>
					</div>
				</div>

				<div class="card">
					<div class="switch-row">
						<div>
							<p class="row-label">Direct messages from server members</p>
							<p class="row-value muted">Allow anyone sharing a server with you to message you directly.</p>
						</div>
						<label class="switch">
							<input type="checkbox" checked />
							<span class="track"><span class="thumb"></span></span>
						</label>
					</div>
				</div>
			{:else if section === "notifications"}
				<h2>Notifications</h2>

				<div class="card">
					<div class="switch-row">
						<div>
							<p class="row-label">Messages</p>
							<p class="row-value muted">Notify when someone sends you a message.</p>
						</div>
						<label class="switch">
							<input type="checkbox" bind:checked={notifyMessages} onchange={() => toggle((v) => (notifyMessages = v), !notifyMessages, "Message notifications")} />
							<span class="track"><span class="thumb"></span></span>
						</label>
					</div>
					<div class="switch-row">
						<div>
							<p class="row-label">Mentions</p>
							<p class="row-value muted">Notify when someone @mentions you.</p>
						</div>
						<label class="switch">
							<input type="checkbox" bind:checked={notifyMentions} onchange={() => toggle((v) => (notifyMentions = v), !notifyMentions, "Mention notifications")} />
							<span class="track"><span class="thumb"></span></span>
						</label>
					</div>
					<div class="switch-row">
						<div>
							<p class="row-label">Notification sounds</p>
							<p class="row-value muted">Play a sound for incoming notifications.</p>
						</div>
						<label class="switch">
							<input type="checkbox" bind:checked={notifySounds} onchange={() => toggle((v) => (notifySounds = v), !notifySounds, "Notification sounds")} />
							<span class="track"><span class="thumb"></span></span>
						</label>
					</div>
				</div>
			{:else if section === "appearance"}
				<h2>Appearance</h2>

				<div class="card">
					<div class="switch-row">
						<div>
							<p class="row-label">Compact mode</p>
							<p class="row-value muted">Reduce spacing between messages.</p>
						</div>
						<label class="switch">
							<input type="checkbox" bind:checked={compactMode} onchange={() => toggle((v) => (compactMode = v), !compactMode, "Compact mode")} />
							<span class="track"><span class="thumb"></span></span>
						</label>
					</div>
					<div class="switch-row">
						<div>
							<p class="row-label">Reduce motion</p>
							<p class="row-value muted">Minimize animations and transitions across the app.</p>
						</div>
						<label class="switch">
							<input type="checkbox" bind:checked={reducedMotion} onchange={() => toggle((v) => (reducedMotion = v), !reducedMotion, "Reduced motion")} />
							<span class="track"><span class="thumb"></span></span>
						</label>
					</div>
				</div>

				<div class="card">
					<p class="row-label">Theme</p>
					<p class="row-value muted">HollowChat ships with one neutral theme. More are on the way.</p>
				</div>
			{:else}
				<h2>Devices</h2>
				<p class="hint" style="margin-bottom: 16px;">Sessions currently signed in to your account.</p>

				<div class="card">
					{#each sessions.filter((s) => !revokedIds.includes(s.id)) as s (s.id)}
						<div class="row">
							<div>
								<p class="row-label">{s.label}{s.current ? " (current)" : ""}</p>
								<p class="row-value muted">{s.detail}</p>
							</div>
							{#if !s.current}
								<button class="edit danger-text" onclick={() => revoke(s.id)}>Revoke</button>
							{/if}
						</div>
					{/each}
				</div>
			{/if}
		</div>
	</div>
</div>

<style>
	.overlay {
		position: fixed;
		inset: 0;
		background: rgba(0, 0, 0, 0.6);
		display: flex;
		z-index: 100;
	}

	.modal {
		display: flex;
		width: 100%;
		height: 100%;
		background: var(--panel);
	}

	.nav {
		width: 220px;
		flex-shrink: 0;
		background: var(--sidebar);
		padding: 24px 12px;
		display: flex;
		flex-direction: column;
	}

	.nav-label {
		margin: 0 8px 8px;
		font-family: var(--font-mono);
		font-size: 11px;
		font-weight: 600;
		text-transform: uppercase;
		letter-spacing: 0.04em;
		color: var(--ink-faint);
	}

	.nav-item {
		display: flex;
		align-items: center;
		gap: 10px;
		padding: 8px;
		border-radius: 6px;
		font-size: 14px;
		font-weight: 500;
		color: var(--ink-dim);
		text-align: left;
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

	.nav-spacer {
		flex: 1;
	}

	.nav-item.danger {
		color: var(--danger);
	}

	.nav-item.danger:hover {
		background: rgba(216, 60, 62, 0.12);
		color: var(--danger);
	}

	.content {
		flex: 1;
		position: relative;
		padding: 48px 40px;
		max-width: 660px;
		overflow-y: auto;
	}

	.close {
		position: absolute;
		top: 24px;
		right: 24px;
		padding: 8px;
		border-radius: 50%;
		border: 1px solid var(--hairline);
		color: var(--ink-dim);
		transition: background-color 0.15s ease, color 0.15s ease;
	}

	.close:hover {
		background: var(--hover);
		color: var(--ink);
	}

	h2 {
		margin: 0 0 20px;
		font-family: var(--font-display);
		font-weight: 700;
		font-size: 20px;
	}

	.card {
		background: var(--sidebar);
		border-radius: 8px;
		padding: 20px;
		margin-bottom: 16px;
	}

	.identity {
		display: flex;
		align-items: center;
		gap: 14px;
		margin-bottom: 20px;
	}

	.avatar {
		width: 48px;
		height: 48px;
		border-radius: 50%;
		background: var(--accent-fill);
		color: var(--accent-fill-ink);
		display: flex;
		align-items: center;
		justify-content: center;
		font-family: var(--font-body);
		font-weight: 700;
		font-size: 15px;
		flex-shrink: 0;
	}

	.username {
		margin: 0;
		font-family: var(--font-mono);
		font-weight: 600;
		font-size: 16px;
		color: var(--ink);
	}

	.hint {
		margin: 2px 0 0;
		font-size: 12px;
		color: var(--ink-faint);
	}

	.row {
		display: flex;
		align-items: center;
		justify-content: space-between;
		gap: 16px;
		padding: 12px 0;
		border-top: 1px solid var(--hairline);
	}

	.row:first-of-type {
		border-top: none;
		padding-top: 0;
	}

	.row-label {
		margin: 0 0 4px;
		font-family: var(--font-mono);
		font-size: 11px;
		font-weight: 600;
		text-transform: uppercase;
		letter-spacing: 0.03em;
		color: var(--ink-faint);
	}

	.row-value {
		margin: 0;
		font-size: 14px;
		color: var(--ink);
	}

	.row-value.muted {
		color: var(--ink-dim);
		line-height: 1.5;
	}

	.row-actions {
		display: flex;
		gap: 8px;
		flex-shrink: 0;
	}

	.inline-input {
		background: var(--panel);
		border: 1px solid var(--hairline);
		border-radius: 6px;
		padding: 8px 10px;
		color: var(--ink);
		font-family: var(--font-body);
		font-size: 13px;
		min-width: 220px;
	}

	.inline-input:focus {
		outline: none;
		border-color: var(--ink-dim);
	}

	.edit {
		flex-shrink: 0;
		padding: 8px 14px;
		border-radius: 6px;
		background: var(--active);
		color: var(--ink);
		font-weight: 600;
		font-size: 12px;
	}

	.edit:hover {
		background: var(--hover);
	}

	.edit.danger-text {
		color: var(--danger);
	}

	.ghost {
		padding: 8px 14px;
		border-radius: 6px;
		color: var(--ink-dim);
		font-weight: 600;
		font-size: 12px;
	}

	.ghost:hover {
		background: var(--hover);
		color: var(--ink);
	}

	.primary {
		padding: 8px 14px;
		border-radius: 6px;
		background: var(--accent-fill);
		color: var(--accent-fill-ink);
		font-weight: 700;
		font-size: 12px;
	}

	.primary:disabled {
		background: var(--active);
		color: var(--ink-faint);
	}

	.switch-row {
		display: flex;
		align-items: center;
		justify-content: space-between;
		gap: 16px;
		padding: 12px 0;
		border-top: 1px solid var(--hairline);
	}

	.switch-row:first-of-type {
		border-top: none;
		padding-top: 0;
	}

	.switch {
		position: relative;
		flex-shrink: 0;
		width: 40px;
		height: 22px;
	}

	.switch input {
		position: absolute;
		opacity: 0;
		width: 100%;
		height: 100%;
		margin: 0;
		cursor: pointer;
	}

	.track {
		display: block;
		width: 100%;
		height: 100%;
		border-radius: 999px;
		background: var(--active);
		transition: background-color 0.15s ease;
	}

	.thumb {
		position: absolute;
		top: 3px;
		left: 3px;
		width: 16px;
		height: 16px;
		border-radius: 50%;
		background: var(--ink-faint);
		transition: transform 0.15s ease, background-color 0.15s ease;
	}

	.switch input:checked + .track {
		background: var(--accent-soft);
	}

	.switch input:checked + .track .thumb {
		transform: translateX(18px);
		background: var(--ink);
	}
</style>

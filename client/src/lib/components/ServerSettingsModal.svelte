<script lang="ts">
	import { fade, scale } from "svelte/transition";
	import { cubicOut } from "svelte/easing";
	import X from "@lucide/svelte/icons/x";
	import LayoutGrid from "@lucide/svelte/icons/layout-grid";
	import UserPlus from "@lucide/svelte/icons/user-plus";
	import ShieldAlert from "@lucide/svelte/icons/shield-alert";
	import Hash from "@lucide/svelte/icons/hash";
	import Trash2 from "@lucide/svelte/icons/trash-2";
	import InviteModal from "$lib/components/InviteModal.svelte";
	import { toast } from "$lib/stores/toast.svelte";
	import { session } from "$lib/stores/session.svelte";
	import { setSlowmode } from "$lib/api/client";
	import type { ServerEntry } from "$lib/data/mock";

	let { server, onClose, onLeave }: {
		server: ServerEntry;
		onClose: () => void;
		onLeave: () => void;
	} = $props();

	type Section = "overview" | "channels" | "invites" | "moderation";
	const initialName = server.name;
	const isOwner = server.ownerId === session.userId;

	const SLOWMODE_OPTIONS = [
		{ label: "Off", seconds: 0 },
		{ label: "5s", seconds: 5 },
		{ label: "10s", seconds: 10 },
		{ label: "30s", seconds: 30 },
		{ label: "1m", seconds: 60 },
		{ label: "5m", seconds: 300 },
		{ label: "15m", seconds: 900 },
		{ label: "1h", seconds: 3600 }
	];

	let section = $state<Section>("overview");
	let name = $state(initialName);
	let inviteOpen = $state(false);
	let confirmDelete = $state(false);

	async function changeSlowmode(channelId: string, seconds: number) {
		const token = session.token;
		if (!token) return;
		try {
			await setSlowmode(token, server.id, channelId, seconds);
			const channel = server.channels.find((c) => c.id === channelId);
			if (channel) channel.slowmodeSeconds = seconds;
		} catch {
			toast.push("Couldn't update slowmode");
		}
	}

	function onKeydown(event: KeyboardEvent) {
		if (event.key === "Escape") onClose();
	}

	function saveName() {
		const trimmed = name.trim();
		if (!trimmed) return;
		server.name = trimmed;
		server.initials = trimmed.slice(0, 2).toUpperCase();
		toast.push("Server updated");
	}

	function deleteServer() {
		onClose();
		onLeave();
		toast.push(`${server.name} deleted`);
	}
</script>

<svelte:window onkeydown={onKeydown} />

<div class="overlay" role="presentation" onclick={onClose} transition:fade={{ duration: 150 }}>
	<div
		class="modal"
		role="dialog"
		aria-modal="true"
		aria-label="Server settings"
		tabindex="-1"
		onclick={(e) => e.stopPropagation()}
		transition:scale={{ duration: 180, start: 0.97, easing: cubicOut }}
	>
		<nav class="nav">
			<p class="nav-label">{server.name}</p>
			<button class="nav-item" class:active={section === "overview"} onclick={() => (section = "overview")}>
				<LayoutGrid size={16} strokeWidth={2} />
				Overview
			</button>
			<button class="nav-item" class:active={section === "channels"} onclick={() => (section = "channels")}>
				<Hash size={16} strokeWidth={2} />
				Channels
			</button>
			<button class="nav-item" class:active={section === "invites"} onclick={() => (section = "invites")}>
				<UserPlus size={16} strokeWidth={2} />
				Invites
			</button>
			<button class="nav-item" class:active={section === "moderation"} onclick={() => (section = "moderation")}>
				<ShieldAlert size={16} strokeWidth={2} />
				Moderation
			</button>

			<div class="nav-spacer"></div>

			<button class="nav-item danger" onclick={() => (confirmDelete = true)}>
				<Trash2 size={16} strokeWidth={2} />
				Delete Server
			</button>
		</nav>

		<div class="content">
			<button class="close" onclick={onClose} title="Close">
				<X size={20} strokeWidth={2} />
			</button>

			{#if section === "overview"}
				<h2>Server Overview</h2>

				<div class="card">
					<div class="identity">
						<div class="server-icon">{server.initials}</div>
						<div>
							<p class="hint">Server icon</p>
							<p class="hint muted">Initials are generated from the server name.</p>
						</div>
					</div>

					<label class="field">
						Server name
						<div class="row-input">
							<input type="text" bind:value={name} maxlength="48" />
							<button class="save" disabled={!name.trim() || name === server.name} onclick={saveName}>
								Save
							</button>
						</div>
					</label>

					<div class="row">
						<div>
							<p class="row-label">Server ID</p>
							<p class="row-value muted">{server.id}</p>
						</div>
					</div>

					<div class="row">
						<div>
							<p class="row-label">Channels</p>
							<p class="row-value muted">{server.channels.length} total</p>
						</div>
					</div>
				</div>
			{:else if section === "channels"}
				<h2>Channels</h2>
				<div class="card">
					<p class="row-label">Slowmode</p>
					<p class="row-value muted" style="margin-bottom: 12px;">
						Limit how often each member can send a message in a text channel.
					</p>
					{#each server.channels.filter((c) => c.type === "text") as channel (channel.id)}
						<div class="row slowmode-row">
							<span class="channel-name">
								<Hash size={14} strokeWidth={2} />
								{channel.name}
							</span>
							{#if isOwner}
								<select
									value={channel.slowmodeSeconds ?? 0}
									onchange={(e) => changeSlowmode(channel.id, Number(e.currentTarget.value))}
								>
									{#each SLOWMODE_OPTIONS as opt (opt.seconds)}
										<option value={opt.seconds}>{opt.label}</option>
									{/each}
								</select>
							{:else}
								<span class="row-value muted">
									{SLOWMODE_OPTIONS.find((o) => o.seconds === (channel.slowmodeSeconds ?? 0))?.label ?? "Off"}
								</span>
							{/if}
						</div>
					{/each}
				</div>
			{:else if section === "invites"}
				<h2>Invites</h2>
				<div class="card">
					<p class="row-label">Active invite link</p>
					<p class="row-value muted" style="margin-bottom: 12px;">
						Anyone with this link can join. It never expires.
					</p>
					<button class="save" onclick={() => (inviteOpen = true)}>Show Invite Link</button>
				</div>
			{:else}
				<h2>Moderation</h2>
				<div class="card">
					<div class="row">
						<div>
							<p class="row-label">Verification level</p>
							<p class="row-value muted">None — anyone with an invite link can join instantly.</p>
						</div>
					</div>
					<div class="row">
						<div>
							<p class="row-label">Message logging</p>
							<p class="row-value muted">Off. HollowChat never logs message content, even for admins.</p>
						</div>
					</div>
				</div>
			{/if}
		</div>
	</div>
</div>

{#if inviteOpen}
	<InviteModal serverName={server.name} serverId={server.id} onClose={() => (inviteOpen = false)} />
{/if}

{#if confirmDelete}
	<div class="confirm-overlay" role="presentation" onclick={() => (confirmDelete = false)} transition:fade={{ duration: 120 }}>
		<div class="confirm" role="dialog" aria-modal="true" tabindex="-1" onclick={(e) => e.stopPropagation()}>
			<h3>Delete "{server.name}"?</h3>
			<p>This can't be undone. All channels and messages in this server will be gone.</p>
			<div class="confirm-actions">
				<button class="cancel" onclick={() => (confirmDelete = false)}>Cancel</button>
				<button class="delete" onclick={deleteServer}>Delete Server</button>
			</div>
		</div>
	</div>
{/if}

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
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
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

	.server-icon {
		width: 48px;
		height: 48px;
		border-radius: 16px;
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

	.hint {
		margin: 0;
		font-size: 13px;
		color: var(--ink);
	}

	.hint.muted {
		margin-top: 2px;
		font-size: 12px;
		color: var(--ink-faint);
	}

	.field {
		display: flex;
		flex-direction: column;
		gap: 6px;
		font-family: var(--font-mono);
		font-size: 11px;
		font-weight: 600;
		text-transform: uppercase;
		letter-spacing: 0.03em;
		color: var(--ink-dim);
		margin-bottom: 4px;
	}

	.row-input {
		display: flex;
		gap: 8px;
	}

	.row-input input {
		flex: 1;
		background: var(--panel);
		border: 1px solid var(--hairline);
		border-radius: 6px;
		padding: 10px 12px;
		color: var(--ink);
		font-family: var(--font-body);
		font-size: 14px;
	}

	.row-input input:focus {
		outline: none;
		border-color: var(--ink-dim);
	}

	.save {
		flex-shrink: 0;
		padding: 10px 16px;
		border-radius: 6px;
		background: var(--accent-fill);
		color: var(--accent-fill-ink);
		font-weight: 700;
		font-size: 13px;
	}

	.save:disabled {
		background: var(--active);
		color: var(--ink-faint);
	}

	.row {
		display: flex;
		align-items: center;
		justify-content: space-between;
		padding: 12px 0;
		border-top: 1px solid var(--hairline);
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

	.slowmode-row {
		padding: 10px 0;
	}

	.channel-name {
		display: flex;
		align-items: center;
		gap: 6px;
		font-size: 14px;
		color: var(--ink);
	}

	.channel-name :global(svg) {
		color: var(--ink-faint);
	}

	.slowmode-row select {
		background: var(--panel);
		border: 1px solid var(--hairline);
		border-radius: 6px;
		padding: 6px 8px;
		color: var(--ink);
		font-family: var(--font-body);
		font-size: 13px;
	}

	.confirm-overlay {
		position: fixed;
		inset: 0;
		background: rgba(0, 0, 0, 0.6);
		display: flex;
		align-items: center;
		justify-content: center;
		z-index: 300;
	}

	.confirm {
		width: 420px;
		background: var(--sidebar);
		border-radius: 10px;
		padding: 24px;
		box-shadow: 0 20px 60px rgba(0, 0, 0, 0.5);
	}

	.confirm h3 {
		margin: 0 0 10px;
		font-family: var(--font-body);
		font-size: 17px;
	}

	.confirm p {
		margin: 0 0 20px;
		font-size: 13px;
		color: var(--ink-dim);
		line-height: 1.5;
	}

	.confirm-actions {
		display: flex;
		justify-content: flex-end;
		gap: 8px;
	}

	.cancel {
		padding: 9px 16px;
		border-radius: 6px;
		color: var(--ink-dim);
	}

	.cancel:hover {
		background: var(--hover);
		color: var(--ink);
	}

	.delete {
		padding: 9px 16px;
		border-radius: 6px;
		background: var(--danger);
		color: white;
		font-weight: 600;
	}
</style>

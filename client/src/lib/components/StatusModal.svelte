<script lang="ts">
	import ChevronDown from "@lucide/svelte/icons/chevron-down";
	import X from "@lucide/svelte/icons/x";
	import Modal from "$lib/components/Modal.svelte";
	import Badges from "$lib/components/Badges.svelte";
	import { clickOutside } from "$lib/actions/clickOutside";
	import { session } from "$lib/stores/session.svelte";
	import { badgeStore } from "$lib/stores/badges.svelte";
	import { profileStore } from "$lib/stores/profile.svelte";
	import { toast } from "$lib/stores/toast.svelte";
	import * as api from "$lib/api/client";

	let { username, onClose }: {
		username: string;
		onClose: () => void;
	} = $props();

	const profile = $derived(profileStore.forUser(username));

	const CLEAR_OPTIONS = [
		{ label: "Don't clear", minutes: 0 },
		{ label: "30 minutes", minutes: 30 },
		{ label: "1 hour", minutes: 60 },
		{ label: "4 hours", minutes: 240 },
		{ label: "24 hours", minutes: 1440 }
	];

	const MAX_LEN = 128;

	let statusDraft = $state(profile?.status_text ?? "");
	let clearMinutes = $state(0);
	let pickerOpen = $state(false);
	let saving = $state(false);

	async function save() {
		const token = session.token;
		if (!token) return;
		saving = true;
		try {
			const updated = await api.updateProfile(token, {
				status_text: statusDraft,
				status_clear_minutes: clearMinutes
			});
			profileStore.set(updated);
			onClose();
		} catch {
			toast.push("Couldn't save status");
		} finally {
			saving = false;
		}
	}

	async function clearStatus() {
		const token = session.token;
		if (!token) return;
		saving = true;
		try {
			const updated = await api.updateProfile(token, { status_text: "", status_clear_minutes: 0 });
			profileStore.set(updated);
			onClose();
		} catch {
			toast.push("Couldn't clear status");
		} finally {
			saving = false;
		}
	}
</script>

<Modal title="Set your status" {onClose} width={520}>
	<div class="preview">
		<div class="preview-banner" style:background={api.bannerBackground(profile, session.token)}></div>
		<div class="preview-avatar-row">
			<div class="preview-avatar" style:background-image={profile?.avatar_url ? `url(${api.resolveUrl(profile.avatar_url, session.token)})` : undefined}>
				{#if !profile?.avatar_url}{username.slice(0, 2).toUpperCase()}{/if}
			</div>
			{#if statusDraft.trim()}
				<div class="preview-bubble">{statusDraft}</div>
			{/if}
		</div>
		<p class="preview-name" style:color={profile?.accent_color || undefined}>{profile?.display_name || username}</p>
		<p class="preview-meta">
			<span>{username}</span>
			{#if profile?.pronouns}<span class="dot">•</span>{profile.pronouns}{/if}
			<Badges badges={badgeStore.forUser(username)} />
		</p>
	</div>

	<label class="field">
		Status
		<div class="status-input">
			<input type="text" bind:value={statusDraft} maxlength={MAX_LEN} placeholder="What's happening?" />
			{#if statusDraft}
				<button class="clear-input" onclick={() => (statusDraft = "")} title="Clear text">
					<X size={15} strokeWidth={2.5} />
				</button>
			{/if}
		</div>
		<span class="char-count">{statusDraft.length} / {MAX_LEN}</span>
	</label>

	<div class="clear-row">
		<div class="clear-select-wrapper">
			<button class="clear-select" onclick={() => (pickerOpen = !pickerOpen)}>
				{CLEAR_OPTIONS.find((o) => o.minutes === clearMinutes)?.label}
				<ChevronDown size={16} strokeWidth={2} />
			</button>
			{#if pickerOpen}
				<div class="clear-menu" use:clickOutside={() => (pickerOpen = false)}>
					{#each CLEAR_OPTIONS as option (option.minutes)}
						<button
							class="clear-option"
							class:active={clearMinutes === option.minutes}
							onclick={() => { clearMinutes = option.minutes; pickerOpen = false; }}
						>
							{option.label}
						</button>
					{/each}
				</div>
			{/if}
		</div>
		<button class="primary" onclick={save} disabled={saving}>{saving ? "Saving…" : "Save"}</button>
	</div>

	{#if profile?.status_text}
		<button class="ghost-clear" onclick={clearStatus} disabled={saving}>Clear current status</button>
	{/if}
</Modal>

<style>
	.preview {
		border-radius: 10px;
		overflow: hidden;
		background: var(--sidebar);
		margin-bottom: 18px;
	}

	.preview-banner {
		height: 80px;
		background: var(--active);
	}

	.preview-avatar-row {
		position: relative;
		margin: -32px 0 0 18px;
		width: fit-content;
	}

	.preview-avatar {
		width: 64px;
		height: 64px;
		border-radius: 50%;
		border: 4px solid var(--sidebar);
		background: var(--accent-fill) center/cover;
		color: var(--accent-fill-ink);
		display: flex;
		align-items: center;
		justify-content: center;
		font-family: var(--font-body);
		font-weight: 700;
		font-size: 18px;
	}

	.preview-bubble {
		position: absolute;
		left: 72px;
		bottom: 30px;
		max-width: 280px;
		background: var(--active);
		color: var(--ink);
		border-radius: 12px;
		border-bottom-left-radius: 4px;
		padding: 8px 12px;
		font-size: 14px;
		line-height: 1.4;
		box-shadow: 0 8px 20px rgba(0, 0, 0, 0.35);
	}

	.preview-name {
		margin: 12px 0 0 18px;
		font-family: var(--font-mono);
		font-weight: 700;
		font-size: 18px;
		color: var(--ink);
	}

	.preview-meta {
		margin: 3px 0 16px 18px;
		display: flex;
		align-items: center;
		gap: 6px;
		font-size: 13px;
		font-weight: 500;
		color: var(--ink-faint);
	}

	.dot {
		font-size: 10px;
	}

	.field {
		display: flex;
		flex-direction: column;
		gap: 8px;
		font-family: var(--font-mono);
		font-size: 12px;
		font-weight: 600;
		text-transform: uppercase;
		letter-spacing: 0.03em;
		color: var(--ink-dim);
	}

	.status-input {
		position: relative;
		display: flex;
	}

	.status-input input {
		flex: 1;
		background: var(--panel);
		border: 1px solid var(--hairline);
		border-radius: 8px;
		padding: 13px 40px 13px 14px;
		color: var(--ink);
		font-family: var(--font-body);
		font-size: 15px;
	}

	.status-input input:focus {
		outline: none;
		border-color: var(--ink-dim);
	}

	.char-count {
		align-self: flex-end;
		font-size: 11px;
		font-weight: 500;
		text-transform: none;
		letter-spacing: normal;
		color: var(--ink-faint);
	}

	.clear-input {
		position: absolute;
		right: 10px;
		top: 50%;
		transform: translateY(-50%);
		color: var(--ink-faint);
		display: flex;
	}

	.clear-input:hover {
		color: var(--ink);
	}

	.clear-row {
		display: flex;
		align-items: center;
		justify-content: space-between;
		margin-top: 18px;
	}

	.clear-select-wrapper {
		position: relative;
	}

	.clear-select {
		display: flex;
		align-items: center;
		gap: 8px;
		padding: 11px 16px;
		border-radius: 8px;
		background: var(--sidebar);
		color: var(--ink-dim);
		font-size: 14px;
		font-weight: 600;
	}

	.clear-select:hover {
		background: var(--hover);
		color: var(--ink);
	}

	.primary {
		padding: 11px 22px;
		border-radius: 8px;
		background: var(--accent-fill);
		color: var(--accent-fill-ink);
		font-weight: 700;
		font-size: 14px;
	}

	.primary:disabled {
		background: var(--active);
		color: var(--ink-faint);
	}

	.clear-menu {
		position: absolute;
		bottom: calc(100% + 6px);
		left: 0;
		background: var(--panel);
		border-radius: 10px;
		padding: 6px;
		min-width: 200px;
		box-shadow: 0 12px 32px rgba(0, 0, 0, 0.4);
		z-index: 60;
	}

	.clear-option {
		width: 100%;
		text-align: left;
		padding: 10px 12px;
		border-radius: 6px;
		font-size: 13px;
		color: var(--ink-dim);
	}

	.clear-option:hover {
		background: var(--hover);
		color: var(--ink);
	}

	.clear-option.active {
		color: var(--ink);
		font-weight: 700;
	}

	.ghost-clear {
		width: 100%;
		margin-top: 14px;
		padding: 11px;
		border-radius: 8px;
		color: var(--danger);
		font-size: 13px;
		font-weight: 600;
		text-align: center;
	}

	.ghost-clear:hover {
		background: rgba(216, 60, 62, 0.12);
	}
</style>

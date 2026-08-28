<script lang="ts">
	import Copy from "@lucide/svelte/icons/copy";
	import Check from "@lucide/svelte/icons/check";
	import Modal from "$lib/components/Modal.svelte";
	import { toast } from "$lib/stores/toast.svelte";
	import { session } from "$lib/stores/session.svelte";
	import { getServerInvite } from "$lib/api/client";

	let { serverName, serverId, onClose }: {
		serverName: string;
		serverId: string;
		onClose: () => void;
	} = $props();

	let copied = $state(false);
	let code = $state<string | null>(null);
	let failed = $state(false);
	const link = $derived(code ? `hollowchat.org/invite/${code}` : "");

	$effect(() => {
		const token = session.token;
		if (!token) return;
		getServerInvite(token, serverId)
			.then((res) => (code = res.code))
			.catch(() => (failed = true));
	});

	async function copy() {
		if (!link) return;
		await navigator.clipboard.writeText(link);
		copied = true;
		toast.push("Invite link copied");
		setTimeout(() => (copied = false), 1500);
	}
</script>

<Modal title={`Invite people to ${serverName}`} {onClose}>
	<p class="hint">Share this link. Anyone with it can join — it never expires.</p>
	{#if failed}
		<p class="hint">Couldn't create an invite link. Try again.</p>
	{:else}
		<div class="link-box">
			<span class="link">{code ? link : "Generating…"}</span>
			<button class="copy" onclick={copy} title="Copy link" disabled={!code}>
				{#if copied}<Check size={15} strokeWidth={2.5} />{:else}<Copy size={15} strokeWidth={2} />{/if}
			</button>
		</div>
	{/if}
</Modal>

<style>
	.hint {
		margin: 0 0 16px;
		font-size: 13px;
		color: var(--ink-dim);
		line-height: 1.5;
	}

	.link-box {
		display: flex;
		align-items: center;
		gap: 8px;
		background: var(--panel);
		border: 1px solid var(--hairline);
		border-radius: 6px;
		padding: 10px 10px 10px 14px;
	}

	.link {
		flex: 1;
		font-family: var(--font-mono);
		font-size: 13px;
		color: var(--ink);
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
	}

	.copy {
		flex-shrink: 0;
		padding: 8px 14px;
		border-radius: 6px;
		background: var(--accent-fill);
		color: var(--accent-fill-ink);
		display: flex;
		align-items: center;
	}

	.copy:disabled {
		opacity: 0.5;
	}
</style>

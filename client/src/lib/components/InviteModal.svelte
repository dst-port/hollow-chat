<script lang="ts">
	import Copy from "@lucide/svelte/icons/copy";
	import Check from "@lucide/svelte/icons/check";
	import Modal from "$lib/components/Modal.svelte";
	import { toast } from "$lib/stores/toast.svelte";

	let { serverName, inviteCode, onClose }: {
		serverName: string;
		inviteCode: string;
		onClose: () => void;
	} = $props();

	let copied = $state(false);
	const link = $derived(`hollowchat.app/invite/${inviteCode}`);

	async function copy() {
		await navigator.clipboard.writeText(link);
		copied = true;
		toast.push("Invite link copied");
		setTimeout(() => (copied = false), 1500);
	}
</script>

<Modal title={`Invite people to ${serverName}`} {onClose}>
	<p class="hint">Share this link. Anyone with it can join — it never expires.</p>
	<div class="link-box">
		<span class="link">{link}</span>
		<button class="copy" onclick={copy} title="Copy link">
			{#if copied}<Check size={15} strokeWidth={2.5} />{:else}<Copy size={15} strokeWidth={2} />{/if}
		</button>
	</div>
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
</style>

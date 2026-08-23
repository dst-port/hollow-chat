<script lang="ts">
	import Hash from "@lucide/svelte/icons/hash";
	import Volume2 from "@lucide/svelte/icons/volume-2";
	import Modal from "$lib/components/Modal.svelte";
	import type { ChannelType } from "$lib/data/mock";

	let { onClose, onCreate }: {
		onClose: () => void;
		onCreate: (name: string, type: ChannelType) => void;
	} = $props();

	let name = $state("");
	let type = $state<ChannelType>("text");

	function submit(event: SubmitEvent) {
		event.preventDefault();
		const trimmed = name.trim();
		if (!trimmed) return;
		onCreate(trimmed, type);
	}
</script>

<Modal title="Create channel" {onClose}>
	<form onsubmit={submit}>
		<div class="type-picker">
			<button type="button" class="type" class:active={type === "text"} onclick={() => (type = "text")}>
				<Hash size={18} strokeWidth={2} />
				Text
			</button>
			<button type="button" class="type" class:active={type === "voice"} onclick={() => (type = "voice")}>
				<Volume2 size={18} strokeWidth={2} />
				Voice
			</button>
		</div>

		<label>
			Channel name
			<div class="input-wrap">
				{#if type === "text"}<Hash size={16} strokeWidth={2} />{:else}<Volume2 size={16} strokeWidth={2} />{/if}
				<input type="text" bind:value={name} required maxlength="32" placeholder="new-channel" />
			</div>
		</label>

		<button type="submit" disabled={!name.trim()}>Create Channel</button>
	</form>
</Modal>

<style>
	form {
		display: flex;
		flex-direction: column;
		gap: 16px;
	}

	.type-picker {
		display: flex;
		gap: 8px;
	}

	.type {
		flex: 1;
		display: flex;
		align-items: center;
		justify-content: center;
		gap: 8px;
		padding: 10px;
		border-radius: 6px;
		background: var(--panel);
		color: var(--ink-dim);
		font-size: 13px;
		font-weight: 600;
		transition: background-color 0.15s ease, color 0.15s ease;
	}

	.type.active {
		background: var(--active);
		color: var(--ink);
	}

	label {
		display: flex;
		flex-direction: column;
		gap: 6px;
		font-family: var(--font-mono);
		font-size: 11px;
		font-weight: 600;
		text-transform: uppercase;
		letter-spacing: 0.03em;
		color: var(--ink-dim);
	}

	.input-wrap {
		display: flex;
		align-items: center;
		gap: 8px;
		background: var(--panel);
		border: 1px solid var(--hairline);
		border-radius: 6px;
		padding: 0 12px;
		color: var(--ink-faint);
	}

	.input-wrap input {
		flex: 1;
		background: none;
		border: none;
		padding: 10px 0;
		color: var(--ink);
		font-family: var(--font-body);
		font-size: 14px;
	}

	button[type="submit"] {
		padding: 10px;
		border-radius: 6px;
		background: var(--accent-fill);
		color: var(--accent-fill-ink);
		font-weight: 700;
		font-size: 14px;
	}

	button[type="submit"]:disabled {
		background: var(--active);
		color: var(--ink-faint);
	}
</style>

<script lang="ts">
	import { fly } from "svelte/transition";
	import Copy from "@lucide/svelte/icons/copy";
	import Pin from "@lucide/svelte/icons/pin";
	import PinOff from "@lucide/svelte/icons/pin-off";
	import Trash2 from "@lucide/svelte/icons/trash-2";
	import { clickOutside } from "$lib/actions/clickOutside";

	let { pinned, onClose, onCopy, onTogglePin, onDelete }: {
		pinned: boolean;
		onClose: () => void;
		onCopy: () => void;
		onTogglePin: () => void;
		onDelete: () => void;
	} = $props();
</script>

<div class="menu" use:clickOutside={onClose} transition:fly={{ y: -4, duration: 120 }}>
	<button
		class="item"
		onclick={() => {
			onCopy();
			onClose();
		}}
	>
		<Copy size={14} strokeWidth={2} />
		Copy Text
	</button>
	<button
		class="item"
		onclick={() => {
			onTogglePin();
			onClose();
		}}
	>
		{#if pinned}<PinOff size={14} strokeWidth={2} />{:else}<Pin size={14} strokeWidth={2} />{/if}
		{pinned ? "Unpin Message" : "Pin Message"}
	</button>
	<button
		class="item danger"
		onclick={() => {
			onDelete();
			onClose();
		}}
	>
		<Trash2 size={14} strokeWidth={2} />
		Delete Message
	</button>
</div>

<style>
	.menu {
		position: absolute;
		top: -8px;
		right: 40px;
		background: var(--panel);
		border-radius: 8px;
		padding: 6px;
		min-width: 170px;
		box-shadow: 0 12px 32px rgba(0, 0, 0, 0.4);
		z-index: 60;
	}

	.item {
		width: 100%;
		display: flex;
		align-items: center;
		gap: 8px;
		padding: 8px 10px;
		border-radius: 6px;
		font-size: 13px;
		font-weight: 500;
		color: var(--ink-dim);
		transition: background-color 0.15s ease, color 0.15s ease;
	}

	.item:hover {
		background: var(--hover);
		color: var(--ink);
	}

	.item.danger {
		color: var(--danger);
	}

	.item.danger:hover {
		background: rgba(216, 60, 62, 0.12);
		color: var(--danger);
	}
</style>

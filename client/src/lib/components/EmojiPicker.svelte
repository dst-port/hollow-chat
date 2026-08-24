<script lang="ts">
	import { fly } from "svelte/transition";
	import { clickOutside } from "$lib/actions/clickOutside";
	import { emojify } from "$lib/actions/emojify";
	import { EMOJI_PALETTE } from "$lib/data/mock";

	let { onClose, onPick }: {
		onClose: () => void;
		onPick: (emoji: string) => void;
	} = $props();
</script>

<div class="picker" use:clickOutside={onClose} transition:fly={{ y: 6, duration: 140 }}>
	{#each EMOJI_PALETTE as emoji (emoji)}
		<button
			class="emoji"
			use:emojify
			onclick={() => {
				onPick(emoji);
				onClose();
			}}
		>
			{emoji}
		</button>
	{/each}
</div>

<style>
	.picker {
		position: absolute;
		bottom: calc(100% + 8px);
		right: 0;
		display: flex;
		gap: 4px;
		background: var(--panel);
		border-radius: 8px;
		padding: 6px;
		box-shadow: 0 12px 32px rgba(0, 0, 0, 0.4);
		z-index: 60;
	}

	.emoji {
		font-size: 18px;
		padding: 6px;
		border-radius: 6px;
		transition: background-color 0.15s ease;
	}

	.emoji:hover {
		background: var(--hover);
	}
</style>

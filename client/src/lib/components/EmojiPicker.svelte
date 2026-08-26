<script lang="ts">
	import { fly } from "svelte/transition";
	import { clickOutside } from "$lib/actions/clickOutside";
	import { emojify } from "$lib/actions/emojify";
	import { EMOJI_PALETTE } from "$lib/data/mock";
	import { resolveUrl, type CustomEmoji } from "$lib/api/client";
	import { session } from "$lib/stores/session.svelte";

	let { onClose, onPick, customEmoji = [] }: {
		onClose: () => void;
		onPick: (emoji: string) => void;
		customEmoji?: CustomEmoji[];
	} = $props();
</script>

<div class="picker" use:clickOutside={onClose} transition:fly={{ y: 6, duration: 140 }}>
	<div class="row">
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
	{#if customEmoji.length > 0}
		<div class="divider"></div>
		<div class="row">
			{#each customEmoji as item (item.id)}
				<button
					class="emoji custom"
					title={`:${item.name}:`}
					onclick={() => {
						onPick(`:${item.name}:`);
						onClose();
					}}
				>
					<img src={resolveUrl(item.image_url, session.token)} alt={item.name} />
				</button>
			{/each}
		</div>
	{/if}
</div>

<style>
	.picker {
		position: absolute;
		bottom: calc(100% + 8px);
		right: 0;
		display: flex;
		flex-direction: column;
		gap: 6px;
		max-width: 260px;
		background: var(--panel);
		border-radius: 8px;
		padding: 6px;
		box-shadow: 0 12px 32px rgba(0, 0, 0, 0.4);
		z-index: 60;
	}

	.row {
		display: flex;
		flex-wrap: wrap;
		gap: 4px;
	}

	.divider {
		height: 1px;
		background: var(--hairline);
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

	.emoji.custom img {
		width: 20px;
		height: 20px;
		object-fit: contain;
		display: block;
	}
</style>

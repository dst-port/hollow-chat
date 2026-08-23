<script lang="ts">
	import { fly, fade } from "svelte/transition";
	import { toast } from "$lib/stores/toast.svelte";
</script>

<div class="host">
	{#each toast.items as item (item.id)}
		<div class="toast" in:fly={{ y: 12, duration: 180 }} out:fade={{ duration: 120 }}>
			{item.message}
		</div>
	{/each}
</div>

<style>
	.host {
		position: fixed;
		bottom: 24px;
		left: 50%;
		transform: translateX(-50%);
		display: flex;
		flex-direction: column;
		align-items: center;
		gap: 8px;
		z-index: 300;
		pointer-events: none;
	}

	.toast {
		background: var(--active);
		color: var(--ink);
		font-size: 13px;
		font-weight: 500;
		padding: 10px 16px;
		border-radius: 8px;
		box-shadow: 0 8px 24px rgba(0, 0, 0, 0.4);
	}
</style>

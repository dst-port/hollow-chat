<script lang="ts">
	import type { Snippet } from "svelte";
	import { fly } from "svelte/transition";
	import { clickOutside } from "$lib/actions/clickOutside";

	let { title, onClose, children }: {
		title: string;
		onClose: () => void;
		children: Snippet;
	} = $props();
</script>

<div class="popover" use:clickOutside={onClose} transition:fly={{ y: -6, duration: 140 }}>
	<p class="title">{title}</p>
	{@render children()}
</div>

<style>
	.popover {
		position: absolute;
		top: calc(100% + 8px);
		right: 0;
		width: 240px;
		background: var(--panel);
		border-radius: 8px;
		padding: 14px;
		box-shadow: 0 16px 40px rgba(0, 0, 0, 0.45);
		z-index: 60;
	}

	.title {
		margin: 0 0 8px;
		font-family: var(--font-mono);
		font-size: 11px;
		font-weight: 600;
		text-transform: uppercase;
		letter-spacing: 0.04em;
		color: var(--ink-faint);
	}
</style>

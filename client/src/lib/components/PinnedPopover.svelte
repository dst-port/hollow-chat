<script lang="ts">
	import { fly } from "svelte/transition";
	import { clickOutside } from "$lib/actions/clickOutside";
	import type { Message } from "$lib/data/mock";
	import { t } from "$lib/i18n/index.svelte";

	let { pinned, onClose }: {
		pinned: Message[];
		onClose: () => void;
	} = $props();
</script>

<div class="popover" use:clickOutside={onClose} transition:fly={{ y: -6, duration: 140 }}>
	<p class="title">{t("pins.title")}</p>
	{#if pinned.length === 0}
		<p class="empty">{t("pins.empty")}</p>
	{:else}
		{#each pinned as message (message.id)}
			<div class="pinned-item">
				<span class="author" style:color={message.color}>{message.author}</span>
				<p class="content">{message.content}</p>
			</div>
		{/each}
	{/if}
</div>

<style>
	.popover {
		position: absolute;
		top: calc(100% + 8px);
		right: 120px;
		width: 280px;
		max-height: 360px;
		overflow-y: auto;
		background: var(--panel);
		border-radius: 8px;
		padding: 14px;
		box-shadow: 0 16px 40px rgba(0, 0, 0, 0.45);
		z-index: 60;
	}

	.title {
		margin: 0 0 10px;
		font-family: var(--font-mono);
		font-size: 11px;
		font-weight: 600;
		text-transform: uppercase;
		letter-spacing: 0.04em;
		color: var(--ink-faint);
	}

	.empty {
		margin: 0;
		font-size: 13px;
		color: var(--ink-dim);
		line-height: 1.5;
	}

	.pinned-item {
		padding: 8px 0;
		border-top: 1px solid var(--hairline);
	}

	.pinned-item:first-of-type {
		border-top: none;
	}

	.author {
		font-family: var(--font-mono);
		font-weight: 600;
		font-size: 12px;
	}

	.content {
		margin: 2px 0 0;
		font-size: 13px;
		color: var(--ink);
	}
</style>

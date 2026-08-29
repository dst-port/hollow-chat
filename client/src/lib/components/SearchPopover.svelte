<script lang="ts">
	import { fly } from "svelte/transition";
	import { clickOutside } from "$lib/actions/clickOutside";
	import type { Message } from "$lib/data/mock";
	import { t } from "$lib/i18n/index.svelte";

	let { results, query, loading, exhausted, onClose }: {
		results: Message[];
		query: string;
		loading: boolean;
		exhausted: boolean;
		onClose: () => void;
	} = $props();

	function formatTime(message: Message): string {
		return message.time ?? "";
	}
</script>

<div class="popover" use:clickOutside={onClose} transition:fly={{ y: -6, duration: 140 }}>
	<p class="title">{t("search.results")}</p>
	{#if !query.trim()}
		<p class="empty">{t("search.typePrompt")}</p>
	{:else if results.length === 0 && loading}
		<p class="empty">Searching…</p>
	{:else if results.length === 0}
		<p class="empty">No messages matched "{query}".</p>
	{:else}
		{#each results as message (message.id)}
			<div class="result-item">
				<p class="meta">
					<span class="author" style:color={message.color}>{message.author}</span>
					<span class="time">{formatTime(message)}</span>
				</p>
				<p class="content">{message.content}</p>
			</div>
		{/each}
		{#if loading}
			<p class="empty">Searching further back…</p>
		{:else if !exhausted}
			<p class="empty">{t("search.recentHint")}</p>
		{/if}
	{/if}
</div>

<style>
	.popover {
		position: absolute;
		top: calc(100% + 8px);
		right: 0;
		width: 320px;
		max-height: 420px;
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

	.result-item {
		padding: 8px 0;
		border-top: 1px solid var(--hairline);
	}

	.result-item:first-of-type {
		border-top: none;
	}

	.meta {
		margin: 0;
		display: flex;
		align-items: baseline;
		gap: 6px;
	}

	.author {
		font-family: var(--font-mono);
		font-weight: 600;
		font-size: 12px;
	}

	.time {
		font-size: 10px;
		color: var(--ink-faint);
	}

	.content {
		margin: 2px 0 0;
		font-size: 13px;
		color: var(--ink);
		overflow-wrap: break-word;
	}
</style>

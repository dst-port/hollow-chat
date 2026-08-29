<script lang="ts">
	import "$lib/app.css";
	import ToastHost from "$lib/components/ToastHost.svelte";
	import TitleBar from "$lib/components/TitleBar.svelte";
	import ResizeHandles from "$lib/components/ResizeHandles.svelte";
	import UpdateBanner from "$lib/components/UpdateBanner.svelte";
	import { themeStore } from "$lib/stores/theme.svelte";
	import { fontStore } from "$lib/stores/font.svelte";
	import { i18n } from "$lib/i18n/index.svelte";
	import { viewport } from "$lib/stores/viewport.svelte";
	import { initAutoUpdateCheck } from "$lib/stores/updater.svelte";
	import { isTauri } from "$lib/utils/isTauri";

	let { children } = $props();

	const inTauri = isTauri();

	i18n.init();
	themeStore.init();
	fontStore.init();
	viewport.init();
	if (inTauri) initAutoUpdateCheck();

	function blockNativeContextMenu(event: MouseEvent) {
		const target = event.target as HTMLElement | null;
		const editable = target?.closest("input, textarea, [contenteditable='true']");
		if (editable) return;
		event.preventDefault();
	}
</script>

<svelte:window oncontextmenu={blockNativeContextMenu} />

<div class="app-shell" class:web={!inTauri}>
	{#if inTauri}<TitleBar />{/if}
	<div class="app-content">
		{@render children()}
	</div>
</div>
{#if inTauri}<ResizeHandles />{/if}
<ToastHost />
{#if inTauri}<UpdateBanner />{/if}

<style>
	.app-shell {
		display: flex;
		flex-direction: column;
		height: 100%;
		width: 100%;
		border-radius: 12px;
		overflow: hidden;
		background: var(--void);
	}

	.app-shell.web {
		border-radius: 0;
	}

	.app-content {
		flex: 1;
		min-height: 0;
	}
</style>

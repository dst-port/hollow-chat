<script lang="ts">
	import "$lib/app.css";
	import ToastHost from "$lib/components/ToastHost.svelte";
	import TitleBar from "$lib/components/TitleBar.svelte";
	import ResizeHandles from "$lib/components/ResizeHandles.svelte";

	let { children } = $props();

	function blockNativeContextMenu(event: MouseEvent) {
		const target = event.target as HTMLElement | null;
		const editable = target?.closest("input, textarea, [contenteditable='true']");
		if (editable) return;
		event.preventDefault();
	}
</script>

<svelte:window oncontextmenu={blockNativeContextMenu} />

<div class="app-shell">
	<TitleBar />
	<div class="app-content">
		{@render children()}
	</div>
</div>
<ResizeHandles />
<ToastHost />

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

	.app-content {
		flex: 1;
		min-height: 0;
	}
</style>

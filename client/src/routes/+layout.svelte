<script lang="ts">
	import "$lib/app.css";
	import ToastHost from "$lib/components/ToastHost.svelte";

	let { children } = $props();

	function blockNativeContextMenu(event: MouseEvent) {
		const target = event.target as HTMLElement | null;
		const editable = target?.closest("input, textarea, [contenteditable='true']");
		if (editable) return;
		event.preventDefault();
	}
</script>

<svelte:window oncontextmenu={blockNativeContextMenu} />

{@render children()}
<ToastHost />

<script lang="ts">
	import "$lib/app.css";
	import ToastHost from "$lib/components/ToastHost.svelte";

	let { children } = $props();
</script>

{@render children()}
<ToastHost />

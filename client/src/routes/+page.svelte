<script lang="ts">
	import AuthView from "$lib/components/AuthView.svelte";
	import ChatShell from "$lib/components/ChatShell.svelte";
	import { session } from "$lib/stores/session.svelte";
</script>

{#if !session.ready}
	<div class="loading">Loading…</div>
{:else if session.isAuthenticated}
	<ChatShell />
{:else}
	<AuthView />
{/if}

<style>
	.loading {
		height: 100vh;
		width: 100vw;
		display: flex;
		align-items: center;
		justify-content: center;
		color: var(--text-muted);
	}
</style>

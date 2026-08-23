<script lang="ts">
	import { fade } from "svelte/transition";
	import MessageCircle from "@lucide/svelte/icons/message-circle";
	import UserBar from "$lib/components/UserBar.svelte";

	let { username, onLogout }: {
		username: string;
		onLogout: () => void;
	} = $props();
</script>

<div class="home">
	<aside class="dm-list">
		<div class="search-bar">
			<input type="text" placeholder="Find or start a conversation" />
		</div>
		<p class="label">Direct Messages</p>
		<div class="spacer"></div>
		<UserBar {username} {onLogout} />
	</aside>

	<div class="empty" in:fade={{ duration: 200 }}>
		<MessageCircle size={40} strokeWidth={1.5} />
		<h2>No conversations yet</h2>
		<p>Start a direct message with someone to see it here.</p>
	</div>
</div>

<style>
	.home {
		flex: 1;
		display: flex;
		height: 100%;
		min-width: 0;
	}

	.dm-list {
		width: 240px;
		flex-shrink: 0;
		background: var(--sidebar);
		padding-top: 12px;
		display: flex;
		flex-direction: column;
	}

	.dm-list .search-bar {
		margin-left: 8px;
		margin-right: 8px;
	}

	.spacer {
		flex: 1;
	}

	.search-bar {
		margin-bottom: 12px;
	}

	.search-bar input {
		width: 100%;
		background: var(--panel);
		border: none;
		border-radius: 6px;
		padding: 8px 10px;
		font-size: 13px;
		color: var(--ink);
	}

	.search-bar input::placeholder {
		color: var(--ink-faint);
	}

	.label {
		margin: 8px 8px 4px;
		font-family: var(--font-mono);
		font-size: 11px;
		font-weight: 600;
		text-transform: uppercase;
		letter-spacing: 0.04em;
		color: var(--ink-faint);
	}

	.empty {
		flex: 1;
		display: flex;
		flex-direction: column;
		align-items: center;
		justify-content: center;
		gap: 8px;
		color: var(--ink-faint);
		background: var(--panel);
	}

	.empty h2 {
		margin: 8px 0 0;
		font-family: var(--font-display);
		font-size: 16px;
		color: var(--ink-dim);
	}

	.empty p {
		margin: 0;
		font-size: 13px;
	}
</style>

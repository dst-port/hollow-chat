<script lang="ts">
	import Plus from "@lucide/svelte/icons/plus";
	import type { ServerEntry } from "$lib/data/mock";

	let { servers, activeId, onSelect }: {
		servers: ServerEntry[];
		activeId: string;
		onSelect: (id: string) => void;
	} = $props();
</script>

<nav class="rail">
	<button class="home" title="Direct Messages">HC</button>
	<div class="divider"></div>

	<ul>
		{#each servers as server (server.id)}
			<li>
				<button
					class="server"
					class:active={server.id === activeId}
					title={server.name}
					onclick={() => onSelect(server.id)}
				>
					<span
						class="pill"
						class:active={server.id === activeId}
						class:unread={!!server.unread && server.id !== activeId}
					></span>
					<span class="icon">{server.initials}</span>
					{#if server.unread}
						<span class="badge">{server.unread > 9 ? "9+" : server.unread}</span>
					{/if}
				</button>
			</li>
		{/each}
	</ul>

	<div class="spacer"></div>

	<button class="add" title="Add a server">
		<Plus size={20} strokeWidth={2.25} />
	</button>
</nav>

<style>
	.rail {
		width: 72px;
		flex-shrink: 0;
		background: var(--rail);
		display: flex;
		flex-direction: column;
		align-items: center;
		padding: 12px 0;
		gap: 8px;
		height: 100%;
	}

	.home {
		width: 48px;
		height: 48px;
		border-radius: 16px;
		background: var(--wraith-soft);
		color: var(--wraith);
		font-family: var(--font-display);
		font-weight: 700;
		font-size: 12px;
		display: flex;
		align-items: center;
		justify-content: center;
	}

	.divider {
		width: 32px;
		height: 2px;
		background: var(--hairline);
		border-radius: 1px;
		margin: 4px 0;
	}

	ul {
		list-style: none;
		margin: 0;
		padding: 0;
		display: flex;
		flex-direction: column;
		gap: 8px;
		width: 100%;
		align-items: center;
		overflow-y: auto;
	}

	.server {
		position: relative;
		width: 48px;
		height: 48px;
		border-radius: 24px;
		background: var(--sidebar);
		color: var(--ink-dim);
		font-family: var(--font-mono);
		font-size: 11px;
		font-weight: 600;
		display: flex;
		align-items: center;
		justify-content: center;
		transition: border-radius 0.15s ease, background 0.15s ease, color 0.15s ease;
	}

	.server:hover {
		border-radius: 16px;
		background: var(--wraith);
		color: var(--void);
	}

	.server.active {
		border-radius: 16px;
		background: var(--wraith);
		color: var(--void);
	}

	.pill {
		position: absolute;
		left: -12px;
		width: 4px;
		height: 8px;
		border-radius: 0 4px 4px 0;
		background: var(--ink);
		box-shadow: 0 0 8px 1px var(--ink);
		transition: height 0.15s ease;
	}

	.server:hover .pill {
		height: 20px;
	}

	.pill.active {
		height: 36px;
		background: var(--wraith);
		box-shadow: 0 0 10px 2px var(--wraith);
	}

	.pill.unread {
		height: 10px;
		background: var(--ember);
		box-shadow: 0 0 6px 1px var(--ember);
	}

	.badge {
		position: absolute;
		bottom: -4px;
		right: -4px;
		min-width: 18px;
		height: 18px;
		padding: 0 4px;
		border-radius: 9px;
		background: var(--danger);
		color: white;
		font-family: var(--font-mono);
		font-size: 10px;
		font-weight: 600;
		display: flex;
		align-items: center;
		justify-content: center;
		border: 3px solid var(--rail);
	}

	.spacer {
		flex: 1;
	}

	.add {
		width: 48px;
		height: 48px;
		border-radius: 24px;
		background: var(--sidebar);
		color: var(--ember);
		display: flex;
		align-items: center;
		justify-content: center;
		transition: border-radius 0.15s ease, background 0.15s ease, color 0.15s ease;
	}

	.add:hover {
		border-radius: 16px;
		background: var(--ember);
		color: var(--void);
	}
</style>

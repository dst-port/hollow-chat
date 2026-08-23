<script lang="ts">
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

	<button class="add" title="Add a server">+</button>
</nav>

<style>
	.rail {
		width: 72px;
		flex-shrink: 0;
		background: var(--bg-rail);
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
		background: var(--accent-soft);
		color: var(--accent);
		font-weight: 700;
		font-size: 13px;
		display: flex;
		align-items: center;
		justify-content: center;
	}

	.divider {
		width: 32px;
		height: 2px;
		background: var(--border);
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
		background: var(--bg-sidebar);
		color: var(--text-muted);
		font-size: 12px;
		font-weight: 600;
		display: flex;
		align-items: center;
		justify-content: center;
		transition: border-radius 0.15s ease, background 0.15s ease, color 0.15s ease;
	}

	.server:hover {
		border-radius: 16px;
		background: var(--accent);
		color: white;
	}

	.server.active {
		border-radius: 16px;
		background: var(--accent);
		color: white;
	}

	.pill {
		position: absolute;
		left: -12px;
		width: 4px;
		height: 8px;
		border-radius: 0 4px 4px 0;
		background: var(--text-primary);
		transition: height 0.15s ease;
	}

	.server:hover .pill {
		height: 20px;
	}

	.pill.active {
		height: 36px;
	}

	.pill.unread {
		height: 10px;
		background: var(--text-primary);
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
		font-size: 10px;
		font-weight: 700;
		display: flex;
		align-items: center;
		justify-content: center;
		border: 3px solid var(--bg-rail);
	}

	.spacer {
		flex: 1;
	}

	.add {
		width: 48px;
		height: 48px;
		border-radius: 24px;
		background: var(--bg-sidebar);
		color: var(--online);
		font-size: 22px;
		display: flex;
		align-items: center;
		justify-content: center;
		transition: border-radius 0.15s ease, background 0.15s ease;
	}

	.add:hover {
		border-radius: 16px;
		background: var(--online);
		color: var(--bg-rail);
	}
</style>

<script lang="ts">
	import { fade } from "svelte/transition";
	import Plus from "@lucide/svelte/icons/plus";
	import type { ServerEntry } from "$lib/data/mock";

	let { servers, activeId, onSelect, onSelectHome, onAddServer }: {
		servers: ServerEntry[];
		activeId: string | null;
		onSelect: (id: string) => void;
		onSelectHome: () => void;
		onAddServer: () => void;
	} = $props();

	let hovered = $state<{ label: string; anchor: HTMLElement } | null>(null);

	function show(label: string) {
		return (event: MouseEvent | FocusEvent) => {
			hovered = { label, anchor: event.currentTarget as HTMLElement };
		};
	}

	function hide() {
		hovered = null;
	}

	function tooltipPosition(anchor: HTMLElement) {
		const frame = document.querySelector(".window-frame");
		const frameRect = frame ? frame.getBoundingClientRect() : { top: 0, left: 0 };
		const anchorRect = anchor.getBoundingClientRect();

		return {
			top: anchorRect.top - frameRect.top + anchorRect.height / 2,
			left: anchorRect.right - frameRect.left + 14
		};
	}
</script>

<nav class="rail">
	<button
		class="home"
		class:active={activeId === null}
		aria-label="Direct Messages"
		onclick={onSelectHome}
		onmouseenter={show("Direct Messages")}
		onmouseleave={hide}
		onfocus={show("Direct Messages")}
		onblur={hide}
	>
		<span class="pill" class:active={activeId === null}></span>
		<img class="mark" src="/logo/hollowchat-mark.png" alt="" />
	</button>
	<div class="divider"></div>

	<ul>
		{#each servers as server (server.id)}
			<li>
				<button
					class="server"
					class:active={server.id === activeId}
					aria-label={server.name}
					onclick={() => onSelect(server.id)}
					onmouseenter={show(server.name)}
					onmouseleave={hide}
					onfocus={show(server.name)}
					onblur={hide}
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

	<button
		class="add"
		aria-label="Add a server"
		onclick={onAddServer}
		onmouseenter={show("Add a Server")}
		onmouseleave={hide}
		onfocus={show("Add a Server")}
		onblur={hide}
	>
		<Plus size={20} strokeWidth={2.25} />
	</button>
</nav>

{#if hovered}
	{@const pos = tooltipPosition(hovered.anchor)}
	<span
		class="rail-tooltip"
		style:top={`${pos.top}px`}
		style:left={`${pos.left}px`}
		transition:fade={{ duration: 100 }}
	>
		{hovered.label}
	</span>
{/if}

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
		position: relative;
		width: 48px;
		height: 48px;
		border-radius: 16px;
		background: var(--accent-soft);
		color: var(--ink);
		font-family: var(--font-body);
		font-weight: 700;
		font-size: 12px;
		display: flex;
		align-items: center;
		justify-content: center;
		transition: background-color 0.15s ease, color 0.15s ease;
	}

	.home:hover,
	.home.active {
		background: var(--accent-fill);
		color: var(--accent-fill-ink);
	}

	.mark {
		width: 26px;
		height: 26px;
		object-fit: contain;
		pointer-events: none;
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
		background: var(--accent-fill);
		color: var(--accent-fill-ink);
	}

	.server.active {
		border-radius: 16px;
		background: var(--accent-fill);
		color: var(--accent-fill-ink);
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
		background: var(--ink);
		box-shadow: 0 0 8px 1px var(--ink);
	}

	.pill.unread {
		height: 10px;
		background: var(--ink);
		box-shadow: 0 0 6px 1px var(--ink);
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
		color: var(--online);
		display: flex;
		align-items: center;
		justify-content: center;
		transition: border-radius 0.15s ease, background 0.15s ease, color 0.15s ease;
	}

	.add:hover {
		border-radius: 16px;
		background: var(--online);
		color: var(--void);
	}

	.rail-tooltip {
		position: fixed;
		transform: translateY(-50%);
		padding: 8px 12px;
		border-radius: 6px;
		background: var(--void);
		color: var(--ink);
		font-family: var(--font-body);
		font-weight: 700;
		font-size: 13px;
		white-space: nowrap;
		pointer-events: none;
		box-shadow: 0 6px 18px rgba(0, 0, 0, 0.4);
		z-index: 200;
	}

	.rail-tooltip::before {
		content: "";
		position: absolute;
		right: 100%;
		top: 50%;
		transform: translateY(-50%);
		border: 6px solid transparent;
		border-right-color: var(--void);
	}
</style>

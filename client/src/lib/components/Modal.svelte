<script lang="ts">
	import type { Snippet } from "svelte";
	import { fade, scale } from "svelte/transition";
	import { cubicOut } from "svelte/easing";
	import X from "@lucide/svelte/icons/x";

	let { title, onClose, width = 440, children }: {
		title: string;
		onClose: () => void;
		width?: number;
		children: Snippet;
	} = $props();

	function onKeydown(event: KeyboardEvent) {
		if (event.key === "Escape") onClose();
	}
</script>

<svelte:window onkeydown={onKeydown} />

<div class="overlay" role="presentation" onclick={onClose} transition:fade={{ duration: 150 }}>
	<div
		class="modal"
		role="dialog"
		aria-modal="true"
		aria-label={title}
		tabindex="-1"
		style:width={`${width}px`}
		onclick={(e) => e.stopPropagation()}
		transition:scale={{ duration: 180, start: 0.96, easing: cubicOut }}
	>
		<div class="header">
			<h2>{title}</h2>
			<button class="close" onclick={onClose} title="Close">
				<X size={18} strokeWidth={2} />
			</button>
		</div>
		<div class="body">
			{@render children()}
		</div>
	</div>
</div>

<style>
	.overlay {
		position: fixed;
		inset: 0;
		background: rgba(0, 0, 0, 0.6);
		display: flex;
		align-items: center;
		justify-content: center;
		z-index: 200;
	}

	.modal {
		background: var(--void);
		border-radius: 10px;
		overflow: hidden;
		box-shadow: 0 20px 60px rgba(0, 0, 0, 0.5);
	}

	.header {
		display: flex;
		align-items: center;
		justify-content: space-between;
		padding: 18px 20px;
		border-bottom: 1px solid rgba(255, 255, 255, 0.12);
	}

	.header h2 {
		margin: 0;
		font-family: var(--font-body);
		font-weight: 700;
		font-size: 16px;
	}

	.close {
		padding: 6px;
		border-radius: 6px;
		color: var(--ink-dim);
		transition: background-color 0.15s ease, color 0.15s ease;
	}

	.close:hover {
		background: var(--hover);
		color: var(--ink);
	}

	.body {
		padding: 20px;
	}
</style>

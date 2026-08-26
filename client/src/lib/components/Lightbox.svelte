<script lang="ts">
	import { fade, scale } from "svelte/transition";
	import { cubicOut } from "svelte/easing";
	import X from "@lucide/svelte/icons/x";
	import Download from "@lucide/svelte/icons/download";

	let { src, kind, alt, onClose, onDownload }: {
		src: string;
		kind: "image" | "video";
		alt?: string;
		onClose: () => void;
		onDownload?: () => void;
	} = $props();

	function onKeydown(event: KeyboardEvent) {
		if (event.key === "Escape") onClose();
	}
</script>

<svelte:window onkeydown={onKeydown} />

<div class="overlay" role="presentation" onclick={onClose} transition:fade={{ duration: 150 }}>
	<div class="toolbar" onclick={(e) => e.stopPropagation()}>
		{#if onDownload}
			<button class="icon-btn" title="Download" onclick={onDownload}>
				<Download size={20} strokeWidth={2} />
			</button>
		{/if}
		<button class="icon-btn" title="Close" onclick={onClose}>
			<X size={22} strokeWidth={2} />
		</button>
	</div>

	<div class="frame" onclick={(e) => e.stopPropagation()} transition:scale={{ duration: 180, start: 0.96, easing: cubicOut }}>
		{#if kind === "image"}
			<img {src} alt={alt ?? ""} />
		{:else}
			<video {src} controls autoplay></video>
		{/if}
	</div>
</div>

<style>
	.overlay {
		position: fixed;
		inset: 0;
		background: rgba(0, 0, 0, 0.85);
		display: flex;
		align-items: center;
		justify-content: center;
		z-index: 400;
	}

	.toolbar {
		position: absolute;
		top: 16px;
		right: 16px;
		display: flex;
		gap: 8px;
		z-index: 1;
	}

	.icon-btn {
		display: flex;
		align-items: center;
		justify-content: center;
		width: 38px;
		height: 38px;
		border-radius: 50%;
		background: rgba(255, 255, 255, 0.12);
		color: white;
		transition: background-color 0.15s ease;
	}

	.icon-btn:hover {
		background: rgba(255, 255, 255, 0.24);
	}

	.frame {
		max-width: 90vw;
		max-height: 90vh;
		display: flex;
	}

	.frame img,
	.frame video {
		max-width: 90vw;
		max-height: 90vh;
		object-fit: contain;
		border-radius: 4px;
	}
</style>

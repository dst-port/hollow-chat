<script lang="ts">
	import { onMount } from "svelte";
	import { getCurrentWindow } from "@tauri-apps/api/window";
	import { platform } from "@tauri-apps/plugin-os";
	import Minus from "@lucide/svelte/icons/minus";
	import Square from "@lucide/svelte/icons/square";
	import X from "@lucide/svelte/icons/x";
	import { t } from "$lib/i18n/index.svelte";

	const appWindow = getCurrentWindow();
	let os = $state("windows");

	onMount(async () => {
		os = await platform();
	});

	function minimize() {
		appWindow.minimize();
	}

	function toggleMaximize() {
		appWindow.toggleMaximize();
	}

	function close() {
		appWindow.close();
	}

	function startDrag(event: MouseEvent) {
		if (event.button !== 0) return;
		if (event.detail === 2) {
			toggleMaximize();
			return;
		}
		appWindow.startDragging();
	}

	function stopDrag(event: MouseEvent) {
		event.stopPropagation();
	}
</script>

<div class="titlebar" class:mac={os === "macos"} onmousedown={startDrag}>
	{#if os === "macos"}
		<div class="traffic-lights">
			<button class="tl close" onmousedown={stopDrag} onclick={close} aria-label={t("common.close")}></button>
			<button class="tl minimize" onmousedown={stopDrag} onclick={minimize} aria-label={t("titleBar.minimize")}
			></button>
			<button class="tl maximize" onmousedown={stopDrag} onclick={toggleMaximize} aria-label={t("titleBar.maximize")}
			></button>
		</div>
		<div class="titlebar-title">HollowChat</div>
		<div class="titlebar-spacer"></div>
	{:else}
		<div class="titlebar-title">HollowChat</div>
		<div class="titlebar-controls">
			<button onmousedown={stopDrag} onclick={minimize} aria-label={t("titleBar.minimize")}>
				<Minus size={15} />
			</button>
			<button onmousedown={stopDrag} onclick={toggleMaximize} aria-label={t("titleBar.maximize")}>
				<Square size={12} />
			</button>
			<button class="close" onmousedown={stopDrag} onclick={close} aria-label={t("common.close")}>
				<X size={16} />
			</button>
		</div>
	{/if}
</div>

<style>
	.titlebar {
		display: flex;
		align-items: center;
		height: 36px;
		padding: 0 12px;
		background: var(--titlebar-bg, var(--sidebar));
		color: var(--ink);
		user-select: none;
		flex-shrink: 0;
	}

	.titlebar.mac {
		justify-content: center;
		position: relative;
	}

	.titlebar:not(.mac) {
		justify-content: space-between;
	}

	.titlebar-title {
		font-size: 13px;
		font-weight: 500;
		opacity: 0.8;
		pointer-events: none;
	}

	.traffic-lights {
		position: absolute;
		left: 12px;
		display: flex;
		gap: 8px;
	}

	.tl {
		width: 12px;
		height: 12px;
		border-radius: 50%;
		border: none;
		padding: 0;
	}

	.tl.close {
		background: #ff5f57;
	}

	.tl.minimize {
		background: #febc2e;
	}

	.tl.maximize {
		background: #28c840;
	}

	.titlebar-controls {
		display: flex;
		gap: 4px;
	}

	.titlebar-controls button {
		display: flex;
		align-items: center;
		justify-content: center;
		width: 32px;
		height: 24px;
		background: none;
		border: none;
		color: var(--ink-dim);
		border-radius: 4px;
		transition: background-color 0.15s ease;
	}

	.titlebar-controls button:hover {
		background: var(--hover);
	}

	.titlebar-controls button.close:hover {
		background: var(--danger);
		color: white;
	}
</style>

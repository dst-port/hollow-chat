<script lang="ts">
	import Modal from "$lib/components/Modal.svelte";
	import { t } from "$lib/i18n/index.svelte";

	let { src, filename, onCancel, onConfirm }: {
		src: string;
		filename: string;
		onCancel: () => void;
		onConfirm: (file: File) => void;
	} = $props();

	let imageEl = $state<HTMLImageElement | undefined>();
	let frameEl = $state<HTMLDivElement | undefined>();

	let rect = $state<{ x: number; y: number; w: number; h: number } | null>(null);
	let dragStart: { x: number; y: number } | null = null;

	function pointerPos(event: PointerEvent) {
		const frame = frameEl!.getBoundingClientRect();
		const x = Math.min(Math.max(event.clientX - frame.left, 0), frame.width);
		const y = Math.min(Math.max(event.clientY - frame.top, 0), frame.height);
		return { x, y };
	}

	function onPointerDown(event: PointerEvent) {
		if (!frameEl) return;
		(event.target as HTMLElement).setPointerCapture(event.pointerId);
		dragStart = pointerPos(event);
		rect = { x: dragStart.x, y: dragStart.y, w: 0, h: 0 };
	}

	function onPointerMove(event: PointerEvent) {
		if (!dragStart || !frameEl) return;
		const p = pointerPos(event);
		rect = {
			x: Math.min(dragStart.x, p.x),
			y: Math.min(dragStart.y, p.y),
			w: Math.abs(p.x - dragStart.x),
			h: Math.abs(p.y - dragStart.y)
		};
	}

	function onPointerUp() {
		dragStart = null;
		if (rect && (rect.w < 8 || rect.h < 8)) rect = null;
	}

	function resetSelection() {
		rect = null;
	}

	async function confirmCrop() {
		if (!imageEl || !frameEl) return;
		const frame = frameEl.getBoundingClientRect();
		const scaleX = imageEl.naturalWidth / frame.width;
		const scaleY = imageEl.naturalHeight / frame.height;

		const region = rect ?? { x: 0, y: 0, w: frame.width, h: frame.height };
		const sx = Math.round(region.x * scaleX);
		const sy = Math.round(region.y * scaleY);
		const sw = Math.max(1, Math.round(region.w * scaleX));
		const sh = Math.max(1, Math.round(region.h * scaleY));

		const canvas = document.createElement("canvas");
		canvas.width = sw;
		canvas.height = sh;
		const ctx = canvas.getContext("2d");
		if (!ctx) return;
		ctx.drawImage(imageEl, sx, sy, sw, sh, 0, 0, sw, sh);

		canvas.toBlob((blob) => {
			if (!blob) return;
			onConfirm(new File([blob], filename, { type: "image/png" }));
		}, "image/png");
	}
</script>

<Modal title={t("crop.title")} onClose={onCancel} width={560}>
	<div class="crop-body">
		<p class="hint">{t("crop.hint")}</p>
		<div
			class="frame"
			bind:this={frameEl}
			onpointerdown={onPointerDown}
			onpointermove={onPointerMove}
			onpointerup={onPointerUp}
		>
			<img bind:this={imageEl} {src} alt={filename} draggable="false" />
			{#if rect}
				<div class="selection" style:left={`${rect.x}px`} style:top={`${rect.y}px`} style:width={`${rect.w}px`} style:height={`${rect.h}px`}></div>
			{/if}
		</div>
		<div class="actions">
			<button type="button" class="ghost" onclick={resetSelection} disabled={!rect}>{t("crop.resetSelection")}</button>
			<div class="spacer"></div>
			<button type="button" class="ghost" onclick={onCancel}>{t("common.cancel")}</button>
			<button type="button" class="primary" onclick={confirmCrop}>{t("common.apply")}</button>
		</div>
	</div>
</Modal>

<style>
	.crop-body {
		display: flex;
		flex-direction: column;
		gap: 12px;
	}

	.hint {
		margin: 0;
		font-size: 12px;
		color: var(--ink-faint);
	}

	.frame {
		position: relative;
		width: 100%;
		max-height: 60vh;
		overflow: hidden;
		border-radius: 8px;
		background: var(--void);
		touch-action: none;
		cursor: crosshair;
	}

	.frame img {
		display: block;
		width: 100%;
		height: auto;
		max-height: 60vh;
		object-fit: contain;
		user-select: none;
	}

	.selection {
		position: absolute;
		border: 2px solid var(--accent-fill);
		background: rgba(255, 255, 255, 0.12);
		pointer-events: none;
	}

	.actions {
		display: flex;
		align-items: center;
		gap: 8px;
	}

	.spacer {
		flex: 1;
	}

	.ghost {
		padding: 8px 14px;
		border-radius: 6px;
		background: var(--active);
		color: var(--ink-dim);
		font-size: 13px;
		font-weight: 600;
	}

	.ghost:hover:not(:disabled) {
		background: var(--hover);
		color: var(--ink);
	}

	.ghost:disabled {
		opacity: 0.5;
	}

	.primary {
		padding: 8px 14px;
		border-radius: 6px;
		background: var(--accent-fill);
		color: var(--accent-fill-ink);
		font-size: 13px;
		font-weight: 700;
	}

	.primary:hover {
		filter: brightness(1.08);
	}
</style>

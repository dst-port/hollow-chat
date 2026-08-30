<script lang="ts">
	import Modal from "$lib/components/Modal.svelte";
	import { t } from "$lib/i18n/index.svelte";

	let {
		src,
		filename,
		round = false,
		outputSize = 512,
		onCancel,
		onConfirm
	}: {
		src: string;
		filename: string;
		round?: boolean;
		outputSize?: number;
		onCancel: () => void;
		onConfirm: (file: File) => void;
	} = $props();

	const FRAME = 300; // on-screen square viewport, px

	let imgEl = $state<HTMLImageElement | undefined>();
	let natW = $state(0);
	let natH = $state(0);
	let ready = $state(false);

	let zoom = $state(1); // multiplier on top of the cover-fit base scale
	let offsetX = $state(0); // px, image top-left relative to frame top-left (<= 0)
	let offsetY = $state(0);

	const MAX_ZOOM = 4;

	// base scale so the image exactly covers the frame at zoom = 1
	const baseScale = $derived(natW && natH ? FRAME / Math.min(natW, natH) : 1);
	const dispW = $derived(natW * baseScale * zoom);
	const dispH = $derived(natH * baseScale * zoom);

	function clampOffsets() {
		const minX = FRAME - dispW;
		const minY = FRAME - dispH;
		offsetX = Math.min(0, Math.max(minX, offsetX));
		offsetY = Math.min(0, Math.max(minY, offsetY));
	}

	function onImgLoad() {
		if (!imgEl) return;
		natW = imgEl.naturalWidth;
		natH = imgEl.naturalHeight;
		zoom = 1;
		// center
		offsetX = (FRAME - natW * baseScale) / 2;
		offsetY = (FRAME - natH * baseScale) / 2;
		clampOffsets();
		ready = true;
	}

	let dragging = false;
	let last = { x: 0, y: 0 };

	function onPointerDown(e: PointerEvent) {
		dragging = true;
		last = { x: e.clientX, y: e.clientY };
		(e.currentTarget as HTMLElement).setPointerCapture(e.pointerId);
	}
	function onPointerMove(e: PointerEvent) {
		if (!dragging) return;
		offsetX += e.clientX - last.x;
		offsetY += e.clientY - last.y;
		last = { x: e.clientX, y: e.clientY };
		clampOffsets();
	}
	function onPointerUp(e: PointerEvent) {
		dragging = false;
		try {
			(e.currentTarget as HTMLElement).releasePointerCapture(e.pointerId);
		} catch {
			/* ignore */
		}
	}

	function applyZoom(nextZoom: number, pivotX = FRAME / 2, pivotY = FRAME / 2) {
		const clamped = Math.min(MAX_ZOOM, Math.max(1, nextZoom));
		// keep the point under the pivot fixed
		const ratio = clamped / zoom;
		offsetX = pivotX - (pivotX - offsetX) * ratio;
		offsetY = pivotY - (pivotY - offsetY) * ratio;
		zoom = clamped;
		clampOffsets();
	}

	function onWheel(e: WheelEvent) {
		e.preventDefault();
		const rect = (e.currentTarget as HTMLElement).getBoundingClientRect();
		applyZoom(zoom * (e.deltaY < 0 ? 1.12 : 0.89), e.clientX - rect.left, e.clientY - rect.top);
	}

	function confirm() {
		if (!imgEl || !natW) return;
		const px = baseScale * zoom; // screen px per natural px
		const sx = -offsetX / px;
		const sy = -offsetY / px;
		const sSize = FRAME / px;

		const canvas = document.createElement("canvas");
		canvas.width = outputSize;
		canvas.height = outputSize;
		const ctx = canvas.getContext("2d");
		if (!ctx) return;
		ctx.imageSmoothingQuality = "high";
		ctx.drawImage(imgEl, sx, sy, sSize, sSize, 0, 0, outputSize, outputSize);

		const type = filename.toLowerCase().endsWith(".png") ? "image/png" : "image/jpeg";
		canvas.toBlob(
			(blob) => {
				if (!blob) return;
				const base = filename.replace(/\.[^.]+$/, "") || "image";
				const ext = type === "image/png" ? "png" : "jpg";
				onConfirm(new File([blob], `${base}.${ext}`, { type }));
			},
			type,
			0.92
		);
	}
</script>

<Modal title={t("imageCrop.title")} onClose={onCancel} width={380}>
	<div class="crop">
		<div
			class="frame"
			class:round
			onpointerdown={onPointerDown}
			onpointermove={onPointerMove}
			onpointerup={onPointerUp}
			onpointercancel={onPointerUp}
			onwheel={onWheel}
			role="presentation"
		>
			<img
				bind:this={imgEl}
				{src}
				alt=""
				draggable="false"
				onload={onImgLoad}
				style:width={`${dispW}px`}
				style:height={`${dispH}px`}
				style:transform={`translate(${offsetX}px, ${offsetY}px)`}
			/>
			<div class="frame-ring"></div>
		</div>

		<input
			class="zoom"
			type="range"
			min="1"
			max={MAX_ZOOM}
			step="0.01"
			value={zoom}
			oninput={(e) => applyZoom(Number(e.currentTarget.value))}
			disabled={!ready}
			aria-label={t("imageCrop.zoom")}
		/>

		<p class="hint">{t("imageCrop.hint")}</p>

		<div class="actions">
			<button type="button" class="ghost" onclick={onCancel}>{t("common.cancel")}</button>
			<button type="button" class="primary" onclick={confirm} disabled={!ready}>{t("common.apply")}</button>
		</div>
	</div>
</Modal>

<style>
	.crop {
		display: flex;
		flex-direction: column;
		gap: 14px;
		align-items: center;
	}

	.frame {
		position: relative;
		width: 300px;
		height: 300px;
		overflow: hidden;
		border-radius: 10px;
		background: var(--void);
		touch-action: none;
		cursor: grab;
	}

	.frame:active {
		cursor: grabbing;
	}

	.frame.round {
		border-radius: 50%;
	}

	.frame img {
		position: absolute;
		top: 0;
		left: 0;
		max-width: none;
		user-select: none;
	}

	.frame-ring {
		position: absolute;
		inset: 0;
		pointer-events: none;
		box-shadow: 0 0 0 1px rgba(255, 255, 255, 0.25) inset;
		border-radius: inherit;
	}

	.zoom {
		width: 100%;
		accent-color: var(--accent-fill);
	}

	.hint {
		margin: 0;
		font-size: 12px;
		color: var(--ink-faint);
		text-align: center;
	}

	.actions {
		display: flex;
		gap: 8px;
		align-self: stretch;
	}

	.actions button {
		flex: 1;
		padding: 9px 14px;
		border-radius: 6px;
		font-size: 13px;
		font-weight: 700;
	}

	.ghost {
		background: var(--active);
		color: var(--ink-dim);
	}

	.ghost:hover {
		background: var(--hover);
		color: var(--ink);
	}

	.primary {
		background: var(--accent-fill);
		color: var(--accent-fill-ink);
	}

	.primary:hover:not(:disabled) {
		filter: brightness(1.08);
	}

	.primary:disabled {
		opacity: 0.5;
	}
</style>

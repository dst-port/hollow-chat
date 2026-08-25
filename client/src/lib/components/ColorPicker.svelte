<script lang="ts">
	import { clickOutside } from "$lib/actions/clickOutside";

	let { value = $bindable() }: {
		value: string;
	} = $props();

	const PRESETS = [
		"#5b96c9",
		"#7fa88a",
		"#d9718a",
		"#c9a227",
		"#e2793f",
		"#9c93c2",
		"#3ba55d",
		"#d83c3e",
		"#8f97a8",
		"#2b2d31"
	];

	const WHEEL_SIZE = 140;
	const WHEEL_RADIUS = WHEEL_SIZE / 2;

	let open = $state(false);
	let hexDraft = $state(value);
	let canvasEl: HTMLCanvasElement | undefined;
	let dragging = $state(false);

	let hue = $state(0);
	let sat = $state(0);
	let light = $state(0.5);

	function hexToRgb(hex: string): [number, number, number] {
		const clean = hex.replace("#", "");
		return [
			parseInt(clean.slice(0, 2), 16),
			parseInt(clean.slice(2, 4), 16),
			parseInt(clean.slice(4, 6), 16)
		];
	}

	function rgbToHex(r: number, g: number, b: number): string {
		const toHex = (n: number) => Math.round(Math.max(0, Math.min(255, n))).toString(16).padStart(2, "0");
		return `#${toHex(r)}${toHex(g)}${toHex(b)}`;
	}

	function rgbToHsl(r: number, g: number, b: number): [number, number, number] {
		r /= 255;
		g /= 255;
		b /= 255;
		const max = Math.max(r, g, b);
		const min = Math.min(r, g, b);
		const l = (max + min) / 2;
		if (max === min) return [0, 0, l];
		const d = max - min;
		const s = l > 0.5 ? d / (2 - max - min) : d / (max + min);
		let h: number;
		if (max === r) h = ((g - b) / d + (g < b ? 6 : 0)) * 60;
		else if (max === g) h = ((b - r) / d + 2) * 60;
		else h = ((r - g) / d + 4) * 60;
		return [h, s, l];
	}

	function hslToRgb(h: number, s: number, l: number): [number, number, number] {
		if (s === 0) return [l * 255, l * 255, l * 255];
		const q = l < 0.5 ? l * (1 + s) : l + s - l * s;
		const p = 2 * l - q;
		const hue2rgb = (t: number) => {
			let tt = t;
			if (tt < 0) tt += 1;
			if (tt > 1) tt -= 1;
			if (tt < 1 / 6) return p + (q - p) * 6 * tt;
			if (tt < 1 / 2) return q;
			if (tt < 2 / 3) return p + (q - p) * (2 / 3 - tt) * 6;
			return p;
		};
		const hh = h / 360;
		return [hue2rgb(hh + 1 / 3) * 255, hue2rgb(hh) * 255, hue2rgb(hh - 1 / 3) * 255];
	}

	function syncFromHex(hex: string) {
		if (!/^#[0-9a-fA-F]{6}$/.test(hex)) return;
		const [r, g, b] = hexToRgb(hex);
		const [h, s, l] = rgbToHsl(r, g, b);
		hue = h;
		sat = s;
		light = l;
	}

	function currentHex(): string {
		const [r, g, b] = hslToRgb(hue, sat, light);
		return rgbToHex(r, g, b);
	}

	function drawWheel() {
		if (!canvasEl) return;
		const ctx = canvasEl.getContext("2d");
		if (!ctx) return;
		const img = ctx.createImageData(WHEEL_SIZE, WHEEL_SIZE);
		for (let y = 0; y < WHEEL_SIZE; y++) {
			for (let x = 0; x < WHEEL_SIZE; x++) {
				const dx = x - WHEEL_RADIUS;
				const dy = y - WHEEL_RADIUS;
				const dist = Math.sqrt(dx * dx + dy * dy);
				const i = (y * WHEEL_SIZE + x) * 4;
				if (dist > WHEEL_RADIUS) {
					img.data[i + 3] = 0;
					continue;
				}
				const angle = (Math.atan2(dy, dx) * 180) / Math.PI;
				const h = (angle + 360) % 360;
				const s = Math.min(1, dist / WHEEL_RADIUS);
				const [r, g, b] = hslToRgb(h, s, light);
				img.data[i] = r;
				img.data[i + 1] = g;
				img.data[i + 2] = b;
				img.data[i + 3] = 255;
			}
		}
		ctx.putImageData(img, 0, 0);
	}

	function pickAt(clientX: number, clientY: number) {
		if (!canvasEl) return;
		const rect = canvasEl.getBoundingClientRect();
		const x = clientX - rect.left - WHEEL_RADIUS;
		const y = clientY - rect.top - WHEEL_RADIUS;
		const dist = Math.sqrt(x * x + y * y);
		const angle = (Math.atan2(y, x) * 180) / Math.PI;
		hue = (angle + 360) % 360;
		sat = Math.min(1, dist / WHEEL_RADIUS);
		const hex = currentHex();
		value = hex;
		hexDraft = hex;
	}

	function onPointerDown(event: PointerEvent) {
		dragging = true;
		(event.target as HTMLElement).setPointerCapture(event.pointerId);
		pickAt(event.clientX, event.clientY);
	}

	function onPointerMove(event: PointerEvent) {
		if (!dragging) return;
		pickAt(event.clientX, event.clientY);
	}

	function onPointerUp() {
		dragging = false;
	}

	function onLightChange() {
		const hex = currentHex();
		value = hex;
		hexDraft = hex;
		drawWheel();
	}

	function pick(color: string) {
		value = color;
		hexDraft = color;
		syncFromHex(color);
		drawWheel();
	}

	function applyHex() {
		const trimmed = hexDraft.trim();
		if (/^#[0-9a-fA-F]{6}$/.test(trimmed)) {
			value = trimmed;
			syncFromHex(trimmed);
			drawWheel();
		}
	}

	$effect(() => {
		if (open) {
			syncFromHex(value);
			hexDraft = value;
			requestAnimationFrame(drawWheel);
		}
	});

	const knobPos = $derived.by(() => {
		const rad = (hue * Math.PI) / 180;
		const r = sat * WHEEL_RADIUS;
		return { x: WHEEL_RADIUS + Math.cos(rad) * r, y: WHEEL_RADIUS + Math.sin(rad) * r };
	});
</script>

<div class="picker">
	<button class="swatch" style:background={value} onclick={() => { hexDraft = value; open = !open; }} title="Choose color"></button>
	{#if open}
		<div class="panel" use:clickOutside={() => (open = false)}>
			<div class="wheel-wrap">
				<canvas
					bind:this={canvasEl}
					width={WHEEL_SIZE}
					height={WHEEL_SIZE}
					onpointerdown={onPointerDown}
					onpointermove={onPointerMove}
					onpointerup={onPointerUp}
				></canvas>
				<span class="knob" style:left={`${knobPos.x}px`} style:top={`${knobPos.y}px`}></span>
			</div>

			<input
				class="light-slider"
				type="range"
				min="0"
				max="100"
				value={Math.round(light * 100)}
				oninput={(e) => { light = Number((e.target as HTMLInputElement).value) / 100; onLightChange(); }}
			/>

			<div class="grid">
				{#each PRESETS as color (color)}
					<button
						class="preset"
						class:active={color.toLowerCase() === value.toLowerCase()}
						style:background={color}
						onclick={() => pick(color)}
						title={color}
					></button>
				{/each}
			</div>
			<div class="hex-row">
				<span class="hex-preview" style:background={hexDraft}></span>
				<input
					class="hex-input"
					type="text"
					bind:value={hexDraft}
					maxlength="7"
					placeholder="#5b96c9"
					onblur={applyHex}
					onkeydown={(e) => e.key === "Enter" && applyHex()}
				/>
			</div>
		</div>
	{/if}
</div>

<style>
	.picker {
		position: relative;
	}

	.swatch {
		width: 40px;
		height: 32px;
		border: 1px solid var(--hairline);
		border-radius: 6px;
		flex-shrink: 0;
		transition: transform 0.1s ease, border-color 0.15s ease;
	}

	.swatch:hover {
		border-color: var(--ink-dim);
		transform: scale(1.04);
	}

	.panel {
		position: absolute;
		top: calc(100% + 8px);
		right: 0;
		background: var(--panel);
		border-radius: 10px;
		padding: 12px;
		box-shadow: 0 16px 40px rgba(0, 0, 0, 0.45);
		z-index: 50;
		width: 190px;
	}

	.wheel-wrap {
		position: relative;
		width: 140px;
		height: 140px;
		margin: 0 auto 10px;
	}

	.wheel-wrap canvas {
		width: 100%;
		height: 100%;
		border-radius: 50%;
		cursor: crosshair;
		touch-action: none;
	}

	.knob {
		position: absolute;
		width: 12px;
		height: 12px;
		border-radius: 50%;
		border: 2px solid white;
		box-shadow: 0 0 0 1px rgba(0, 0, 0, 0.5), 0 1px 3px rgba(0, 0, 0, 0.4);
		transform: translate(-50%, -50%);
		pointer-events: none;
	}

	.light-slider {
		width: 100%;
		margin-bottom: 10px;
		accent-color: var(--ink);
	}

	.grid {
		display: grid;
		grid-template-columns: repeat(5, 1fr);
		gap: 6px;
		margin-bottom: 10px;
	}

	.preset {
		width: 100%;
		aspect-ratio: 1;
		border-radius: 50%;
		border: 2px solid transparent;
		transition: transform 0.1s ease, border-color 0.15s ease;
	}

	.preset:hover {
		transform: scale(1.1);
	}

	.preset.active {
		border-color: var(--ink);
	}

	.hex-row {
		display: flex;
		align-items: center;
		gap: 6px;
		border-top: 1px solid var(--hairline);
		padding-top: 10px;
	}

	.hex-preview {
		width: 22px;
		height: 22px;
		border-radius: 6px;
		flex-shrink: 0;
		border: 1px solid var(--hairline);
	}

	.hex-input {
		flex: 1;
		min-width: 0;
		background: var(--sidebar);
		border: 1px solid var(--hairline);
		border-radius: 6px;
		padding: 6px 8px;
		color: var(--ink);
		font-family: var(--font-mono);
		font-size: 12px;
	}

	.hex-input:focus {
		outline: none;
		border-color: var(--ink-dim);
	}
</style>

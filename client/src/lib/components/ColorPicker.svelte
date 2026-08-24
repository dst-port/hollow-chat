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

	let open = $state(false);
	let hexDraft = $state(value);

	function pick(color: string) {
		value = color;
		hexDraft = color;
		open = false;
	}

	function applyHex() {
		const trimmed = hexDraft.trim();
		if (/^#[0-9a-fA-F]{6}$/.test(trimmed)) value = trimmed;
	}
</script>

<div class="picker">
	<button class="swatch" style:background={value} onclick={() => { hexDraft = value; open = !open; }} title="Choose color"></button>
	{#if open}
		<div class="panel" use:clickOutside={() => (open = false)}>
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

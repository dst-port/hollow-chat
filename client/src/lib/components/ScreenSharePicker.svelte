<script lang="ts">
	import Modal from "$lib/components/Modal.svelte";
	import Check from "@lucide/svelte/icons/check";
	import { t } from "$lib/i18n/index.svelte";
	import type { ScreenShareOpts } from "$lib/webrtc/call.svelte";

	let { onCancel, onGoLive }: {
		onCancel: () => void;
		onGoLive: (opts: ScreenShareOpts) => void;
	} = $props();

	const RES: { label: string; w: number; h: number }[] = [
		{ label: "480", w: 854, h: 480 },
		{ label: "720", w: 1280, h: 720 },
		{ label: "1080", w: 1920, h: 1080 },
		{ label: "1440", w: 2560, h: 1440 },
		{ label: "2160", w: 3840, h: 2160 }
	];
	const FPS = [15, 30, 60];

	let resIndex = $state(1); // 720
	let fps = $state(30);
	let contentHint = $state<"motion" | "detail">("motion");
	let shareAudio = $state(false);

	function goLive() {
		const r = RES[resIndex];
		onGoLive({ width: r.w, height: r.h, frameRate: fps, contentHint, audio: shareAudio });
	}
</script>

<Modal title={t("screenShare.title")} onClose={onCancel} width={440}>
	<div class="picker">
		<div class="field">
			<span class="field-label">{t("screenShare.resolution")}</span>
			<div class="segments">
				{#each RES as r, i (r.label)}
					<button type="button" class="seg" class:active={resIndex === i} onclick={() => (resIndex = i)}>
						{r.label}
					</button>
				{/each}
			</div>
		</div>

		<div class="field">
			<span class="field-label">{t("screenShare.frameRate")}</span>
			<div class="segments">
				{#each FPS as f (f)}
					<button type="button" class="seg" class:active={fps === f} onclick={() => (fps = f)}>{f}</button>
				{/each}
			</div>
		</div>

		<div class="field">
			<span class="field-label">{t("screenShare.contentType")}</span>
			<div class="segments wide">
				<button type="button" class="seg" class:active={contentHint === "motion"} onclick={() => (contentHint = "motion")}>
					{t("screenShare.smoothness")}
				</button>
				<button type="button" class="seg" class:active={contentHint === "detail"} onclick={() => (contentHint = "detail")}>
					{t("screenShare.clarity")}
				</button>
			</div>
			<p class="hint">
				{contentHint === "detail" ? t("screenShare.clarityHint") : t("screenShare.smoothnessHint")}
			</p>
		</div>

		<label class="audio-row">
			<input type="checkbox" bind:checked={shareAudio} />
			<span class="box" aria-hidden="true">
				{#if shareAudio}<Check size={12} strokeWidth={3} />{/if}
			</span>
			<span>{t("screenShare.shareAudio")}</span>
		</label>

		<div class="actions">
			<button type="button" class="ghost" onclick={onCancel}>{t("common.cancel")}</button>
			<button type="button" class="primary" onclick={goLive}>{t("screenShare.goLive")}</button>
		</div>
	</div>
</Modal>

<style>
	.picker {
		display: flex;
		flex-direction: column;
		gap: 16px;
	}

	.field {
		display: flex;
		flex-direction: column;
		gap: 8px;
	}

	.field-label {
		font-size: 12px;
		font-weight: 700;
		text-transform: uppercase;
		letter-spacing: 0.04em;
		color: var(--ink-faint);
	}

	.segments {
		display: flex;
		gap: 6px;
	}

	.segments.wide .seg {
		flex: 1;
	}

	.seg {
		flex: 1;
		padding: 8px 10px;
		border-radius: 6px;
		background: var(--active);
		color: var(--ink-dim);
		font-size: 13px;
		font-weight: 600;
		transition: background-color 0.12s ease, color 0.12s ease;
	}

	.seg:hover {
		background: var(--hover);
		color: var(--ink);
	}

	.seg.active {
		background: var(--accent-fill);
		color: var(--accent-fill-ink);
	}

	.hint {
		margin: 0;
		font-size: 12px;
		color: var(--ink-faint);
	}

	.audio-row {
		display: flex;
		align-items: center;
		gap: 9px;
		font-size: 13px;
		color: var(--ink-dim);
		cursor: pointer;
		user-select: none;
	}

	.audio-row input {
		position: absolute;
		opacity: 0;
		width: 0;
		height: 0;
	}

	.audio-row .box {
		flex-shrink: 0;
		width: 18px;
		height: 18px;
		border-radius: 5px;
		border: 1.5px solid var(--hairline, var(--ink-faint));
		background: var(--active);
		display: flex;
		align-items: center;
		justify-content: center;
		color: var(--accent-fill-ink);
		transition: background-color 0.12s ease, border-color 0.12s ease;
	}

	.audio-row input:checked + .box {
		background: var(--accent-fill);
		border-color: var(--accent-fill);
	}

	.audio-row input:focus-visible + .box {
		outline: 2px solid var(--accent, #8ea1ff);
		outline-offset: 2px;
	}

	.actions {
		display: flex;
		gap: 8px;
		margin-top: 4px;
	}

	.actions button {
		flex: 1;
		padding: 10px 14px;
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
		background: var(--online);
		color: var(--void);
	}

	.primary:hover {
		filter: brightness(1.08);
	}
</style>

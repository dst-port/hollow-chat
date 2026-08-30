<script lang="ts">
	import Modal from "$lib/components/Modal.svelte";
	import RefreshCw from "@lucide/svelte/icons/refresh-cw";
	import MonitorUp from "@lucide/svelte/icons/monitor-up";
	import { t } from "$lib/i18n/index.svelte";
	import { call } from "$lib/webrtc/call.svelte";
	import { toast } from "$lib/stores/toast.svelte";
	import type { ScreenShareOpts } from "$lib/webrtc/call.svelte";

	let { onClose }: { onClose: () => void } = $props();

	type Surface = "screen" | "window" | "tab";
	type AudioMode = "none" | "system";

	const SURFACES: { id: Surface; label: string }[] = [
		{ id: "screen", label: t("screenShare.sourceScreen") },
		{ id: "window", label: t("screenShare.sourceWindow") },
		{ id: "tab", label: t("screenShare.sourceTab") }
	];

	const RES: { label: string; w: number; h: number }[] = [
		{ label: "480", w: 854, h: 480 },
		{ label: "720", w: 1280, h: 720 },
		{ label: "1080", w: 1920, h: 1080 },
		{ label: "1440", w: 2560, h: 1440 },
		{ label: "2160", w: 3840, h: 2160 }
	];
	const FPS = [15, 30, 60];

	let surface = $state<Surface>("screen");
	let resIndex = $state(1); // 720
	let fps = $state(30);
	let contentHint = $state<"motion" | "detail">("motion");
	let audioMode = $state<AudioMode>("none");

	let preview = $state<MediaStream | null>(null);
	let busy = $state(false);
	let handedOff = false;

	function currentOpts(): ScreenShareOpts {
		const r = RES[resIndex];
		return {
			surface,
			width: r.w,
			height: r.h,
			frameRate: fps,
			contentHint,
			audio: audioMode !== "none"
		};
	}

	function attach(node: HTMLVideoElement) {
		$effect(() => {
			node.srcObject = preview;
			if (preview) node.play().catch(() => {});
		});
	}

	function stopPreview() {
		preview?.getTracks().forEach((tr) => tr.stop());
		preview = null;
	}

	async function pickSource() {
		if (busy) return;
		busy = true;
		try {
			stopPreview();
			const stream = await call.acquireDisplayStream(currentOpts());
			stream.getVideoTracks()[0]?.addEventListener("ended", () => {
				if (!handedOff) preview = null;
			});
			preview = stream;
		} catch (err) {
			if (!(err instanceof DOMException && (err.name === "NotAllowedError" || err.name === "AbortError"))) {
				toast.push(t("toast.screenShareFailed"));
			}
		} finally {
			busy = false;
		}
	}

	// Live tweaks that don't need a re-pick.
	$effect(() => {
		const track = preview?.getVideoTracks()[0];
		if (!track) return;
		const r = RES[resIndex];
		track.applyConstraints({ width: { ideal: r.w }, height: { ideal: r.h }, frameRate: { ideal: fps } }).catch(() => {});
	});
	$effect(() => {
		const track = preview?.getVideoTracks()[0];
		if (track) track.contentHint = contentHint;
	});

	async function goLive() {
		if (busy) return;
		busy = true;
		try {
			let stream = preview;
			if (!stream) stream = await call.acquireDisplayStream(currentOpts());
			handedOff = true;
			await call.startScreenShareWithStream(stream);
			preview = null;
			onClose();
		} catch (err) {
			handedOff = false;
			if (!(err instanceof DOMException && (err.name === "NotAllowedError" || err.name === "AbortError"))) {
				toast.push(t("toast.screenShareFailed"));
			}
		} finally {
			busy = false;
		}
	}

	function cancel() {
		if (!handedOff) stopPreview();
		onClose();
	}
</script>

<Modal title={t("screenShare.pickerTitle")} onClose={cancel} width={520}>
	<div class="picker">
		<div class="field">
			<span class="field-label">{t("screenShare.whatYoureStreaming")}</span>
			<div class="preview" class:empty={!preview}>
				{#if preview}
					<video use:attach autoplay playsinline muted></video>
				{:else}
					<button type="button" class="pick" onclick={pickSource} disabled={busy}>
						<MonitorUp size={20} strokeWidth={1.75} />
						{busy ? t("common.loading") : t("screenShare.chooseSource")}
					</button>
				{/if}
			</div>
			<div class="source-row">
				<select bind:value={surface} onchange={() => preview && pickSource()}>
					{#each SURFACES as s (s.id)}
						<option value={s.id}>{s.label}</option>
					{/each}
				</select>
				{#if preview}
					<button type="button" class="mini" onclick={pickSource} disabled={busy}>
						<RefreshCw size={13} strokeWidth={2.25} />
						{t("screenShare.rePick")}
					</button>
				{/if}
			</div>
			<p class="hint">{t("screenShare.browserAsks")}</p>
		</div>

		<div class="field">
			<span class="field-label">{t("screenShare.streamSettings")}</span>
			<div class="grid2">
				<div class="sub">
					<span class="sub-label">{t("screenShare.resolution")}</span>
					<div class="segments">
						{#each RES as r, i (r.label)}
							<button type="button" class="seg" class:active={resIndex === i} onclick={() => (resIndex = i)}>{r.label}</button>
						{/each}
					</div>
				</div>
				<div class="sub">
					<span class="sub-label">{t("screenShare.frameRate")}</span>
					<div class="segments">
						{#each FPS as f (f)}
							<button type="button" class="seg" class:active={fps === f} onclick={() => (fps = f)}>{f}</button>
						{/each}
					</div>
				</div>
			</div>
			<div class="sub">
				<span class="sub-label">{t("screenShare.contentType")}</span>
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
		</div>

		<div class="field">
			<span class="field-label">{t("screenShare.audioSources")}</span>
			<select bind:value={audioMode} onchange={() => preview && pickSource()}>
				<option value="none">{t("screenShare.audioNone")}</option>
				<option value="system">{surface === "tab" ? t("screenShare.shareTabAudio") : t("screenShare.audioSystem")}</option>
			</select>
		</div>

		<div class="actions">
			<button type="button" class="ghost" onclick={cancel}>{t("common.cancel")}</button>
			<button type="button" class="primary" onclick={goLive} disabled={busy}>{t("screenShare.goLive")}</button>
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

	.sub {
		display: flex;
		flex-direction: column;
		gap: 6px;
	}

	.sub-label {
		font-size: 11px;
		font-weight: 700;
		color: var(--ink-faint);
	}

	.grid2 {
		display: grid;
		grid-template-columns: 1.6fr 1fr;
		gap: 10px;
	}

	.preview {
		position: relative;
		width: 100%;
		aspect-ratio: 16 / 9;
		border-radius: 10px;
		overflow: hidden;
		background: #000;
		display: flex;
		align-items: center;
		justify-content: center;
	}

	.preview.empty {
		background: var(--active);
	}

	.preview video {
		width: 100%;
		height: 100%;
		object-fit: contain;
	}

	.pick {
		display: flex;
		align-items: center;
		gap: 8px;
		padding: 10px 16px;
		border-radius: 8px;
		background: var(--hover);
		color: var(--ink);
		font-size: 13px;
		font-weight: 700;
	}

	.pick:hover:not(:disabled) {
		background: var(--accent-fill);
		color: var(--accent-fill-ink);
	}

	.pick:disabled {
		opacity: 0.6;
	}

	.source-row {
		display: flex;
		gap: 8px;
	}

	select {
		flex: 1;
		padding: 9px 10px;
		border-radius: 6px;
		background: var(--active);
		color: var(--ink);
		font-size: 13px;
		border: 1px solid var(--hairline, transparent);
	}

	.mini {
		display: flex;
		align-items: center;
		gap: 5px;
		padding: 0 12px;
		border-radius: 6px;
		background: var(--active);
		color: var(--ink-dim);
		font-size: 12px;
		font-weight: 600;
	}

	.mini:hover:not(:disabled) {
		background: var(--hover);
		color: var(--ink);
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
		font-size: 11px;
		color: var(--ink-faint);
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

	.primary:hover:not(:disabled) {
		filter: brightness(1.08);
	}

	.primary:disabled {
		opacity: 0.6;
	}
</style>

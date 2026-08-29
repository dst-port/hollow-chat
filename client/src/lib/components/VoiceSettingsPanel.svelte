<script lang="ts">
	import { onMount, onDestroy } from "svelte";
	import Check from "@lucide/svelte/icons/check";
	import Dropdown from "$lib/components/Dropdown.svelte";
	import { call } from "$lib/webrtc/call.svelte";
	import { t } from "$lib/i18n/index.svelte";

	let { focus = "all", onOpenFullSettings }: {
		focus?: "input" | "output" | "all";
		onOpenFullSettings?: () => void;
	} = $props();

	onMount(() => {
		call.refreshDevices();
	});

	const NAMED_KEY_LABELS: Record<string, string> = {
		Space: "voiceSettings.keySpace",
		ControlLeft: "voiceSettings.keyLeftCtrl",
		ControlRight: "voiceSettings.keyRightCtrl",
		AltLeft: "voiceSettings.keyLeftAlt",
		AltRight: "voiceSettings.keyRightAlt",
		ShiftLeft: "voiceSettings.keyLeftShift",
		ShiftRight: "voiceSettings.keyRightShift",
		CapsLock: "voiceSettings.keyCapsLock",
		Tab: "voiceSettings.keyTab",
		MetaLeft: "voiceSettings.keyLeftSuper",
		MetaRight: "voiceSettings.keyRightSuper"
	};

	function keyLabel(code: string): string {
		if (code === "Backquote") return "`";
		if (NAMED_KEY_LABELS[code]) return t(NAMED_KEY_LABELS[code]);
		if (code.startsWith("Key")) return code.slice(3);
		if (code.startsWith("Digit")) return code.slice(5);
		if (code.startsWith("F") && /^F\d+$/.test(code)) return code;
		return code;
	}

	let capturing = $state(false);

	function startCapture() {
		capturing = true;
	}

	function onCaptureKeydown(event: KeyboardEvent) {
		if (!capturing) return;
		event.preventDefault();
		if (event.code === "Escape") {
			capturing = false;
			return;
		}
		call.setPushToTalkKey(event.code);
		capturing = false;
	}

	onDestroy(() => {
		capturing = false;
	});
</script>

<svelte:window onkeydown={onCaptureKeydown} />

<div class="panel">
	{#if focus === "input" || focus === "all"}
		<div class="section">
			{#if focus === "all"}<p class="section-title">{t("voiceSettings.input")}</p>{/if}

			<div class="field">
				<span class="field-label">{t("voiceSettings.inputDevice")}</span>
				<Dropdown
					value={call.inputDeviceId ?? ""}
					options={[
						{ value: "", label: t("voiceSettings.systemDefault") },
						...call.inputDevices.map((d) => ({ value: d.deviceId, label: d.label || t("voiceSettings.microphone") }))
					]}
					onChange={(v) => call.setInputDevice(v || null)}
				/>
			</div>

			<div class="field">
				<span class="field-label">{t("voiceSettings.noiseSuppression")}</span>
				{#if call.noiseSuppressionSupported}
					<div class="radio-list">
						<button
							type="button"
							class="radio-row"
							class:active={!call.noiseSuppression}
							onclick={() => call.setNoiseSuppression(false)}
						>
							<span>{t("voiceSettings.off")}</span>
							{#if !call.noiseSuppression}<Check size={14} strokeWidth={2.5} />{/if}
						</button>
						<button
							type="button"
							class="radio-row"
							class:active={call.noiseSuppression}
							onclick={() => call.setNoiseSuppression(true)}
						>
							<span>{t("voiceSettings.on")}</span>
							{#if call.noiseSuppression}<Check size={14} strokeWidth={2.5} />{/if}
						</button>
					</div>
					{#if call.noiseSuppression}
						<p class="hint" class:warn={!call.noiseSuppressionActive}>
							{call.noiseSuppressionActive
								? t("voiceSettings.noiseActive")
								: t("voiceSettings.noiseUnconfirmed")}
						</p>
					{/if}
				{:else}
					<p class="hint warn">{t("voiceSettings.noiseUnsupported")}</p>
				{/if}
			</div>

			<label class="switch-row">
				<span class="field-label">{t("voiceSettings.pushToTalk")}</span>
				<label class="switch">
					<input
						type="checkbox"
						checked={call.pushToTalk}
						onchange={(e) => call.setPushToTalk(e.currentTarget.checked)}
					/>
					<span class="track"><span class="thumb"></span></span>
				</label>
			</label>

			{#if call.pushToTalk}
				<div class="field">
					<span class="field-label">{t("voiceSettings.pushToTalkKey")}</span>
					<button type="button" class="keybind-btn" class:capturing onclick={startCapture}>
						{capturing ? t("voiceSettings.pressAnyKey") : keyLabel(call.pushToTalkKey)}
					</button>
				</div>
				{#if !capturing}<p class="hint">{t("voiceSettings.holdToTransmit", { key: keyLabel(call.pushToTalkKey) })}</p>{/if}
			{/if}
		</div>
	{/if}

	{#if focus === "output" || focus === "all"}
		<div class="section">
			{#if focus === "all"}<p class="section-title">{t("voiceSettings.output")}</p>{/if}

			<div class="field">
				<span class="field-label">{t("voiceSettings.outputDevice")}</span>
				{#if call.outputDeviceSelectionSupported}
					<Dropdown
						value={call.outputDeviceId ?? ""}
						options={[
							{ value: "", label: t("voiceSettings.systemDefault") },
							...(call.nativeOutputRouting
								? call.nativeSinks.map((s) => ({ value: s.name, label: s.description }))
								: call.outputDevices.map((d) => ({ value: d.deviceId, label: d.label || t("voiceSettings.speaker") })))
						]}
						onChange={(v) => call.setOutputDevice(v || null)}
					/>
				{:else}
					<p class="hint warn">
						{t("voiceSettings.outputUnsupported")}
					</p>
				{/if}
			</div>

			<label class="field">
				<span class="field-label">{t("voiceSettings.outputVolume")}</span>
				<input
					type="range"
					min="0"
					max="100"
					value={Math.round(call.outputVolume * 100)}
					oninput={(e) => call.setOutputVolume(Number(e.currentTarget.value) / 100)}
				/>
			</label>
		</div>
	{/if}

	{#if focus !== "all" && onOpenFullSettings}
		<button type="button" class="full-settings-link" onclick={onOpenFullSettings}>{t("voiceSettings.title")}</button>
	{/if}
</div>

<style>
	.panel {
		display: flex;
		flex-direction: column;
		gap: 14px;
	}

	.section {
		display: flex;
		flex-direction: column;
		gap: 10px;
	}

	.section-title {
		margin: 0;
		font-size: 11px;
		font-weight: 700;
		text-transform: uppercase;
		letter-spacing: 0.03em;
		color: var(--ink-faint);
	}

	.field {
		display: flex;
		flex-direction: column;
		gap: 5px;
	}

	.field-label {
		font-size: 12px;
		font-weight: 600;
		color: var(--ink-dim);
	}

	input[type="range"] {
		width: 100%;
		accent-color: var(--accent-fill);
	}

	.radio-list {
		display: flex;
		flex-direction: column;
		gap: 2px;
		border-radius: 6px;
		overflow: hidden;
	}

	.radio-row {
		display: flex;
		align-items: center;
		justify-content: space-between;
		padding: 7px 10px;
		background: var(--active);
		color: var(--ink-dim);
		font-size: 13px;
	}

	.radio-row:hover {
		background: var(--hover);
	}

	.radio-row.active {
		color: var(--ink);
		font-weight: 600;
	}

	.switch-row {
		display: flex;
		align-items: center;
		justify-content: space-between;
	}

	.switch {
		position: relative;
		width: 34px;
		height: 20px;
		flex-shrink: 0;
	}

	.switch input {
		position: absolute;
		opacity: 0;
		width: 100%;
		height: 100%;
		margin: 0;
		cursor: pointer;
	}

	.switch .track {
		display: block;
		width: 100%;
		height: 100%;
		border-radius: 999px;
		background: var(--active);
		transition: background-color 0.15s ease;
	}

	.switch .thumb {
		position: absolute;
		top: 2px;
		left: 2px;
		width: 16px;
		height: 16px;
		border-radius: 50%;
		background: var(--ink-dim);
		transition: transform 0.15s ease, background-color 0.15s ease;
	}

	.switch input:checked + .track {
		background: var(--online);
	}

	.switch input:checked + .track .thumb {
		transform: translateX(14px);
		background: white;
	}

	.hint {
		margin: 0;
		font-size: 11px;
		color: var(--ink-faint);
	}

	.hint.warn {
		color: var(--idle);
	}

	.keybind-btn {
		width: 100%;
		padding: 7px 9px;
		border-radius: 6px;
		background: var(--active);
		color: var(--ink);
		font-size: 13px;
		font-weight: 600;
		text-align: center;
		transition: background-color 0.15s ease, color 0.15s ease;
	}

	.keybind-btn:hover {
		background: var(--hover);
	}

	.keybind-btn.capturing {
		background: var(--accent-fill);
		color: var(--accent-fill-ink);
	}

	.full-settings-link {
		padding: 8px 10px;
		border-radius: 6px;
		background: var(--active);
		color: var(--ink-dim);
		font-size: 12px;
		font-weight: 700;
		text-align: center;
		transition: background-color 0.15s ease, color 0.15s ease;
	}

	.full-settings-link:hover {
		background: var(--hover);
		color: var(--ink);
	}
</style>

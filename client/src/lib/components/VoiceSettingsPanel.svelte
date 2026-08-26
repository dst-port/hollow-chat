<script lang="ts">
	import { onMount, onDestroy } from "svelte";
	import Check from "@lucide/svelte/icons/check";
	import Dropdown from "$lib/components/Dropdown.svelte";
	import { call } from "$lib/webrtc/call.svelte";

	let { focus = "all", onOpenFullSettings }: {
		focus?: "input" | "output" | "all";
		onOpenFullSettings?: () => void;
	} = $props();

	onMount(() => {
		call.refreshDevices();
	});

	const NAMED_KEYS: Record<string, string> = {
		Space: "Space",
		ControlLeft: "Left Ctrl",
		ControlRight: "Right Ctrl",
		AltLeft: "Left Alt",
		AltRight: "Right Alt",
		ShiftLeft: "Left Shift",
		ShiftRight: "Right Shift",
		CapsLock: "Caps Lock",
		Backquote: "`",
		Tab: "Tab",
		MetaLeft: "Left Super",
		MetaRight: "Right Super"
	};

	function keyLabel(code: string): string {
		if (NAMED_KEYS[code]) return NAMED_KEYS[code];
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
			{#if focus === "all"}<p class="section-title">Input</p>{/if}

			<div class="field">
				<span class="field-label">Input Device</span>
				<Dropdown
					value={call.inputDeviceId ?? ""}
					options={[
						{ value: "", label: "System Default" },
						...call.inputDevices.map((d) => ({ value: d.deviceId, label: d.label || "Microphone" }))
					]}
					onChange={(v) => call.setInputDevice(v || null)}
				/>
			</div>

			<div class="field">
				<span class="field-label">Noise Suppression</span>
				{#if call.noiseSuppressionSupported}
					<div class="radio-list">
						<button
							type="button"
							class="radio-row"
							class:active={!call.noiseSuppression}
							onclick={() => call.setNoiseSuppression(false)}
						>
							<span>Off</span>
							{#if !call.noiseSuppression}<Check size={14} strokeWidth={2.5} />{/if}
						</button>
						<button
							type="button"
							class="radio-row"
							class:active={call.noiseSuppression}
							onclick={() => call.setNoiseSuppression(true)}
						>
							<span>On</span>
							{#if call.noiseSuppression}<Check size={14} strokeWidth={2.5} />{/if}
						</button>
					</div>
					{#if call.noiseSuppression}
						<p class="hint" class:warn={!call.noiseSuppressionActive}>
							{call.noiseSuppressionActive
								? "Active — applied by your OS/driver on the raw mic signal."
								: "Requested, but your mic/driver didn't confirm it's actually applying it."}
						</p>
					{/if}
				{:else}
					<p class="hint warn">Not supported by this device or platform.</p>
				{/if}
			</div>

			<label class="switch-row">
				<span class="field-label">Push to Talk</span>
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
					<span class="field-label">Push to Talk Key</span>
					<button type="button" class="keybind-btn" class:capturing onclick={startCapture}>
						{capturing ? "Press any key… (Esc to cancel)" : keyLabel(call.pushToTalkKey)}
					</button>
				</div>
				{#if !capturing}<p class="hint">Hold {keyLabel(call.pushToTalkKey)} to transmit.</p>{/if}
			{/if}
		</div>
	{/if}

	{#if focus === "output" || focus === "all"}
		<div class="section">
			{#if focus === "all"}<p class="section-title">Output</p>{/if}

			<div class="field">
				<span class="field-label">Output Device</span>
				{#if call.outputDeviceSelectionSupported}
					<Dropdown
						value={call.outputDeviceId ?? ""}
						options={[
							{ value: "", label: "System Default" },
							...(call.nativeOutputRouting
								? call.nativeSinks.map((s) => ({ value: s.name, label: s.description }))
								: call.outputDevices.map((d) => ({ value: d.deviceId, label: d.label || "Speaker" })))
						]}
						onChange={(v) => call.setOutputDevice(v || null)}
					/>
				{:else}
					<p class="hint warn">
						This platform can't switch output device per-app — change it in your system's sound
						settings instead.
					</p>
				{/if}
			</div>

			<label class="field">
				<span class="field-label">Output Volume</span>
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
		<button type="button" class="full-settings-link" onclick={onOpenFullSettings}>Voice Settings</button>
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

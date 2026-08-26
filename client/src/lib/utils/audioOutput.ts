export type AudioSink = { name: string; description: string };

let platformChecked = false;
let isLinux = false;

async function ensurePlatformChecked(): Promise<void> {
	if (platformChecked) return;
	platformChecked = true;
	try {
		const { platform } = await import("@tauri-apps/plugin-os");
		isLinux = (await platform()) === "linux";
	} catch {
		isLinux = false;
	}
}

/// True only inside the Tauri shell on Linux - the only place setSinkId
/// doesn't exist and this native PipeWire/PulseAudio routing is wired up.
export async function nativeOutputRoutingAvailable(): Promise<boolean> {
	await ensurePlatformChecked();
	return isLinux;
}

export async function listNativeAudioSinks(): Promise<AudioSink[]> {
	if (!(await nativeOutputRoutingAvailable())) return [];
	try {
		const { invoke } = await import("@tauri-apps/api/core");
		return await invoke<AudioSink[]>("list_audio_sinks");
	} catch {
		return [];
	}
}

export async function getNativeDefaultSink(): Promise<string | null> {
	if (!(await nativeOutputRoutingAvailable())) return null;
	try {
		const { invoke } = await import("@tauri-apps/api/core");
		return await invoke<string>("get_default_audio_sink");
	} catch {
		return null;
	}
}

export async function setNativeAppAudioSink(sinkName: string): Promise<void> {
	if (!(await nativeOutputRoutingAvailable())) return;
	try {
		const { invoke } = await import("@tauri-apps/api/core");
		await invoke("set_app_audio_sink", { sinkName });
	} catch {
		// no active stream yet, or pactl unavailable - the preference is
		// still saved and gets re-applied next time a stream opens
	}
}

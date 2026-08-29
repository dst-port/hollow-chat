<script lang="ts">
	import Play from "@lucide/svelte/icons/play";
	import Pause from "@lucide/svelte/icons/pause";
	import Volume2 from "@lucide/svelte/icons/volume-2";
	import VolumeX from "@lucide/svelte/icons/volume-x";
	import Music from "@lucide/svelte/icons/music";

	let { src, filename, sizeBytes }: {
		src: string;
		filename: string;
		sizeBytes: number;
	} = $props();

	let audioEl: HTMLAudioElement | undefined;
	let playing = $state(false);
	let currentTime = $state(0);
	let duration = $state(0);
	let muted = $state(false);

	function formatSize(bytes: number): string {
		if (bytes < 1024) return `${bytes} B`;
		if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`;
		if (bytes < 1024 * 1024 * 1024) return `${(bytes / (1024 * 1024)).toFixed(1)} MB`;
		return `${(bytes / (1024 * 1024 * 1024)).toFixed(2)} GB`;
	}

	function formatTime(seconds: number): string {
		if (!Number.isFinite(seconds)) return "0:00";
		const m = Math.floor(seconds / 60);
		const s = Math.floor(seconds % 60);
		return `${m}:${s.toString().padStart(2, "0")}`;
	}

	function toggle() {
		if (!audioEl) return;
		if (playing) audioEl.pause();
		else audioEl.play();
	}

	function seek(event: Event) {
		if (!audioEl) return;
		const value = Number((event.currentTarget as HTMLInputElement).value);
		audioEl.currentTime = value;
		currentTime = value;
	}

	function toggleMute() {
		if (!audioEl) return;
		audioEl.muted = !audioEl.muted;
		muted = audioEl.muted;
	}
</script>

<div class="audio-card">
	<audio
		bind:this={audioEl}
		{src}
		preload="metadata"
		onplay={() => (playing = true)}
		onpause={() => (playing = false)}
		onended={() => (playing = false)}
		ontimeupdate={() => (currentTime = audioEl?.currentTime ?? 0)}
		onloadedmetadata={() => (duration = audioEl?.duration ?? 0)}
	></audio>

	<div class="audio-icon"><Music size={20} strokeWidth={2} /></div>

	<div class="audio-body">
		<span class="audio-filename">{filename}</span>
		<span class="audio-size">{formatSize(sizeBytes)}</span>
		<div class="audio-controls">
			<button type="button" class="audio-play" onclick={toggle} title={playing ? "Pause" : "Play"}>
				{#if playing}
					<Pause size={14} strokeWidth={2} fill="currentColor" />
				{:else}
					<Play size={14} strokeWidth={2} fill="currentColor" />
				{/if}
			</button>
			<span class="audio-time">{formatTime(currentTime)} / {formatTime(duration)}</span>
			<input
				class="audio-seek"
				type="range"
				min="0"
				max={duration || 0}
				step="0.1"
				value={currentTime}
				oninput={seek}
			/>
			<button type="button" class="audio-mute" onclick={toggleMute} title={muted ? "Unmute" : "Mute"}>
				{#if muted}
					<VolumeX size={16} strokeWidth={2} />
				{:else}
					<Volume2 size={16} strokeWidth={2} />
				{/if}
			</button>
		</div>
	</div>
</div>

<style>
	.audio-card {
		display: flex;
		align-items: center;
		gap: 12px;
		margin-top: 4px;
		padding: 10px 12px;
		max-width: 360px;
		background: var(--sidebar);
		border-radius: 8px;
	}

	.audio-icon {
		flex-shrink: 0;
		width: 40px;
		height: 40px;
		border-radius: 8px;
		display: flex;
		align-items: center;
		justify-content: center;
		background: var(--accent-fill);
		color: var(--accent-fill-ink);
	}

	.audio-body {
		flex: 1;
		min-width: 0;
		display: flex;
		flex-direction: column;
	}

	.audio-filename {
		font-size: 13px;
		font-weight: 600;
		color: var(--link, #6ea8fe);
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
	}

	.audio-size {
		font-size: 11px;
		color: var(--ink-faint);
		margin-bottom: 4px;
	}

	.audio-controls {
		display: flex;
		align-items: center;
		gap: 8px;
	}

	.audio-play,
	.audio-mute {
		flex-shrink: 0;
		display: flex;
		align-items: center;
		justify-content: center;
		width: 26px;
		height: 26px;
		border-radius: 50%;
		background: var(--hover);
		color: var(--ink);
	}

	.audio-play:hover,
	.audio-mute:hover {
		background: var(--active);
	}

	.audio-time {
		flex-shrink: 0;
		font-size: 11px;
		font-variant-numeric: tabular-nums;
		color: var(--ink-faint);
	}

	.audio-seek {
		flex: 1;
		min-width: 0;
		height: 4px;
		accent-color: var(--accent-fill);
	}
</style>

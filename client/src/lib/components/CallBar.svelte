<script lang="ts">
	import { fly } from "svelte/transition";
	import Mic from "@lucide/svelte/icons/mic";
	import MicOff from "@lucide/svelte/icons/mic-off";
	import Video from "@lucide/svelte/icons/video";
	import VideoOff from "@lucide/svelte/icons/video-off";
	import ScreenShare from "@lucide/svelte/icons/screen-share";
	import ScreenShareOff from "@lucide/svelte/icons/screen-share-off";
	import PhoneOff from "@lucide/svelte/icons/phone-off";
	import { call } from "$lib/webrtc/call.svelte";
	import {
		attachRemoteStream,
		attachLocalStream,
		attachRemoteScreenStream,
		attachLocalScreenStream
	} from "$lib/actions/attachStream";
	import { colorForName } from "$lib/utils/color";

	const sharingUserIds = $derived.by(() => {
		call.streamsVersion;
		return new Set(call.participants.filter((p) => call.getRemoteScreenStream(p.userId)).map((p) => p.userId));
	});
</script>

{#if call.status !== "idle"}
	<div class="call-bar" transition:fly={{ y: 12, duration: 160 }}>
		<div class="call-header">
			<span class="status-dot" class:connecting={call.status === "connecting"}></span>
			<span class="label">{call.label}</span>
			<span class="status-text">{call.status === "connecting" ? "Connecting…" : "Voice Connected"}</span>
		</div>

		{#if call.screenSharing}
			<div class="screen-tile">
				<video use:attachLocalScreenStream autoplay playsinline muted></video>
				<span class="tile-name">Your screen</span>
			</div>
		{/if}
		{#each call.participants as participant (participant.userId)}
			{#if sharingUserIds.has(participant.userId)}
				<div class="screen-tile">
					<video use:attachRemoteScreenStream={participant.userId} autoplay playsinline></video>
					<span class="tile-name">{participant.username}'s screen</span>
				</div>
			{/if}
		{/each}

		<div class="tiles">
			<div class="tile">
				{#if call.cameraEnabled}
					<video use:attachLocalStream autoplay playsinline muted></video>
				{:else}
					<div class="avatar" style:background={colorForName("you")}>You</div>
				{/if}
				<span class="tile-name">You {call.muted ? "(muted)" : ""}</span>
			</div>
			{#each call.participants as participant (participant.userId)}
				<div class="tile">
					<video use:attachRemoteStream={participant.userId} autoplay playsinline></video>
					<audio use:attachRemoteStream={participant.userId} autoplay></audio>
					<span class="tile-name">{participant.username}</span>
				</div>
			{/each}
		</div>

		<div class="controls">
			<button class="control" class:active={call.muted} title={call.muted ? "Unmute" : "Mute"} onclick={() => call.toggleMute()}>
				{#if call.muted}<MicOff size={16} strokeWidth={2} />{:else}<Mic size={16} strokeWidth={2} />{/if}
			</button>
			<button class="control" class:active={call.cameraEnabled} title={call.cameraEnabled ? "Turn off camera" : "Turn on camera"} onclick={() => call.toggleCamera()}>
				{#if call.cameraEnabled}<Video size={16} strokeWidth={2} />{:else}<VideoOff size={16} strokeWidth={2} />{/if}
			</button>
			<button class="control" class:active={call.screenSharing} title={call.screenSharing ? "Stop screen share" : "Share your screen"} onclick={() => call.toggleScreenShare()}>
				{#if call.screenSharing}<ScreenShareOff size={16} strokeWidth={2} />{:else}<ScreenShare size={16} strokeWidth={2} />{/if}
			</button>
			<button class="control leave" title="Leave call" onclick={() => call.leave()}>
				<PhoneOff size={16} strokeWidth={2} />
			</button>
		</div>
	</div>
{/if}

<style>
	.call-bar {
		flex-shrink: 0;
		display: flex;
		flex-direction: column;
		gap: 10px;
		padding: 10px 12px;
		margin: 8px;
		background: var(--sidebar);
		border-radius: 10px;
	}

	.call-header {
		display: flex;
		align-items: center;
		gap: 8px;
	}

	.status-dot {
		width: 8px;
		height: 8px;
		border-radius: 50%;
		background: #3ba55d;
		flex-shrink: 0;
	}

	.status-dot.connecting {
		background: #f0b232;
	}

	.label {
		font-size: 13px;
		font-weight: 700;
		color: var(--ink);
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
	}

	.status-text {
		margin-left: auto;
		font-size: 11px;
		color: var(--ink-faint);
	}

	.screen-tile {
		position: relative;
		width: 100%;
		aspect-ratio: 16 / 9;
		border-radius: 8px;
		overflow: hidden;
		background: var(--void);
	}

	.screen-tile video {
		width: 100%;
		height: 100%;
		object-fit: contain;
		background: black;
	}

	.screen-tile .tile-name {
		position: absolute;
		left: 6px;
		bottom: 5px;
		font-size: 11px;
		font-weight: 600;
		color: white;
		text-shadow: 0 1px 2px rgba(0, 0, 0, 0.8);
	}

	.tiles {
		display: flex;
		flex-wrap: wrap;
		gap: 8px;
	}

	.tile {
		position: relative;
		width: 96px;
		height: 64px;
		border-radius: 8px;
		overflow: hidden;
		background: var(--void);
		display: flex;
		align-items: center;
		justify-content: center;
	}

	.tile video {
		width: 100%;
		height: 100%;
		object-fit: cover;
	}

	.tile audio {
		display: none;
	}

	.avatar {
		width: 32px;
		height: 32px;
		border-radius: 50%;
		display: flex;
		align-items: center;
		justify-content: center;
		font-size: 10px;
		font-weight: 700;
		color: var(--void);
	}

	.tile-name {
		position: absolute;
		left: 4px;
		bottom: 3px;
		font-size: 10px;
		font-weight: 600;
		color: white;
		text-shadow: 0 1px 2px rgba(0, 0, 0, 0.8);
	}

	.controls {
		display: flex;
		gap: 6px;
	}

	.control {
		flex: 1;
		display: flex;
		align-items: center;
		justify-content: center;
		padding: 8px;
		border-radius: 6px;
		background: var(--active);
		color: var(--ink-dim);
		transition: background-color 0.15s ease, color 0.15s ease;
	}

	.control:hover {
		background: var(--hover);
		color: var(--ink);
	}

	.control.active {
		background: var(--accent-fill);
		color: var(--accent-fill-ink);
	}

	.control.leave {
		background: var(--danger);
		color: white;
	}

	.control.leave:hover {
		filter: brightness(1.1);
	}
</style>

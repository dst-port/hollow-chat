<script lang="ts">
	import { fly } from "svelte/transition";
	import Mic from "@lucide/svelte/icons/mic";
	import MicOff from "@lucide/svelte/icons/mic-off";
	import Video from "@lucide/svelte/icons/video";
	import VideoOff from "@lucide/svelte/icons/video-off";
	import ScreenShare from "@lucide/svelte/icons/screen-share";
	import ScreenShareOff from "@lucide/svelte/icons/screen-share-off";
	import PhoneOff from "@lucide/svelte/icons/phone-off";
	import Users from "@lucide/svelte/icons/users";
	import Sparkles from "@lucide/svelte/icons/sparkles";
	import { call } from "$lib/webrtc/call.svelte";
	import {
		attachRemoteStream,
		attachLocalStream,
		attachRemoteScreenStream,
		attachLocalScreenStream
	} from "$lib/actions/attachStream";
	import { session } from "$lib/stores/session.svelte";
	import { profileStore } from "$lib/stores/profile.svelte";

	$effect(() => {
		const token = session.token;
		const username = session.username;
		if (!token || !username) return;
		profileStore.load(token, username);
		for (const participant of call.participants) {
			profileStore.load(token, participant.username);
		}
	});

	const sharingUserIds = $derived.by(() => {
		call.streamsVersion;
		return new Set(call.participants.filter((p) => call.getRemoteScreenStream(p.userId)).map((p) => p.userId));
	});

	const camUserIds = $derived.by(() => {
		call.streamsVersion;
		return new Set(
			call.participants
				.filter((p) => (call.getRemoteStream(p.userId)?.getVideoTracks().length ?? 0) > 0)
				.map((p) => p.userId)
		);
	});

	let showParticipants = $state(false);
</script>

{#if call.status !== "idle"}
	<div class="call-bar" transition:fly={{ y: 12, duration: 160 }}>
		<div class="call-header">
			<span class="status-dot" class:connecting={call.status === "connecting"}></span>
			<div class="status-text">
				<span class="status-title">{call.status === "connecting" ? "Connecting…" : "Voice Connected"}</span>
				<span class="status-sub">{call.label}</span>
			</div>
			<button class="mini-icon" class:muted-active={call.muted} aria-label={call.muted ? "Unmute" : "Mute"} onclick={() => call.toggleMute()}>
				{#if call.muted}<MicOff size={14} strokeWidth={2} />{:else}<Mic size={14} strokeWidth={2} />{/if}
				<span class="tooltip">{call.muted ? "Unmute" : "Mute"}</span>
			</button>
			<button class="mini-icon leave" aria-label="Disconnect" onclick={() => call.leave()}>
				<PhoneOff size={14} strokeWidth={2} />
				<span class="tooltip">Disconnect</span>
			</button>
		</div>

		<div class="feature-row">
			<button class="feature-btn" class:active={call.cameraEnabled} aria-label={call.cameraEnabled ? "Turn Off Camera" : "Turn On Camera"} onclick={() => call.toggleCamera()}>
				{#if call.cameraEnabled}<Video size={14} strokeWidth={2} />{:else}<VideoOff size={14} strokeWidth={2} />{/if}
				<span class="tooltip">{call.cameraEnabled ? "Turn Off Camera" : "Turn On Camera"}</span>
			</button>
			<button class="feature-btn" class:active={call.screenSharing} aria-label={call.screenSharing ? "Stop Sharing" : "Share Your Screen"} onclick={() => call.toggleScreenShare()}>
				{#if call.screenSharing}<ScreenShareOff size={14} strokeWidth={2} />{:else}<ScreenShare size={14} strokeWidth={2} />{/if}
				<span class="tooltip">{call.screenSharing ? "Stop Sharing" : "Share Your Screen"}</span>
			</button>
			<button class="feature-btn" aria-label="Participants">
				<Users size={14} strokeWidth={2} />
				<span class="tooltip">Participants</span>
			</button>
			<button class="feature-btn" aria-label="Effects">
				<Sparkles size={14} strokeWidth={2} />
				<span class="tooltip">Effects</span>
			</button>
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

		{#if call.cameraEnabled || camUserIds.size > 0}
			<div class="tiles">
				{#if call.cameraEnabled}
					<div class="tile">
						<video use:attachLocalStream autoplay playsinline muted></video>
						<span class="tile-name">You</span>
					</div>
				{/if}
				{#each call.participants as participant (participant.userId)}
					{#if camUserIds.has(participant.userId)}
						{@const remoteProfile = profileStore.forUser(participant.username)}
						<div class="tile">
							<video use:attachRemoteStream={participant.userId} autoplay playsinline muted></video>
							<span class="tile-name">{remoteProfile?.display_name || participant.username}</span>
						</div>
					{/if}
				{/each}
			</div>
		{/if}

		{#each call.participants as participant (participant.userId)}
			<audio use:attachRemoteStream={participant.userId} autoplay muted={call.deafened}></audio>
		{/each}
	</div>
{/if}

<style>
	.call-bar {
		flex-shrink: 0;
		display: flex;
		flex-direction: column;
		gap: 8px;
		padding: 10px 12px;
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
		background: var(--online);
		flex-shrink: 0;
	}

	.status-dot.connecting {
		background: var(--idle);
	}

	.status-text {
		flex: 1;
		min-width: 0;
		display: flex;
		flex-direction: column;
	}

	.status-title {
		font-size: 12px;
		font-weight: 700;
		color: var(--online);
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
	}

	.status-sub {
		font-size: 11px;
		color: var(--ink-faint);
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
	}

	.mini-icon {
		position: relative;
		flex-shrink: 0;
		display: flex;
		align-items: center;
		justify-content: center;
		padding: 6px;
		border-radius: 999px;
		color: var(--ink-dim);
		transition: background-color 0.15s ease, color 0.15s ease;
	}

	.tooltip {
		position: absolute;
		bottom: calc(100% + 8px);
		left: 50%;
		transform: translateX(-50%);
		padding: 6px 10px;
		border-radius: 6px;
		background: var(--void);
		color: var(--ink);
		font-size: 12px;
		font-weight: 700;
		white-space: nowrap;
		box-shadow: 0 4px 12px rgba(0, 0, 0, 0.45);
		opacity: 0;
		pointer-events: none;
		transition: opacity 0.1s ease;
		z-index: 30;
	}

	.tooltip::after {
		content: "";
		position: absolute;
		top: 100%;
		left: 50%;
		transform: translateX(-50%);
		border: 5px solid transparent;
		border-top-color: var(--void);
	}

	.mini-icon:hover .tooltip,
	.feature-btn:hover .tooltip {
		opacity: 1;
	}

	.mini-icon:hover {
		background: var(--hover);
		color: var(--ink);
	}

	.mini-icon.muted-active {
		color: var(--danger);
	}

	.mini-icon.leave {
		color: var(--danger);
	}

	.mini-icon.leave:hover {
		background: var(--danger);
		color: white;
	}

	.feature-row {
		display: flex;
		gap: 8px;
	}

	.feature-btn {
		position: relative;
		flex: 1;
		height: 32px;
		display: flex;
		align-items: center;
		justify-content: center;
		border-radius: 8px;
		background: var(--active);
		color: var(--ink-dim);
		transition: background-color 0.15s ease, color 0.15s ease;
	}

	.feature-btn:hover {
		background: var(--hover);
		color: var(--ink);
	}

	.feature-btn.active {
		background: var(--accent-fill);
		color: var(--accent-fill-ink);
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

	.call-bar > audio {
		display: none;
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

</style>

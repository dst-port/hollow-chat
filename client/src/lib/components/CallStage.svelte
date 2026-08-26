<script lang="ts">
	import { fade } from "svelte/transition";
	import Mic from "@lucide/svelte/icons/mic";
	import MicOff from "@lucide/svelte/icons/mic-off";
	import Headphones from "@lucide/svelte/icons/headphones";
	import HeadphoneOff from "@lucide/svelte/icons/headphone-off";
	import Video from "@lucide/svelte/icons/video";
	import VideoOff from "@lucide/svelte/icons/video-off";
	import ScreenShare from "@lucide/svelte/icons/screen-share";
	import ScreenShareOff from "@lucide/svelte/icons/screen-share-off";
	import PhoneOff from "@lucide/svelte/icons/phone-off";
	import Volume2 from "@lucide/svelte/icons/volume-2";
	import { call } from "$lib/webrtc/call.svelte";
	import {
		attachRemoteStream,
		attachLocalStream,
		attachRemoteScreenStream,
		attachLocalScreenStream
	} from "$lib/actions/attachStream";
	import { session } from "$lib/stores/session.svelte";
	import { profileStore } from "$lib/stores/profile.svelte";
	import * as api from "$lib/api/client";
	import type { Channel, ServerEntry } from "$lib/data/mock";

	let { server, channel, onJoin }: {
		server: ServerEntry;
		channel: Channel;
		onJoin: () => void;
	} = $props();

	const joined = $derived(call.roomId === channel.id);

	$effect(() => {
		const token = session.token;
		if (!token) return;
		profileStore.load(token, session.username ?? "");
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

	const ownProfile = $derived(profileStore.forUser(session.username ?? ""));

	const stageCount = $derived(1 + call.participants.length);
	const gridCols = $derived(
		stageCount <= 1 ? 1 : stageCount <= 4 ? 2 : stageCount <= 9 ? 3 : 4
	);

	function initials(name: string) {
		return name.slice(0, 2).toUpperCase();
	}
</script>

{#if !joined}
	<div class="prejoin" transition:fade={{ duration: 140 }}>
		<div class="prejoin-icon"><Volume2 size={40} strokeWidth={1.5} /></div>
		<h2>{channel.name}</h2>
		<p>No one is connected to this voice channel from this window.</p>
		<button class="join-btn" onclick={onJoin}>Join Voice</button>
	</div>
{:else}
	<div class="stage" transition:fade={{ duration: 140 }}>
		<div class="stage-header">
			<Volume2 size={16} strokeWidth={2.25} />
			<span>{server.name} / {channel.name}</span>
		</div>

		{#if call.screenSharing || sharingUserIds.size > 0}
			<div class="screen-row">
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
			</div>
		{/if}

		<div class="grid" style:grid-template-columns={`repeat(${gridCols}, 1fr)`}>
			<div class="cell" class:speaking={call.selfSpeaking && !call.muted}>
				{#if call.cameraEnabled}
					<video class="cell-video" use:attachLocalStream autoplay playsinline muted></video>
				{:else}
					<div
						class="cell-avatar"
						style:background={ownProfile?.avatar_url ? undefined : "var(--accent-fill)"}
						style:background-image={ownProfile?.avatar_url
							? `url(${api.resolveUrl(ownProfile.avatar_url, session.token)})`
							: undefined}
					>
						{#if !ownProfile?.avatar_url}<span>{initials(session.username ?? "")}</span>{/if}
					</div>
				{/if}
				<div class="cell-tag">
					{#if call.muted}<MicOff size={13} strokeWidth={2.25} />{:else}<Mic size={13} strokeWidth={2.25} />{/if}
					<span>{ownProfile?.display_name || session.username} (you)</span>
				</div>
			</div>

			{#each call.participants as participant (participant.userId)}
				{@const remoteProfile = profileStore.forUser(participant.username)}
				<div class="cell" class:speaking={call.speakingUserIds.has(participant.userId)}>
					{#if camUserIds.has(participant.userId)}
						<video class="cell-video" use:attachRemoteStream={participant.userId} autoplay playsinline muted></video>
					{:else}
						<div
							class="cell-avatar"
							style:background={remoteProfile?.avatar_url ? undefined : "var(--accent-fill)"}
							style:background-image={remoteProfile?.avatar_url
								? `url(${api.resolveUrl(remoteProfile.avatar_url, session.token)})`
								: undefined}
						>
							{#if !remoteProfile?.avatar_url}<span>{initials(participant.username)}</span>{/if}
						</div>
					{/if}
					<div class="cell-tag">
						<span>{remoteProfile?.display_name || participant.username}</span>
					</div>
				</div>
			{/each}
		</div>

		<div class="controls">
			<button class="ctrl" class:active-danger={call.muted} aria-label={call.muted ? "Unmute" : "Mute"} onclick={() => call.toggleMute()}>
				{#if call.muted}<MicOff size={18} strokeWidth={2} />{:else}<Mic size={18} strokeWidth={2} />{/if}
			</button>
			<button class="ctrl" class:active-danger={call.deafened} aria-label={call.deafened ? "Undeafen" : "Deafen"} onclick={() => call.toggleDeafen()}>
				{#if call.deafened}<HeadphoneOff size={18} strokeWidth={2} />{:else}<Headphones size={18} strokeWidth={2} />{/if}
			</button>
			<button class="ctrl" class:active={call.cameraEnabled} aria-label={call.cameraEnabled ? "Turn Off Camera" : "Turn On Camera"} onclick={() => call.toggleCamera()}>
				{#if call.cameraEnabled}<Video size={18} strokeWidth={2} />{:else}<VideoOff size={18} strokeWidth={2} />{/if}
			</button>
			<button class="ctrl" class:active={call.screenSharing} aria-label={call.screenSharing ? "Stop Sharing" : "Share Your Screen"} onclick={() => call.toggleScreenShare()}>
				{#if call.screenSharing}<ScreenShareOff size={18} strokeWidth={2} />{:else}<ScreenShare size={18} strokeWidth={2} />{/if}
			</button>
			<button class="ctrl leave" aria-label="Disconnect" onclick={() => call.leave()}>
				<PhoneOff size={18} strokeWidth={2} />
			</button>
		</div>

		{#each call.participants as participant (participant.userId)}
			<audio use:attachRemoteStream={participant.userId} autoplay muted={call.deafened}></audio>
		{/each}
	</div>
{/if}

<style>
	.prejoin {
		flex: 1;
		display: flex;
		flex-direction: column;
		align-items: center;
		justify-content: center;
		gap: 10px;
		color: var(--ink-dim);
		text-align: center;
		padding: 32px;
	}

	.prejoin-icon {
		width: 88px;
		height: 88px;
		border-radius: 50%;
		display: flex;
		align-items: center;
		justify-content: center;
		background: var(--active);
		color: var(--ink-faint);
		margin-bottom: 8px;
	}

	.prejoin h2 {
		margin: 0;
		font-family: var(--font-display);
		font-size: 22px;
		color: var(--ink);
	}

	.prejoin p {
		margin: 0;
		font-size: 13px;
		color: var(--ink-faint);
		max-width: 320px;
	}

	.join-btn {
		margin-top: 10px;
		padding: 10px 22px;
		border-radius: 8px;
		background: var(--online);
		color: var(--void);
		font-weight: 700;
		font-size: 13px;
		transition: filter 0.15s ease;
	}

	.join-btn:hover {
		filter: brightness(1.08);
	}

	.stage {
		flex: 1;
		display: flex;
		flex-direction: column;
		min-height: 0;
		min-width: 0;
		background: var(--void);
	}

	.stage-header {
		flex-shrink: 0;
		display: flex;
		align-items: center;
		gap: 8px;
		padding: 14px 20px;
		border-bottom: 1px solid var(--hairline);
		color: var(--ink);
		font-weight: 700;
		font-size: 13px;
	}

	.screen-row {
		flex-shrink: 0;
		display: flex;
		gap: 10px;
		padding: 16px 20px 0;
		flex-wrap: wrap;
	}

	.screen-tile {
		position: relative;
		flex: 1;
		min-width: 320px;
		aspect-ratio: 16 / 9;
		border-radius: 10px;
		overflow: hidden;
		background: black;
	}

	.screen-tile video {
		width: 100%;
		height: 100%;
		object-fit: contain;
	}

	.screen-tile .tile-name {
		position: absolute;
		left: 10px;
		bottom: 8px;
		font-size: 12px;
		font-weight: 600;
		color: white;
		text-shadow: 0 1px 3px rgba(0, 0, 0, 0.85);
	}

	.grid {
		flex: 1;
		min-height: 0;
		display: grid;
		gap: 14px;
		padding: 20px;
		align-content: center;
		overflow-y: auto;
	}

	.cell {
		position: relative;
		aspect-ratio: 16 / 10;
		border-radius: 12px;
		background: var(--sidebar);
		display: flex;
		align-items: center;
		justify-content: center;
		overflow: hidden;
		box-shadow: 0 0 0 2px transparent;
		transition: box-shadow 0.12s ease;
	}

	.cell.speaking {
		box-shadow: 0 0 0 2px var(--online);
	}

	.cell-video {
		width: 100%;
		height: 100%;
		object-fit: cover;
	}

	.cell-avatar {
		width: 30%;
		aspect-ratio: 1;
		max-width: 96px;
		border-radius: 50%;
		background-size: cover;
		background-position: center;
		display: flex;
		align-items: center;
		justify-content: center;
		font-family: var(--font-mono);
		font-weight: 700;
		font-size: 22px;
		color: var(--accent-fill-ink);
	}

	.cell-tag {
		position: absolute;
		left: 10px;
		bottom: 10px;
		display: flex;
		align-items: center;
		gap: 6px;
		padding: 4px 9px;
		border-radius: 6px;
		background: rgba(0, 0, 0, 0.55);
		color: white;
		font-size: 12px;
		font-weight: 600;
		max-width: calc(100% - 20px);
	}

	.cell-tag span {
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
	}

	.controls {
		flex-shrink: 0;
		display: flex;
		align-items: center;
		justify-content: center;
		gap: 10px;
		padding: 16px;
		border-top: 1px solid var(--hairline);
	}

	.ctrl {
		width: 44px;
		height: 44px;
		border-radius: 50%;
		display: flex;
		align-items: center;
		justify-content: center;
		background: var(--active);
		color: var(--ink);
		transition: background-color 0.15s ease, color 0.15s ease;
	}

	.ctrl:hover {
		background: var(--hover);
	}

	.ctrl.active {
		background: var(--accent-fill);
		color: var(--accent-fill-ink);
	}

	.ctrl.active-danger {
		background: var(--danger);
		color: white;
	}

	.ctrl.leave {
		background: var(--danger);
		color: white;
	}

	.ctrl.leave:hover {
		filter: brightness(1.1);
	}
</style>

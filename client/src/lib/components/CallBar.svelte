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
	import { call, type ScreenShareOpts } from "$lib/webrtc/call.svelte";
	import ScreenSharePicker from "$lib/components/ScreenSharePicker.svelte";
	import {
		attachRemoteStream,
		attachLocalStream,
		attachRemoteScreenStream,
		attachLocalScreenStream
	} from "$lib/actions/attachStream";
	import { session } from "$lib/stores/session.svelte";
	import { profileStore } from "$lib/stores/profile.svelte";
	import * as api from "$lib/api/client";
	import { t } from "$lib/i18n/index.svelte";

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
	let sharePickerOpen = $state(false);

	function onScreenShareClick() {
		if (call.screenSharing) call.toggleScreenShare();
		else sharePickerOpen = true;
	}

	function goLive(opts: ScreenShareOpts) {
		sharePickerOpen = false;
		call.toggleScreenShare(opts);
	}
</script>

{#if call.status !== "idle"}
	<div class="call-bar" transition:fly={{ y: 12, duration: 160 }}>
		<div class="call-header">
			<span class="status-dot" class:connecting={call.status === "connecting"}></span>
			<div class="status-text">
				<span class="status-title">{call.status === "connecting" ? t("call.connecting") : t("call.connected")}</span>
				<span class="status-sub">{call.label}</span>
			</div>
			<button class="mini-icon" class:muted-active={call.muted} aria-label={call.muted ? t("call.unmute") : t("call.mute")} onclick={() => call.toggleMute()}>
				{#if call.muted}<MicOff size={14} strokeWidth={2} />{:else}<Mic size={14} strokeWidth={2} />{/if}
				<span class="tooltip">{call.muted ? t("call.unmute") : t("call.mute")}</span>
			</button>
			<button class="mini-icon leave" aria-label={t("call.disconnect")} onclick={() => call.leave()}>
				<PhoneOff size={14} strokeWidth={2} />
				<span class="tooltip">{t("call.disconnect")}</span>
			</button>
		</div>

		<div class="feature-row">
			<button class="feature-btn" class:active={call.cameraEnabled} aria-label={call.cameraEnabled ? t("call.cameraOff") : t("call.cameraOn")} onclick={() => call.toggleCamera()}>
				{#if call.cameraEnabled}<Video size={14} strokeWidth={2} />{:else}<VideoOff size={14} strokeWidth={2} />{/if}
				<span class="tooltip">{call.cameraEnabled ? t("call.cameraOff") : t("call.cameraOn")}</span>
			</button>
			<button class="feature-btn" class:active={call.screenSharing} aria-label={call.screenSharing ? t("call.stopSharing") : t("call.shareScreen")} onclick={onScreenShareClick}>
				{#if call.screenSharing}<ScreenShareOff size={14} strokeWidth={2} />{:else}<ScreenShare size={14} strokeWidth={2} />{/if}
				<span class="tooltip">{call.screenSharing ? t("call.stopSharing") : t("call.shareScreen")}</span>
			</button>
			<button
				class="feature-btn"
				class:active={showParticipants}
				aria-label={t("call.participants")}
				onclick={() => (showParticipants = !showParticipants)}
			>
				<Users size={14} strokeWidth={2} />
				<span class="tooltip">{t("call.participants")}</span>
			</button>
		</div>

		{#if showParticipants}
			{@const ownProfile = profileStore.forUser(session.username ?? "")}
			<div class="participants-panel" transition:fly={{ y: -6, duration: 120 }}>
				<div class="participant-row">
					<div
						class="participant-avatar"
						class:speaking={call.selfSpeaking && !call.muted}
						style:background={ownProfile?.avatar_url ? undefined : "var(--accent-fill)"}
						style:background-image={ownProfile?.avatar_url
							? `url(${api.resolveUrl(ownProfile.avatar_url, session.token)})`
							: undefined}
					>
						{#if !ownProfile?.avatar_url}{(session.username ?? "").slice(0, 2).toUpperCase()}{/if}
					</div>
					<span class="participant-name">{ownProfile?.display_name || session.username} (you)</span>
					{#if call.muted}<MicOff size={13} strokeWidth={2} class="participant-muted" />{/if}
				</div>
				{#each call.participants as participant (participant.userId)}
					{@const remoteProfile = profileStore.forUser(participant.username)}
					<div class="participant-row">
						<div
							class="participant-avatar"
							class:speaking={call.speakingUserIds.has(participant.userId)}
							style:background={remoteProfile?.avatar_url ? undefined : "var(--accent-fill)"}
							style:background-image={remoteProfile?.avatar_url
								? `url(${api.resolveUrl(remoteProfile.avatar_url, session.token)})`
								: undefined}
						>
							{#if !remoteProfile?.avatar_url}{participant.username.slice(0, 2).toUpperCase()}{/if}
						</div>
						<span class="participant-name">{remoteProfile?.display_name || participant.username}</span>
					</div>
				{/each}
			</div>
		{/if}

		{#if call.screenSharing}
			<div class="screen-tile">
				<video use:attachLocalScreenStream autoplay playsinline muted></video>
				<span class="tile-name">{t("call.yourScreen")}</span>
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

{#if sharePickerOpen}
	<ScreenSharePicker onCancel={() => (sharePickerOpen = false)} onGoLive={goLive} />
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

	.participants-panel {
		display: flex;
		flex-direction: column;
		gap: 4px;
		padding: 6px;
		border-radius: 8px;
		background: var(--active);
	}

	.participant-row {
		display: flex;
		align-items: center;
		gap: 8px;
		padding: 4px;
	}

	.participant-avatar {
		flex-shrink: 0;
		width: 22px;
		height: 22px;
		border-radius: 50%;
		background-size: cover;
		background-position: center;
		display: flex;
		align-items: center;
		justify-content: center;
		font-size: 9px;
		font-weight: 700;
		color: var(--accent-fill-ink);
	}

	.participant-avatar.speaking {
		box-shadow: 0 0 0 2px var(--active), 0 0 0 4px var(--online);
	}

	.participant-name {
		flex: 1;
		min-width: 0;
		font-size: 12px;
		color: var(--ink);
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
	}

	.participant-row :global(.participant-muted) {
		color: var(--danger);
		flex-shrink: 0;
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

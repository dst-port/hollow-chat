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
	import Maximize2 from "@lucide/svelte/icons/maximize-2";
	import Minimize2 from "@lucide/svelte/icons/minimize-2";
	import SignalHigh from "@lucide/svelte/icons/signal-high";
	import SignalMedium from "@lucide/svelte/icons/signal-medium";
	import SignalLow from "@lucide/svelte/icons/signal-low";
	import { call, SELF_KEY, type ScreenShareOpts } from "$lib/webrtc/call.svelte";
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
	import type { Channel, ServerEntry } from "$lib/data/mock";

	let { server, channel, onJoin }: {
		server?: ServerEntry;
		channel: Channel;
		onJoin: () => void;
	} = $props();

	const joined = $derived(call.roomId === channel.id);
	let fullscreen = $state(false);
	let sharePickerOpen = $state(false);

	function onScreenShareClick() {
		if (call.screenSharing) call.toggleScreenShare();
		else sharePickerOpen = true;
	}

	function goLive(opts: ScreenShareOpts) {
		sharePickerOpen = false;
		call.toggleScreenShare(opts);
	}

	function qualityIcon(userId: string) {
		const level = call.connectionQuality[userId];
		if (level === "poor") return SignalLow;
		if (level === "medium") return SignalMedium;
		return SignalHigh;
	}

	$effect(() => {
		const token = session.token;
		if (!token) return;
		profileStore.load(token, session.username ?? "");
		if (!server && channel.name) profileStore.load(token, channel.name);
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

	const hasSpotlight = $derived(
		call.screenSharing || sharingUserIds.size > 0 || call.cameraEnabled || camUserIds.size > 0
	);

	type QuietEntry = { key: string; name: string; avatarUrl?: string; speaking: boolean; isSelf: boolean };

	const quietParticipants = $derived.by(() => {
		const entries: QuietEntry[] = [];
		if (!call.cameraEnabled) {
			entries.push({
				key: "self",
				name: ownProfile?.display_name || session.username || "",
				avatarUrl: ownProfile?.avatar_url ? api.resolveUrl(ownProfile.avatar_url, session.token) : undefined,
				speaking: call.selfSpeaking && !call.muted,
				isSelf: true
			});
		}
		for (const participant of call.participants) {
			if (camUserIds.has(participant.userId)) continue;
			const remoteProfile = profileStore.forUser(participant.username);
			entries.push({
				key: participant.userId,
				name: remoteProfile?.display_name || participant.username,
				avatarUrl: remoteProfile?.avatar_url ? api.resolveUrl(remoteProfile.avatar_url, session.token) : undefined,
				speaking: call.speakingUserIds.has(participant.userId),
				isSelf: false
			});
		}
		return entries;
	});

	const gridCols = $derived.by(() => {
		const n = quietParticipants.length;
		return n <= 1 ? 1 : n <= 4 ? 2 : n <= 9 ? 3 : 4;
	});

	const gridRows = $derived(Math.max(1, Math.ceil(quietParticipants.length / gridCols)));

	function initials(name: string) {
		return name.slice(0, 2).toUpperCase();
	}

	// Outgoing DM call, waiting for the other side: show my avatar + the
	// callee's dimmed avatar with Discord-style ripple rings.
	const ringing = $derived(joined && !server && !hasSpotlight && call.participants.length === 0);
	const calleeProfile = $derived(!server ? profileStore.forUser(channel.name) : null);
	const selfAvatarUrl = $derived(
		ownProfile?.avatar_url ? api.resolveUrl(ownProfile.avatar_url, session.token) : undefined
	);
	const calleeAvatarUrl = $derived(
		calleeProfile?.avatar_url ? api.resolveUrl(calleeProfile.avatar_url, session.token) : undefined
	);
</script>

{#if !joined}
	<div class="prejoin" transition:fade={{ duration: 140 }}>
		<div class="prejoin-icon"><Volume2 size={40} strokeWidth={1.5} /></div>
		<h2>{channel.name}</h2>
		<p>{server ? t("call.noneConnected") : t("call.notConnected")}</p>
		<button class="join-btn" onclick={onJoin}>{server ? t("call.joinVoice") : t("call.join")}</button>
	</div>
{:else}
	{@const HeaderQualityIcon = qualityIcon(SELF_KEY)}
	<div class="stage" class:fullscreen transition:fade={{ duration: 140 }}>
		<div class="stage-header">
			<Volume2 size={16} strokeWidth={2.25} />
			<span>{server ? `${server.name} / ${channel.name}` : channel.name}</span>
			<span class="quality-self" class:good={call.connectionQuality[SELF_KEY] !== "poor" && call.connectionQuality[SELF_KEY] !== "medium"} class:medium={call.connectionQuality[SELF_KEY] === "medium"} class:poor={call.connectionQuality[SELF_KEY] === "poor"}>
				<HeaderQualityIcon size={14} strokeWidth={2.25} />
			</span>
			<div class="header-spacer"></div>
			<button class="fullscreen-btn" title={fullscreen ? t("call.exitFullscreen") : t("call.fullscreen")} onclick={() => (fullscreen = !fullscreen)}>
				{#if fullscreen}<Minimize2 size={15} strokeWidth={2} />{:else}<Maximize2 size={15} strokeWidth={2} />{/if}
			</button>
		</div>

		{#if hasSpotlight}
			<div class="spotlight-row">
				{#if call.screenSharing}
					<div class="spotlight-tile screen">
						<video use:attachLocalScreenStream autoplay playsinline muted></video>
						<span class="tile-name">{t("call.yourScreen")}</span>
					</div>
				{/if}
				{#each call.participants as participant (participant.userId)}
					{#if sharingUserIds.has(participant.userId)}
						<div class="spotlight-tile screen">
							<video use:attachRemoteScreenStream={participant.userId} autoplay playsinline></video>
							<span class="tile-name">{participant.username}'s screen</span>
						</div>
					{/if}
				{/each}
				{#if call.cameraEnabled}
					{@const SelfIcon = qualityIcon(SELF_KEY)}
					<div class="spotlight-tile" class:speaking={call.selfSpeaking && !call.muted}>
						<video use:attachLocalStream autoplay playsinline muted></video>
						<span class="tile-name">
							{#if call.muted}<MicOff size={12} strokeWidth={2.25} />{:else}<Mic size={12} strokeWidth={2.25} />{/if}
							{ownProfile?.display_name || session.username} (you)
							<SelfIcon size={12} strokeWidth={2.25} class="quality-{call.connectionQuality[SELF_KEY] ?? 'good'}" />
						</span>
					</div>
				{/if}
				{#each call.participants as participant (participant.userId)}
					{#if camUserIds.has(participant.userId)}
						{@const remoteProfile = profileStore.forUser(participant.username)}
						{@const PeerIcon = qualityIcon(participant.userId)}
						<div class="spotlight-tile" class:speaking={call.speakingUserIds.has(participant.userId)}>
							<video use:attachRemoteStream={participant.userId} autoplay playsinline muted></video>
							<span class="tile-name">
								{remoteProfile?.display_name || participant.username}
								<PeerIcon size={12} strokeWidth={2.25} class="quality-{call.connectionQuality[participant.userId] ?? 'good'}" />
							</span>
						</div>
					{/if}
				{/each}
			</div>
		{/if}

		{#if ringing}
			<div class="ringing">
				<div
					class="ring-avatar"
					style:background-image={selfAvatarUrl ? `url(${selfAvatarUrl})` : undefined}
				>
					{#if !selfAvatarUrl}<span>{initials(ownProfile?.display_name || session.username || "")}</span>{/if}
				</div>
				<div class="ring-avatar callee">
					<span class="wave"></span>
					<span class="wave"></span>
					<div
						class="ring-face"
						style:background-image={calleeAvatarUrl ? `url(${calleeAvatarUrl})` : undefined}
					>
						{#if !calleeAvatarUrl}<span>{initials(calleeProfile?.display_name || channel.name)}</span>{/if}
					</div>
					<span class="ring-name">{calleeProfile?.display_name || channel.name}</span>
				</div>
			</div>
		{:else if quietParticipants.length > 0}
			<div
				class="grid"
				style:grid-template-columns={`repeat(${gridCols}, 1fr)`}
				style:grid-template-rows={`repeat(${gridRows}, 1fr)`}
			>
				{#each quietParticipants as entry (entry.key)}
					{@const qualityKey = entry.isSelf ? SELF_KEY : entry.key}
					{@const CellQualityIcon = qualityIcon(qualityKey)}
					<div class="cell" class:speaking={entry.speaking}>
						<div
							class="cell-avatar"
							style:background={entry.avatarUrl ? undefined : "var(--accent-fill)"}
							style:background-image={entry.avatarUrl ? `url(${entry.avatarUrl})` : undefined}
						>
							{#if !entry.avatarUrl}<span>{initials(entry.name)}</span>{/if}
						</div>
						<div class="cell-tag">
							{#if entry.isSelf}
								{#if call.muted}<MicOff size={13} strokeWidth={2.25} />{:else}<Mic size={13} strokeWidth={2.25} />{/if}
							{/if}
							<span>{entry.name}{entry.isSelf ? " (you)" : ""}</span>
							<CellQualityIcon size={12} strokeWidth={2.25} class="quality-{call.connectionQuality[qualityKey] ?? 'good'}" />
						</div>
					</div>
				{/each}
			</div>
		{/if}

		<div class="controls">
			<button class="ctrl" class:active-danger={call.muted} aria-label={call.muted ? t("call.unmute") : t("call.mute")} onclick={() => call.toggleMute()}>
				{#if call.muted}<MicOff size={18} strokeWidth={2} />{:else}<Mic size={18} strokeWidth={2} />{/if}
			</button>
			<button class="ctrl" class:active-danger={call.deafened} aria-label={call.deafened ? t("call.undeafen") : t("call.deafen")} onclick={() => call.toggleDeafen()}>
				{#if call.deafened}<HeadphoneOff size={18} strokeWidth={2} />{:else}<Headphones size={18} strokeWidth={2} />{/if}
			</button>
			<button class="ctrl" class:active={call.cameraEnabled} aria-label={call.cameraEnabled ? t("call.cameraOff") : t("call.cameraOn")} onclick={() => call.toggleCamera()}>
				{#if call.cameraEnabled}<Video size={18} strokeWidth={2} />{:else}<VideoOff size={18} strokeWidth={2} />{/if}
			</button>
			<button class="ctrl" class:active={call.screenSharing} aria-label={call.screenSharing ? t("call.stopSharing") : t("call.shareScreen")} onclick={onScreenShareClick}>
				{#if call.screenSharing}<ScreenShareOff size={18} strokeWidth={2} />{:else}<ScreenShare size={18} strokeWidth={2} />{/if}
			</button>
			<button class="ctrl leave" aria-label={t("call.disconnect")} onclick={() => call.leave()}>
				<PhoneOff size={18} strokeWidth={2} />
			</button>
		</div>

		{#each call.participants as participant (participant.userId)}
			<audio use:attachRemoteStream={participant.userId} autoplay muted={call.deafened}></audio>
		{/each}
	</div>
{/if}

{#if sharePickerOpen}
	<ScreenSharePicker onCancel={() => (sharePickerOpen = false)} onGoLive={goLive} />
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

	.header-spacer {
		flex: 1;
	}

	.fullscreen-btn {
		display: flex;
		align-items: center;
		justify-content: center;
		width: 26px;
		height: 26px;
		border-radius: 6px;
		color: var(--ink-dim);
		transition: background-color 0.15s ease, color 0.15s ease;
	}

	.fullscreen-btn:hover {
		background: var(--hover);
		color: var(--ink);
	}

	.quality-self {
		display: flex;
		align-items: center;
	}

	.quality-self.good {
		color: var(--online);
	}

	.quality-self.medium {
		color: var(--idle);
	}

	.quality-self.poor {
		color: var(--danger);
	}

	.stage.fullscreen {
		position: fixed;
		inset: 0;
		z-index: 1000;
		border-radius: 0;
	}

	:global(.quality-good) {
		color: var(--online);
	}

	:global(.quality-medium) {
		color: var(--idle);
	}

	:global(.quality-poor) {
		color: var(--danger);
	}

	.spotlight-row {
		flex-shrink: 0;
		display: flex;
		gap: 10px;
		padding: 16px 20px 0;
		flex-wrap: wrap;
		justify-content: center;
		height: min(46vh, 420px);
		overflow: hidden;
	}

	.spotlight-tile {
		position: relative;
		flex: 0 1 auto;
		height: 100%;
		max-width: 100%;
		aspect-ratio: 16 / 9;
		border-radius: 10px;
		overflow: hidden;
		background: black;
		box-shadow: 0 0 0 2px transparent;
		transition: box-shadow 0.12s ease;
	}

	.spotlight-tile.speaking {
		box-shadow: 0 0 0 2px var(--online);
	}

	.spotlight-tile video {
		width: 100%;
		height: 100%;
		object-fit: cover;
	}

	.spotlight-tile.screen {
		background: black;
	}

	.spotlight-tile.screen video {
		object-fit: contain;
	}

	.spotlight-tile .tile-name {
		position: absolute;
		left: 10px;
		bottom: 8px;
		display: flex;
		align-items: center;
		gap: 5px;
		padding: 3px 8px;
		border-radius: 5px;
		background: rgba(0, 0, 0, 0.55);
		font-size: 12px;
		font-weight: 600;
		color: white;
	}

	.grid {
		flex: 1;
		min-height: 0;
		display: grid;
		gap: 14px;
		padding: 24px;
		overflow: hidden;
		justify-content: center;
		align-content: center;
	}

	.ringing {
		flex: 1;
		display: flex;
		align-items: center;
		justify-content: center;
		gap: 56px;
		padding: 24px;
	}

	.ring-avatar {
		position: relative;
		width: 92px;
		aspect-ratio: 1;
		border-radius: 50%;
		background-size: cover;
		background-position: center;
		background-color: var(--accent-fill);
		display: flex;
		align-items: center;
		justify-content: center;
		font-family: var(--font-mono);
		font-weight: 700;
		font-size: 20px;
		color: var(--accent-fill-ink);
		flex-shrink: 0;
	}

	.ring-avatar.callee {
		background: none;
	}

	.ring-face {
		width: 100%;
		height: 100%;
		border-radius: 50%;
		background-size: cover;
		background-position: center;
		background-color: var(--sidebar);
		display: flex;
		align-items: center;
		justify-content: center;
		filter: grayscale(0.7) brightness(0.8);
	}

	.wave {
		position: absolute;
		inset: -5px;
		border-radius: 50%;
		border: 2px solid rgba(255, 255, 255, 0.55);
		opacity: 0;
		will-change: transform, opacity;
		animation: call-wave 2s ease-out infinite;
	}

	/* second ring is just the same ripple offset by half a cycle so one
	   fades out as the next leaves the centre */
	.wave:nth-child(2) {
		animation-delay: 1s;
	}

	/* one ripple: leave the centre, grow, fade out — then the next starts */
	@keyframes call-wave {
		0% {
			transform: scale(1);
			opacity: 0.6;
		}
		80% {
			opacity: 0.12;
		}
		100% {
			transform: scale(2.4);
			opacity: 0;
		}
	}

	/* the "calling…" pulse is functional feedback, keep it even with
	   reduce-motion (the global rule in app.css otherwise kills it) */
	@media (prefers-reduced-motion: reduce) {
		.wave {
			animation-duration: 2s !important;
			animation-iteration-count: infinite !important;
		}
	}

	.ring-name {
		position: absolute;
		top: calc(100% + 10px);
		left: 50%;
		transform: translateX(-50%);
		white-space: nowrap;
		font-size: 12px;
		font-weight: 600;
		color: var(--ink-dim);
	}

	.cell {
		position: relative;
		min-height: 0;
		width: 100%;
		max-width: 360px;
		max-height: 100%;
		aspect-ratio: 4 / 3;
		border-radius: 14px;
		background:
			radial-gradient(120% 120% at 50% 0%, color-mix(in srgb, var(--accent-fill) 12%, var(--sidebar)), var(--sidebar));
		display: flex;
		align-items: center;
		justify-content: center;
		overflow: hidden;
		box-shadow: inset 0 0 0 1px var(--hairline);
		transition: box-shadow 0.15s ease;
	}

	.cell.speaking {
		box-shadow: inset 0 0 0 2px var(--online);
	}

	.cell-avatar {
		position: relative;
		width: 42%;
		aspect-ratio: 1;
		max-width: 128px;
		min-width: 56px;
		border-radius: 50%;
		background-size: cover;
		background-position: center;
		display: flex;
		align-items: center;
		justify-content: center;
		font-family: var(--font-mono);
		font-weight: 700;
		font-size: 24px;
		color: var(--accent-fill-ink);
		background-color: var(--accent-fill);
	}

	.cell.speaking .cell-avatar::after {
		content: "";
		position: absolute;
		inset: -6px;
		border-radius: 50%;
		border: 3px solid var(--online);
		animation: speak-pulse 1.4s ease-out infinite;
	}

	@keyframes speak-pulse {
		0% {
			transform: scale(0.96);
			opacity: 0.9;
		}
		70% {
			transform: scale(1.12);
			opacity: 0;
		}
		100% {
			opacity: 0;
		}
	}

	.cell-tag {
		position: absolute;
		left: 50%;
		bottom: 12px;
		transform: translateX(-50%);
		display: flex;
		align-items: center;
		gap: 6px;
		padding: 4px 10px;
		border-radius: 999px;
		background: rgba(0, 0, 0, 0.6);
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
		margin: 0 auto 18px;
		padding: 10px 14px;
		border-radius: 999px;
		background: color-mix(in srgb, var(--sidebar) 92%, black);
		box-shadow: 0 8px 24px rgba(0, 0, 0, 0.35);
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
		transition: background-color 0.15s ease, color 0.15s ease, transform 0.1s ease;
	}

	.ctrl:hover {
		background: var(--hover);
		transform: translateY(-1px);
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

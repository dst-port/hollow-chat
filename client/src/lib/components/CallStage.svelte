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
	import { call, SELF_KEY, shareErrorKey, type ScreenShareOpts } from "$lib/webrtc/call.svelte";
	import ScreenSharePicker from "$lib/components/ScreenSharePicker.svelte";
	import { toast } from "$lib/stores/toast.svelte";
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
		if (call.screenSharing) {
			call.toggleScreenShare().catch((err) => toast.push(t(shareErrorKey(err, "toast.screenShareFailed"))));
		} else {
			sharePickerOpen = true;
		}
	}

	function goLive(opts: ScreenShareOpts) {
		sharePickerOpen = false;
		call.toggleScreenShare(opts).catch((err) => toast.push(t(shareErrorKey(err, "toast.screenShareFailed"))));
	}

	function onCameraClick() {
		call.toggleCamera().catch((err) => toast.push(t(shareErrorKey(err, "toast.cameraFailed"))));
	}

	// Blow one screen-share tile up to real OS fullscreen. Falls back to the
	// stage-level fullscreen toggle if the Fullscreen API is unavailable.
	function expandTile(event: MouseEvent) {
		const tile = (event.currentTarget as HTMLElement).closest(".tile");
		if (tile && "requestFullscreen" in tile) {
			(tile as HTMLElement).requestFullscreen().catch(() => (fullscreen = true));
		} else {
			fullscreen = true;
		}
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
		{:else}
			{@const SelfIcon = qualityIcon(SELF_KEY)}
			<!-- One uniform gallery: screen shares and people are all equal
			     tiles that wrap. A lone tile grows to fill; more tiles shrink
			     and wrap. Expand a screen tile for real fullscreen. -->
			<div class="tiles">
				{#if call.screenSharing}
					<div class="tile screen">
						<video use:attachLocalScreenStream autoplay playsinline muted></video>
						<span class="badge-live">LIVE</span>
						<button
							class="tile-expand"
							title={t("call.fullscreen")}
							aria-label={t("call.fullscreen")}
							onclick={expandTile}
						>
							<Maximize2 size={14} strokeWidth={2} />
						</button>
						<span class="tile-name">
							<ScreenShare size={12} strokeWidth={2.25} />
							{ownProfile?.display_name || session.username}
						</span>
					</div>
				{/if}
				{#each call.participants as participant (participant.userId)}
					{#if sharingUserIds.has(participant.userId)}
						<div class="tile screen">
							<!-- muted: an unmuted autoplay <video> is blocked by the
							     autoplay policy and just renders black. Screen-share
							     audio rides on the separate <audio> below. -->
							<video use:attachRemoteScreenStream={participant.userId} autoplay playsinline muted></video>
							<audio use:attachRemoteScreenStream={participant.userId} autoplay muted={call.deafened}></audio>
							<button
								class="tile-expand"
								title={t("call.fullscreen")}
								aria-label={t("call.fullscreen")}
								onclick={expandTile}
							>
								<Maximize2 size={14} strokeWidth={2} />
							</button>
							<span class="tile-name">
								<ScreenShare size={12} strokeWidth={2.25} />
								{participant.username}
							</span>
						</div>
					{/if}
				{/each}

				<div class="tile" class:speaking={call.selfSpeaking && !call.muted}>
					{#if call.cameraEnabled}
						<video use:attachLocalStream autoplay playsinline muted></video>
					{:else}
						<div
							class="tile-avatar"
							style:background={selfAvatarUrl ? undefined : "var(--accent-fill)"}
							style:background-image={selfAvatarUrl ? `url(${selfAvatarUrl})` : undefined}
						>
							{#if !selfAvatarUrl}<span>{initials(ownProfile?.display_name || session.username || "")}</span>{/if}
						</div>
					{/if}
					<span class="tile-name">
						{#if call.muted}<MicOff size={12} strokeWidth={2.25} />{:else}<Mic size={12} strokeWidth={2.25} />{/if}
						{ownProfile?.display_name || session.username} (you)
						<SelfIcon size={12} strokeWidth={2.25} class="quality-{call.connectionQuality[SELF_KEY] ?? 'good'}" />
					</span>
				</div>
				{#each call.participants as participant (participant.userId)}
					{@const remoteProfile = profileStore.forUser(participant.username)}
					{@const PeerIcon = qualityIcon(participant.userId)}
					{@const avatarUrl = remoteProfile?.avatar_url
						? api.resolveUrl(remoteProfile.avatar_url, session.token)
						: undefined}
					<div class="tile" class:speaking={call.speakingUserIds.has(participant.userId)}>
						{#if camUserIds.has(participant.userId)}
							<video use:attachRemoteStream={participant.userId} autoplay playsinline muted></video>
						{:else}
							<div
								class="tile-avatar"
								style:background={avatarUrl ? undefined : "var(--accent-fill)"}
								style:background-image={avatarUrl ? `url(${avatarUrl})` : undefined}
							>
								{#if !avatarUrl}<span>{initials(remoteProfile?.display_name || participant.username)}</span>{/if}
							</div>
						{/if}
						<span class="tile-name">
							{remoteProfile?.display_name || participant.username}
							<PeerIcon size={12} strokeWidth={2.25} class="quality-{call.connectionQuality[participant.userId] ?? 'good'}" />
						</span>
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
			<button class="ctrl" class:active={call.cameraEnabled} aria-label={call.cameraEnabled ? t("call.cameraOff") : t("call.cameraOn")} onclick={onCameraClick}>
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

	/* One uniform gallery. Explicit viewport-relative height (not flex-grow:
	   the stage's ancestor chain doesn't reliably hand down a definite
	   height, so a flex row would collapse). Tiles wrap and centre. */
	.tiles {
		flex: 0 0 auto;
		height: clamp(240px, 64vh, 760px);
		display: flex;
		flex-wrap: wrap;
		gap: 12px;
		padding: 16px 20px;
		align-content: center;
		align-items: center;
		justify-content: center;
		overflow-y: auto;
	}

	.stage.fullscreen .tiles {
		height: clamp(240px, 82vh, 1400px);
	}

	.tile {
		position: relative;
		/* Lone tile grows toward the cap; more tiles shrink past the basis
		   and wrap. aspect-ratio keeps every card the same shape. */
		flex: 1 1 340px;
		max-width: min(900px, 100%);
		max-height: 100%;
		aspect-ratio: 16 / 10;
		border-radius: 14px;
		overflow: hidden;
		display: flex;
		align-items: center;
		justify-content: center;
		background: radial-gradient(
			120% 120% at 50% 0%,
			color-mix(in srgb, var(--accent-fill) 12%, var(--sidebar)),
			var(--sidebar)
		);
		box-shadow: inset 0 0 0 1px var(--hairline);
		transition: box-shadow 0.15s ease;
	}

	.tile.speaking {
		box-shadow: inset 0 0 0 2px var(--online);
	}

	.tile video {
		width: 100%;
		height: 100%;
		object-fit: cover;
	}

	.tile.screen {
		background: black;
	}

	.tile.screen video {
		object-fit: contain;
	}

	.tile.screen:fullscreen {
		max-width: none;
		max-height: none;
		aspect-ratio: auto;
		width: 100vw;
		height: 100vh;
		border-radius: 0;
		background: black;
	}

	.tile-avatar {
		width: 40%;
		aspect-ratio: 1;
		max-width: 132px;
		min-width: 52px;
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

	.badge-live {
		position: absolute;
		top: 8px;
		right: 8px;
		padding: 2px 7px;
		border-radius: 5px;
		background: var(--danger);
		color: white;
		font-size: 10px;
		font-weight: 800;
		letter-spacing: 0.06em;
	}

	.tile-expand {
		position: absolute;
		top: 8px;
		left: 8px;
		display: flex;
		align-items: center;
		justify-content: center;
		width: 28px;
		height: 28px;
		border-radius: 6px;
		background: rgba(0, 0, 0, 0.55);
		color: white;
		opacity: 0;
		transition: opacity 0.12s ease, background 0.12s ease;
	}

	.tile.screen:hover .tile-expand,
	.tile-expand:focus-visible {
		opacity: 1;
	}

	.tile-expand:hover {
		background: rgba(0, 0, 0, 0.8);
	}

	.tile .tile-name {
		position: absolute;
		left: 10px;
		bottom: 8px;
		display: flex;
		align-items: center;
		gap: 5px;
		max-width: calc(100% - 20px);
		padding: 3px 8px;
		border-radius: 6px;
		background: rgba(0, 0, 0, 0.55);
		font-size: 12px;
		font-weight: 600;
		color: white;
		white-space: nowrap;
		overflow: hidden;
		text-overflow: ellipsis;
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
		width: 80px;
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
			opacity: 0.1;
		}
		100% {
			transform: scale(1.7);
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

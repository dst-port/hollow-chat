import { WS_BASE_URL, fetchIceServers } from "$lib/api/client";
import { startCallRing, stopCallRing } from "$lib/utils/sound";
import {
	nativeOutputRoutingAvailable,
	listNativeAudioSinks,
	setNativeAppAudioSink,
	getNativeDefaultSink,
	type AudioSink
} from "$lib/utils/audioOutput";

type Participant = { userId: string; username: string };

/** Machine-readable reason a call couldn't get the mic, so the UI can show
 *  an accurate toast instead of always blaming permissions. */
export type MicErrorCode = "denied" | "notfound" | "busy" | "unavailable";

export class MicError extends Error {
	code: MicErrorCode;
	constructor(code: MicErrorCode, message: string) {
		super(message);
		this.code = code;
	}
}

/** getUserMedia for the mic, with one retry that drops a stale saved
 *  `deviceId` - the #1 cause of "calls suddenly stopped working" is a
 *  previously-picked mic that got unplugged/renamed, which makes
 *  `{ deviceId: { exact } }` throw OverconstrainedError forever. */
async function acquireMic(
	noiseSuppression: boolean,
	inputDeviceId: string | null
): Promise<MediaStream> {
	if (!navigator.mediaDevices?.getUserMedia) {
		throw new MicError("unavailable", "Microphone API unavailable in this context");
	}
	const base: MediaTrackConstraints = { noiseSuppression };
	try {
		const constraints = inputDeviceId
			? { ...base, deviceId: { exact: inputDeviceId } }
			: base;
		return await navigator.mediaDevices.getUserMedia({ audio: constraints });
	} catch (err) {
		const name = err instanceof DOMException ? err.name : "";
		if (inputDeviceId && (name === "OverconstrainedError" || name === "NotFoundError")) {
			return await navigator.mediaDevices.getUserMedia({ audio: base });
		}
		throw err;
	}
}

/** i18n key for a failed `call.join`: a specific one for the cases the user
 *  can actually act on, else the caller's generic fallback. */
export function micErrorKey(err: unknown, fallbackKey: string): string {
	if (err instanceof MicError) {
		if (err.code === "busy") return "toast.callMicBusy";
		if (err.code === "notfound") return "toast.callMicNotFound";
		if (err.code === "unavailable") return "toast.callMicUnavailable";
	}
	return fallbackKey;
}

function asMicError(err: unknown): MicError {
	if (err instanceof MicError) return err;
	const name = err instanceof DOMException ? err.name : "";
	switch (name) {
		case "NotAllowedError":
		case "SecurityError":
			return new MicError("denied", "Microphone permission is blocked");
		case "NotFoundError":
		case "OverconstrainedError":
			return new MicError("notfound", "No microphone found");
		case "NotReadableError":
		case "AbortError":
			return new MicError("busy", "Microphone is in use by another app");
		default:
			return new MicError("unavailable", "Could not access the microphone");
	}
}

export type ScreenShareOpts = {
	width?: number;
	height?: number;
	frameRate?: number;
	contentHint?: "motion" | "detail";
	audio?: boolean;
	/** Which surface kind our picker pre-selected; maps to a displaySurface
	 *  hint so the browser's chooser opens on the right tab. */
	surface?: "tab" | "window" | "screen";
};

type ServerMsg =
	| { type: "room-state"; members: { user_id: string; username: string }[] }
	| { type: "peer-joined"; user_id: string; username: string }
	| { type: "peer-left"; user_id: string }
	| { type: "offer"; from: string; from_username: string; sdp: string }
	| { type: "answer"; from: string; sdp: string }
	| { type: "ice-candidate"; from: string; candidate: string }
	| { type: "track-meta"; from: string; mid: string; kind: string };

type ClientMsg =
	| { type: "offer"; to: string; sdp: string }
	| { type: "answer"; to: string; sdp: string }
	| { type: "ice-candidate"; to: string; candidate: string }
	| { type: "track-meta"; to: string; mid: string; kind: string };

const FALLBACK_ICE_SERVERS: RTCIceServer[] = [{ urls: ["stun:stun.l.google.com:19302"] }];

export const SELF_KEY = "__self__";
const SPEAKING_THRESHOLD = 7;
const SPEAKING_HOLD_MS = 250;
const SPEAKING_POLL_MS = 70;
const STATS_POLL_MS = 2500;

const SETTINGS_KEY = "hollowchat.voice-settings";

type VoiceSettings = {
	inputDeviceId: string | null;
	outputDeviceId: string | null;
	outputVolume: number;
	noiseSuppression: boolean;
	pushToTalk: boolean;
	pushToTalkKey: string;
};

function defaultVoiceSettings(): VoiceSettings {
	return {
		inputDeviceId: null,
		outputDeviceId: null,
		outputVolume: 1,
		noiseSuppression: true,
		pushToTalk: false,
		pushToTalkKey: "Space"
	};
}

function loadVoiceSettings(): VoiceSettings {
	try {
		const raw = localStorage.getItem(SETTINGS_KEY);
		if (!raw) return defaultVoiceSettings();
		return { ...defaultVoiceSettings(), ...JSON.parse(raw) };
	} catch {
		return defaultVoiceSettings();
	}
}

/// Maps a real getDisplayMedia/getUserMedia failure to a toast i18n key,
/// falling back to the caller's context key ("toast.screenShareFailed" or
/// "toast.cameraFailed") for anything not worth its own line. Mirrors
/// micErrorKey. Cancelled-picker errors never reach here — toggleScreenShare /
/// toggleCamera swallow those before they'd rethrow.
export function shareErrorKey(err: unknown, fallbackKey: string): string {
	if (err instanceof DOMException) {
		switch (err.name) {
			// Device/display exists but the OS or another app won't yield it,
			// or the requested constraints can't be met — the generic
			// "couldn't start" line covers all of these well enough.
			case "NotReadableError":
			case "NotFoundError":
			case "OverconstrainedError":
				return fallbackKey;
		}
	}
	return fallbackKey;
}

class CallStore {
	roomId = $state<string | null>(null);
	label = $state("");
	status = $state<"idle" | "connecting" | "connected">("idle");
	muted = $state(false);
	deafened = $state(false);
	cameraEnabled = $state(false);
	screenSharing = $state(false);
	participants = $state<Participant[]>([]);
	streamsVersion = $state(0);
	selfSpeaking = $state(false);
	speakingUserIds = $state<Set<string>>(new Set());
	connectionQuality = $state<Record<string, "good" | "medium" | "poor">>({});

	inputDevices = $state<MediaDeviceInfo[]>([]);
	outputDevices = $state<MediaDeviceInfo[]>([]);
	nativeSinks = $state<AudioSink[]>([]);
	nativeOutputRouting = $state(false);
	private settings = loadVoiceSettings();
	inputDeviceId = $state(this.settings.inputDeviceId);
	outputDeviceId = $state(this.settings.outputDeviceId);
	outputVolume = $state(this.settings.outputVolume);
	noiseSuppression = $state(this.settings.noiseSuppression);
	pushToTalk = $state(this.settings.pushToTalk);
	pushToTalkKey = $state(this.settings.pushToTalkKey);
	pushToTalkActive = $state(false);

	private mutedBeforeDeafen = false;
	private audioCtx: AudioContext | null = null;
	private analysers = new Map<string, { analyser: AnalyserNode; data: Uint8Array }>();
	private lastSpokeAt = new Map<string, number>();
	private speakingInterval: ReturnType<typeof setInterval> | null = null;
	private statsInterval: ReturnType<typeof setInterval> | null = null;
	private ws: WebSocket | null = null;
	private pcs = new Map<string, RTCPeerConnection>();
	private remoteStreams = new Map<string, MediaStream>();
	private remoteScreenStreams = new Map<string, MediaStream>();
	private screenMids = new Set<string>();
	private pendingCandidates = new Map<string, RTCIceCandidateInit[]>();
	// Perfect-negotiation state per peer. `polite` decides who yields on glare:
	// the side that made the PC from an incoming offer is polite, the existing
	// member that offered into a "peer-joined" is impolite.
	private negotiation = new Map<string, { polite: boolean; makingOffer: boolean; ignoreOffer: boolean }>();
	private localStream: MediaStream | null = null;
	private localScreenStream: MediaStream | null = null;
	private iceServers: RTCIceServer[] = FALLBACK_ICE_SERVERS;
	private listeners = new Set<() => void>();

	// Auto-leave an empty call after this long alone, to stop holding a
	// mic + socket (and, with screen share, an encoder) for nobody.
	private static readonly ALONE_TIMEOUT_MS = 120_000;
	private aloneTimer: ReturnType<typeof setTimeout> | null = null;

	// Guards a hung WebSocket in join() (stuck CONNECTING) - fires teardown +
	// throw so the mic doesn't stay live forever.
	private static readonly CONNECT_TIMEOUT_MS = 10_000;
	private connectTimer: ReturnType<typeof setTimeout> | null = null;

	// Bumped on every join(); a superseded join bails after its awaits.
	private joinGeneration = 0;

	// A "failed" connectionState is often just a wifi blip - try one ICE
	// restart and give it a grace window before dropping the peer for good.
	private static readonly ICE_RESTART_GRACE_MS = 8_000;
	private iceRestartTimers = new Map<string, ReturnType<typeof setTimeout>>();
	private iceRestartAttempted = new Set<string>();

	// Call-event bookkeeping so a chat "started a call" line can be posted
	// once, by whoever opened the room, when it ends.
	private joinedAt = 0;
	/** true while the current room is a DM call, not a server voice channel. */
	isDmCall = $state(false);
	createdRoom = $state(false);
	/** Guards the "started a call" chat line to one post per call, even
	 *  across multiple ChatView instances. */
	announcedStart = $state(false);
	/** id of the "started a call" message we posted, so we can edit in the
	 *  duration when the call ends. */
	announcedMessageId = $state<string | null>(null);
	/** Set on teardown for the opener: which message to edit and with what
	 *  duration. ChatView consumes and clears it. */
	callEndEdit = $state<{ roomId: string; messageId: string; durationSec: number } | null>(null);

	onStreamsChanged(callback: () => void): () => void {
		this.listeners.add(callback);
		return () => this.listeners.delete(callback);
	}

	private notify() {
		this.streamsVersion++;
		for (const cb of this.listeners) cb();
	}

	getRemoteStream(userId: string): MediaStream | null {
		return this.remoteStreams.get(userId) ?? null;
	}

	getRemoteScreenStream(userId: string): MediaStream | null {
		return this.remoteScreenStreams.get(userId) ?? null;
	}

	getLocalStream(): MediaStream | null {
		return this.localStream;
	}

	getLocalScreenStream(): MediaStream | null {
		return this.localScreenStream;
	}

	private persistSettings() {
		const settings: VoiceSettings = {
			inputDeviceId: this.inputDeviceId,
			outputDeviceId: this.outputDeviceId,
			outputVolume: this.outputVolume,
			noiseSuppression: this.noiseSuppression,
			pushToTalk: this.pushToTalk,
			pushToTalkKey: this.pushToTalkKey
		};
		try {
			localStorage.setItem(SETTINGS_KEY, JSON.stringify(settings));
		} catch {
			// storage unavailable, setting just won't survive a reload
		}
	}

	/// Device labels only come through once mic permission has been granted
	/// at least once in this session - if there's no active call yet (no
	/// permission grant behind us), request a throwaway stream just to
	/// unlock labels, then immediately let it go. Otherwise the picker in
	/// Settings would list real devices with blank/generic names before
	/// you'd ever joined a voice channel.
	async refreshDevices(): Promise<void> {
		let throwaway: MediaStream | null = null;
		if (!this.localStream) {
			try {
				throwaway = await navigator.mediaDevices.getUserMedia({ audio: true });
			} catch {
				// permission denied/unavailable - enumeration below will just come back unlabeled
			}
		}

		try {
			const devices = await navigator.mediaDevices.enumerateDevices();
			this.inputDevices = devices.filter((d) => d.kind === "audioinput");
			this.outputDevices = devices.filter((d) => d.kind === "audiooutput");
		} catch {
			// enumeration unsupported/blocked - quick settings just won't offer a picker
		} finally {
			throwaway?.getTracks().forEach((t) => t.stop());
		}

		this.nativeOutputRouting = await nativeOutputRoutingAvailable();
		if (this.nativeOutputRouting) {
			this.nativeSinks = await listNativeAudioSinks();
		}
	}

	get noiseSuppressionSupported(): boolean {
		try {
			return !!navigator.mediaDevices.getSupportedConstraints().noiseSuppression;
		} catch {
			return false;
		}
	}

	noiseSuppressionActive = $state(false);

	/// setSinkId (routing audio output to a chosen device) is a Chromium
	/// feature - WebKitGTK, our Linux desktop runtime, has never
	/// implemented it. There we route through PipeWire/PulseAudio directly
	/// instead (see audio_output.rs + utils/audioOutput.ts), which is the
	/// real, non-fake fix rather than just admitting defeat.
	get outputDeviceSelectionSupported(): boolean {
		return this.nativeOutputRouting || typeof HTMLMediaElement.prototype.setSinkId === "function";
	}

	async setInputDevice(deviceId: string | null): Promise<void> {
		this.inputDeviceId = deviceId;
		this.persistSettings();
		if (!this.localStream || this.status === "idle") return;

		try {
			const constraints: MediaTrackConstraints = { noiseSuppression: this.noiseSuppression };
			if (deviceId) constraints.deviceId = { exact: deviceId };
			const newStream = await navigator.mediaDevices.getUserMedia({ audio: constraints });
			const newTrack = newStream.getAudioTracks()[0];
			if (!newTrack) return;

			const oldTrack = this.localStream.getAudioTracks()[0];
			for (const pc of this.pcs.values()) {
				const sender = pc.getSenders().find((s) => s.track === oldTrack);
				if (sender) await sender.replaceTrack(newTrack);
			}

			this.detachSpeakingAnalyser(SELF_KEY);
			if (oldTrack) {
				oldTrack.stop();
				this.localStream.removeTrack(oldTrack);
			}
			this.localStream.addTrack(newTrack);
			this.applyMuted();
			this.attachSpeakingAnalyser(SELF_KEY, this.localStream);
			this.refreshNoiseSuppressionActive();
			this.notify();
		} catch {
			// couldn't switch - keep using the current input rather than dropping audio
		}
	}

	setOutputDevice(deviceId: string | null) {
		this.outputDeviceId = deviceId;
		this.persistSettings();
		this.reapplyOutputDevice();
		this.notify();
	}

	/// Moves every currently-open stream onto the chosen sink - called both
	/// when the setting changes and whenever a new remote audio element
	/// attaches (a stream that opened after the last call would otherwise
	/// stay on the default device). setNativeAppAudioSink no-ops instantly
	/// if not on Linux, so this is cheap to call liberally. No explicit
	/// choice ("System Default") still needs an actual sink name to move
	/// to - PipeWire doesn't have a "reset to default" verb, so this asks
	/// it what the current default is and targets that directly.
	reapplyOutputDevice() {
		if (!this.nativeOutputRouting) return;
		if (this.outputDeviceId) {
			void setNativeAppAudioSink(this.outputDeviceId);
		} else {
			void getNativeDefaultSink().then((sink) => {
				if (sink) void setNativeAppAudioSink(sink);
			});
		}
	}

	setOutputVolume(volume: number) {
		this.outputVolume = Math.min(1, Math.max(0, volume));
		this.persistSettings();
		this.notify();
	}

	async setNoiseSuppression(enabled: boolean): Promise<void> {
		this.noiseSuppression = enabled;
		this.persistSettings();
		const track = this.localStream?.getAudioTracks()[0];
		if (!track) return;
		try {
			await track.applyConstraints({ noiseSuppression: enabled });
		} catch {
			// device/browser doesn't support live constraint changes - takes effect on next join
		}
		this.refreshNoiseSuppressionActive();
	}

	/// What actually took effect, straight off the live track - not just
	/// what we asked for. getUserMedia constraints are a *request*; the
	/// platform can silently ignore them; this is the honest answer.
	private refreshNoiseSuppressionActive() {
		const track = this.localStream?.getAudioTracks()[0];
		if (!track) {
			this.noiseSuppressionActive = false;
			return;
		}
		try {
			this.noiseSuppressionActive = !!track.getSettings().noiseSuppression;
		} catch {
			this.noiseSuppressionActive = false;
		}
	}

	setPushToTalk(enabled: boolean) {
		this.pushToTalk = enabled;
		this.persistSettings();
		if (enabled) {
			this.pushToTalkActive = false;
			this.muted = true;
			this.applyMuted();
		} else {
			this.muted = false;
			this.applyMuted();
		}
	}

	setPushToTalkKey(code: string) {
		this.pushToTalkKey = code;
		this.persistSettings();
	}

	handlePushToTalkKeydown(code: string) {
		if (!this.pushToTalk || code !== this.pushToTalkKey || this.pushToTalkActive) return;
		this.pushToTalkActive = true;
		this.muted = false;
		this.applyMuted();
	}

	handlePushToTalkKeyup(code: string) {
		if (!this.pushToTalk || code !== this.pushToTalkKey) return;
		this.pushToTalkActive = false;
		this.muted = true;
		this.applyMuted();
	}

	/**
	 * @param dmCall true for a 1:1/group DM call (an actual outgoing call —
	 *   rings while you wait, auto-leaves if unanswered, posts a chat line).
	 *   false for a server voice channel, which people just hop in and out of.
	 */
	async join(token: string, roomId: string, label: string, dmCall = false): Promise<void> {
		if (this.roomId === roomId) return;
		const gen = ++this.joinGeneration;
		if (this.roomId) await this.leave();
		if (this.joinGeneration !== gen) return;

		this.roomId = roomId;
		this.label = label;
		this.status = "connecting";
		this.isDmCall = dmCall;
		this.joinedAt = Date.now();
		this.createdRoom = false;
		this.announcedStart = false;
		this.announcedMessageId = null;
		// Ring only for DM calls, and start it here synchronously while the
		// click's user activation is still valid (autoplay policy otherwise
		// blocks the play() that happens after the getUserMedia await).
		if (dmCall) startCallRing();

		try {
			const servers = await fetchIceServers(token);
			this.iceServers = servers.map((s) => ({
				urls: s.urls,
				username: s.username ?? undefined,
				credential: s.credential ?? undefined
			}));
		} catch {
			this.iceServers = FALLBACK_ICE_SERVERS;
		}
		if (this.joinGeneration !== gen) return;

		let localStream: MediaStream;
		try {
			localStream = await acquireMic(this.noiseSuppression, this.inputDeviceId);
		} catch (err) {
			this.teardown();
			throw asMicError(err);
		}
		// Superseded while acquiring the mic - drop the stream we just got.
		if (this.joinGeneration !== gen) {
			localStream.getTracks().forEach((t) => t.stop());
			return;
		}
		this.localStream = localStream;
		this.refreshDevices();
		if (this.pushToTalk) {
			this.muted = true;
			this.pushToTalkActive = false;
		}
		this.applyMuted();
		this.attachSpeakingAnalyser(SELF_KEY, this.localStream);
		this.refreshNoiseSuppressionActive();
		// Ringback was already started at the top of join(); this arms the
		// alone-timeout and stops the ring if a peer is somehow already here.
		this.syncAlone();
		this.notify();

		const ws = new WebSocket(`${WS_BASE_URL}/calls/${roomId}?token=${encodeURIComponent(token)}`);
		this.ws = ws;

		// If the socket hangs in CONNECTING (proxy/auth) it would otherwise
		// leave status "connecting" and the mic live forever - bound the wait,
		// then tear down and let the caller's catch surface it.
		await new Promise<void>((resolve, reject) => {
			this.connectTimer = setTimeout(() => {
				this.connectTimer = null;
				if (ws.readyState !== WebSocket.OPEN) {
					ws.close();
					this.teardown();
					reject(new Error("Call server did not respond"));
				}
			}, CallStore.CONNECT_TIMEOUT_MS);
			ws.onopen = () => {
				if (this.connectTimer) {
					clearTimeout(this.connectTimer);
					this.connectTimer = null;
				}
				this.status = "connected";
				resolve();
			};
			// Refused/dropped before it ever opened - resolve and let the
			// post-await checks below tear down + surface it.
			ws.onclose = () => resolve();
		});
		// Superseded by a newer join() while we were connecting.
		if (this.joinGeneration !== gen) return;
		if (ws.readyState !== WebSocket.OPEN) {
			this.teardown();
			throw new Error("Call server did not respond");
		}

		ws.onmessage = (event) => {
			try {
				this.handleServerMsg(JSON.parse(event.data as string) as ServerMsg);
			} catch {
				return;
			}
		};
		ws.onclose = () => {
			if (this.roomId === roomId) this.teardown();
		};
	}

	async leave(): Promise<void> {
		this.ws?.close();
		this.ws = null;
		this.teardown();
	}

	toggleMute() {
		this.muted = !this.muted;
		if (this.deafened && !this.muted) {
			this.deafened = false;
		}
		this.applyMuted();
	}

	toggleDeafen() {
		if (this.deafened) {
			this.deafened = false;
			this.muted = this.mutedBeforeDeafen;
		} else {
			this.mutedBeforeDeafen = this.muted;
			this.deafened = true;
			this.muted = true;
		}
		this.applyMuted();
	}

	private applyMuted() {
		this.localStream?.getAudioTracks().forEach((track) => {
			track.enabled = !this.muted;
		});
	}

	// Callers may now catch: a cancelled picker still resolves silently, but a
	// real failure rethrows (after undoing partial state) so the UI can toast —
	// map it with shareErrorKey(err, "toast.cameraFailed").
	async toggleCamera(): Promise<void> {
		if (this.cameraEnabled) {
			const tracks = this.localStream?.getVideoTracks() ?? [];
			for (const track of tracks) {
				track.stop();
				this.localStream?.removeTrack(track);
				for (const pc of this.pcs.values()) {
					const sender = pc.getSenders().find((s) => s.track === track);
					if (sender) pc.removeTrack(sender);
				}
			}
			this.cameraEnabled = false;
			this.notify();
			return;
		}

		let track: MediaStreamTrack | undefined;
		try {
			const camStream = await navigator.mediaDevices.getUserMedia({ video: true });
			track = camStream.getVideoTracks()[0];
			if (!this.localStream) this.localStream = new MediaStream();
			this.localStream.addTrack(track);

			for (const [peerId, pc] of this.pcs.entries()) {
				pc.addTrack(track, this.localStream);
				await this.renegotiate(peerId, pc);
			}

			this.cameraEnabled = true;
			this.notify();
		} catch (err) {
			// User denied permission / closed the picker → stay silent.
			if (err instanceof DOMException && (err.name === "NotAllowedError" || err.name === "AbortError")) {
				return;
			}
			// Real failure mid-setup: undo the partial camera state, then
			// rethrow so the caller can toast.
			if (track) {
				const dead = track;
				dead.stop();
				this.localStream?.removeTrack(dead);
				for (const pc of this.pcs.values()) {
					const sender = pc.getSenders().find((s) => s.track === dead);
					if (sender) pc.removeTrack(sender);
				}
			}
			this.cameraEnabled = false;
			this.notify();
			throw err;
		}
	}

	// Callers may now catch: a cancelled picker still resolves silently, but a
	// real failure rethrows (after tearing down half-state) so the UI can toast —
	// map it with shareErrorKey(err, "toast.screenShareFailed").
	/** Grab a display stream (shows the OS chooser once) shaped by our
	 *  picker's choices. The picker calls this itself so it can show a live
	 *  preview before you commit; `startScreenShareWithStream` then takes
	 *  that same stream to the call. */
	async acquireDisplayStream(opts?: ScreenShareOpts): Promise<MediaStream> {
		const video: MediaTrackConstraints & { displaySurface?: string } = {};
		if (opts?.width) video.width = { ideal: opts.width };
		if (opts?.height) video.height = { ideal: opts.height };
		if (opts?.frameRate) video.frameRate = { ideal: opts.frameRate };
		if (opts?.surface === "tab") video.displaySurface = "browser";
		else if (opts?.surface === "window") video.displaySurface = "window";
		else if (opts?.surface === "screen") video.displaySurface = "monitor";

		const displayOpts: DisplayMediaStreamOptions & Record<string, unknown> = {
			video: Object.keys(video).length ? video : true,
			audio: opts?.audio ?? false,
			selfBrowserSurface: "exclude",
			surfaceSwitching: "include",
			monitorTypeSurfaces: "include"
		};
		if (opts?.audio && opts?.surface !== "tab") {
			displayOpts.systemAudio = "include";
		}

		const stream = await navigator.mediaDevices.getDisplayMedia(displayOpts);
		const track = stream.getVideoTracks()[0];
		if (opts?.contentHint) track.contentHint = opts.contentHint;
		if (opts && (opts.width || opts.frameRate)) {
			track.applyConstraints(video).catch(() => {});
		}
		return stream;
	}

	/** Take an already-acquired display stream to the call: publish it to
	 *  every peer and flip `screenSharing`. */
	async startScreenShareWithStream(stream: MediaStream): Promise<void> {
		if (this.screenSharing) return;
		const track = stream.getVideoTracks()[0];
		if (!track) throw new DOMException("no video track", "NotFoundError");

		try {
			this.localScreenStream = stream;
			track.onended = () => this.stopScreenShareInternal();

			for (const [peerId, pc] of this.pcs.entries()) {
				const sender = pc.addTrack(track, stream);
				const neg = this.negotiation.get(peerId);
				if (neg) neg.makingOffer = true;
				try {
					const offer = await pc.createOffer();
					await pc.setLocalDescription(offer);

					const transceiver = pc.getTransceivers().find((t) => t.sender === sender);
					if (transceiver?.mid) {
						this.send({ type: "track-meta", to: peerId, mid: transceiver.mid, kind: "screen" });
					}

					this.send({ type: "offer", to: peerId, sdp: offer.sdp ?? "" });
				} finally {
					if (neg) neg.makingOffer = false;
				}
			}

			this.screenSharing = true;
			this.notify();
		} catch (err) {
			// Threw mid-loop (addTrack/createOffer): tear the half-state down.
			const partial = this.localScreenStream?.getVideoTracks()[0];
			if (partial) {
				for (const pc of this.pcs.values()) {
					const sender = pc.getSenders().find((s) => s.track === partial);
					if (sender) pc.removeTrack(sender);
				}
			}
			this.localScreenStream?.getTracks().forEach((t) => t.stop());
			this.localScreenStream = null;
			this.screenSharing = false;
			this.notify();
			throw err;
		}
	}

	async toggleScreenShare(opts?: ScreenShareOpts): Promise<void> {
		if (this.screenSharing) {
			this.stopScreenShareInternal();
			return;
		}
		try {
			const stream = await this.acquireDisplayStream(opts);
			await this.startScreenShareWithStream(stream);
		} catch (err) {
			// User dismissed the "choose what to share" picker → stay silent.
			if (err instanceof DOMException && (err.name === "NotAllowedError" || err.name === "AbortError")) {
				return;
			}
			throw err;
		}
	}

	private stopScreenShareInternal() {
		const track = this.localScreenStream?.getVideoTracks()[0];
		if (track) {
			track.stop();
			for (const pc of this.pcs.values()) {
				const sender = pc.getSenders().find((s) => s.track === track);
				if (sender) pc.removeTrack(sender);
			}
		}
		this.localScreenStream = null;
		this.screenSharing = false;
		// Only the LOCAL share is stopping here — nothing peer-keyed to prune.
		// Remote screenMids entries are pruned in removePeer() when a peer leaves.
		this.notify();
	}

	private async renegotiate(peerId: string, pc: RTCPeerConnection) {
		const neg = this.negotiation.get(peerId);
		if (neg) neg.makingOffer = true;
		try {
			const offer = await pc.createOffer();
			await pc.setLocalDescription(offer);
			this.send({ type: "offer", to: peerId, sdp: offer.sdp ?? "" });
		} finally {
			if (neg) neg.makingOffer = false;
		}
	}

	private send(msg: ClientMsg) {
		if (this.ws && this.ws.readyState === WebSocket.OPEN) {
			this.ws.send(JSON.stringify(msg));
		}
	}

	private createPeerConnection(userId: string, polite: boolean): RTCPeerConnection {
		const pc = new RTCPeerConnection({ iceServers: this.iceServers });

		if (this.localStream) {
			for (const track of this.localStream.getTracks()) {
				pc.addTrack(track, this.localStream);
			}
		}

		pc.ontrack = (event) => {
			const mid = event.transceiver?.mid;
			if (mid && this.screenMids.has(`${userId}::${mid}`)) {
				this.remoteScreenStreams.set(userId, event.streams[0]);
			} else {
				this.remoteStreams.set(userId, event.streams[0]);
				this.attachSpeakingAnalyser(userId, event.streams[0]);
			}
			this.notify();
		};

		pc.onicecandidate = (event) => {
			if (event.candidate) {
				this.send({ type: "ice-candidate", to: userId, candidate: JSON.stringify(event.candidate.toJSON()) });
			}
		};

		pc.onconnectionstatechange = () => {
			const state = pc.connectionState;
			if (state === "closed") {
				this.removePeer(userId);
				return;
			}
			if (state === "connected") {
				// Recovered - undo any pending drop for this peer.
				const timer = this.iceRestartTimers.get(userId);
				if (timer) {
					clearTimeout(timer);
					this.iceRestartTimers.delete(userId);
				}
				this.iceRestartAttempted.delete(userId);
				return;
			}
			if (state === "failed") {
				// "failed" is often a transient blip - try one ICE restart and
				// give it a grace window before dropping the peer for good.
				if (!this.iceRestartAttempted.has(userId)) {
					this.iceRestartAttempted.add(userId);
					try {
						pc.restartIce();
					} catch {
						// no-op if unsupported - the grace timer still runs
					}
				}
				if (!this.iceRestartTimers.has(userId)) {
					this.iceRestartTimers.set(
						userId,
						setTimeout(() => {
							this.iceRestartTimers.delete(userId);
							if (pc.connectionState === "failed") this.removePeer(userId);
						}, CallStore.ICE_RESTART_GRACE_MS)
					);
				}
			}
		};

		this.negotiation.set(userId, { polite, makingOffer: false, ignoreOffer: false });
		this.pcs.set(userId, pc);
		return pc;
	}

	private addParticipant(userId: string, username: string) {
		if (!this.participants.some((p) => p.userId === userId)) {
			this.participants = [...this.participants, { userId, username }];
		}
		this.syncAlone();
	}

	/**
	 * Ring while alone waiting for someone; auto-leave after ALONE_TIMEOUT_MS
	 * so an unanswered call doesn't keep the mic/socket open indefinitely.
	 */
	private syncAlone() {
		// Ringing + auto-leave only apply to DM calls. A server voice channel
		// you're alone in is normal — people drop in and out.
		const alone =
			this.isDmCall &&
			this.status !== "idle" &&
			this.roomId !== null &&
			this.participants.length === 0;
		if (alone) {
			startCallRing();
			if (!this.aloneTimer) {
				this.aloneTimer = setTimeout(() => {
					this.aloneTimer = null;
					if (this.status !== "idle" && this.participants.length === 0) void this.leave();
				}, CallStore.ALONE_TIMEOUT_MS);
			}
		} else {
			stopCallRing();
			if (this.aloneTimer) {
				clearTimeout(this.aloneTimer);
				this.aloneTimer = null;
			}
		}
	}

	private removePeer(userId: string) {
		const iceTimer = this.iceRestartTimers.get(userId);
		if (iceTimer) {
			clearTimeout(iceTimer);
			this.iceRestartTimers.delete(userId);
		}
		this.iceRestartAttempted.delete(userId);
		this.pcs.get(userId)?.close();
		this.pcs.delete(userId);
		this.negotiation.delete(userId);
		this.remoteStreams.delete(userId);
		this.remoteScreenStreams.delete(userId);
		// Drop this peer's screen-mid tags so a later transceiver-mid reuse
		// (e.g. a camera track on the same mid) isn't misrouted as a screen.
		for (const tag of this.screenMids) {
			if (tag.startsWith(`${userId}::`)) this.screenMids.delete(tag);
		}
		this.pendingCandidates.delete(userId);
		this.detachSpeakingAnalyser(userId);
		if (this.speakingUserIds.has(userId)) {
			const next = new Set(this.speakingUserIds);
			next.delete(userId);
			this.speakingUserIds = next;
		}
		this.participants = this.participants.filter((p) => p.userId !== userId);
		this.syncAlone();
		this.notify();
	}

	private async flushPendingCandidates(userId: string, pc: RTCPeerConnection) {
		const queued = this.pendingCandidates.get(userId);
		if (!queued) return;
		for (const candidate of queued) {
			await pc.addIceCandidate(candidate).catch(() => {});
		}
		this.pendingCandidates.delete(userId);
	}

	private async handleServerMsg(msg: ServerMsg) {
		switch (msg.type) {
			case "room-state":
				// Empty room-state on join means we opened this call.
				this.createdRoom = msg.members.length === 0;
				for (const member of msg.members) this.addParticipant(member.user_id, member.username);
				break;
			case "peer-joined": {
				this.addParticipant(msg.user_id, msg.username);
				// existing member: we drive the first offer, so we're impolite
				const pc = this.createPeerConnection(msg.user_id, false);
				await this.renegotiate(msg.user_id, pc);
				break;
			}
			case "peer-left":
				this.removePeer(msg.user_id);
				break;
			case "offer": {
				this.addParticipant(msg.from, msg.from_username);
				// a PC we make here (newcomer answering, or first contact) is polite
				const pc = this.pcs.get(msg.from) ?? this.createPeerConnection(msg.from, true);
				const neg = this.negotiation.get(msg.from);
				const collision = !!neg && (neg.makingOffer || pc.signalingState !== "stable");
				if (neg) neg.ignoreOffer = !neg.polite && collision;
				if (neg?.ignoreOffer) return; // impolite: keep our offer, drop theirs
				// polite: roll our in-flight offer back, then take theirs.
				// Rollback throws if we're not actually in have-local-offer
				// (narrow race before our setLocalDescription lands) - modern
				// browsers implicitly roll back on setRemoteDescription anyway.
				if (collision && pc.signalingState === "have-local-offer") {
					await pc.setLocalDescription({ type: "rollback" }).catch(() => {});
				}
				await pc.setRemoteDescription({ type: "offer", sdp: msg.sdp });
				await this.flushPendingCandidates(msg.from, pc);
				const answer = await pc.createAnswer();
				await pc.setLocalDescription(answer);
				this.send({ type: "answer", to: msg.from, sdp: answer.sdp ?? "" });
				break;
			}
			case "answer": {
				const pc = this.pcs.get(msg.from);
				if (!pc) return;
				await pc.setRemoteDescription({ type: "answer", sdp: msg.sdp });
				await this.flushPendingCandidates(msg.from, pc);
				break;
			}
			case "ice-candidate": {
				const candidate = JSON.parse(msg.candidate) as RTCIceCandidateInit;
				const pc = this.pcs.get(msg.from);
				if (pc && pc.remoteDescription) {
					await pc.addIceCandidate(candidate).catch(() => {});
				} else {
					const queued = this.pendingCandidates.get(msg.from) ?? [];
					queued.push(candidate);
					this.pendingCandidates.set(msg.from, queued);
				}
				break;
			}
			case "track-meta": {
				if (msg.kind === "screen") {
					this.screenMids.add(`${msg.from}::${msg.mid}`);
				}
				break;
			}
		}
	}

	private attachSpeakingAnalyser(key: string, stream: MediaStream) {
		if (this.analysers.has(key) || stream.getAudioTracks().length === 0) return;
		try {
			if (!this.audioCtx) this.audioCtx = new AudioContext();
			const ctx = this.audioCtx;
			if (ctx.state === "suspended") void ctx.resume();
			const source = ctx.createMediaStreamSource(stream);
			const analyser = ctx.createAnalyser();
			analyser.fftSize = 512;
			analyser.smoothingTimeConstant = 0.6;
			source.connect(analyser);
			this.analysers.set(key, { analyser, data: new Uint8Array(analyser.frequencyBinCount) });
			this.startSpeakingLoop();
			this.startStatsPolling();
		} catch {
			return;
		}
	}

	private detachSpeakingAnalyser(key: string) {
		this.analysers.delete(key);
		this.lastSpokeAt.delete(key);
	}

	private startSpeakingLoop() {
		if (this.speakingInterval) return;
		this.speakingInterval = setInterval(() => this.pollSpeaking(), SPEAKING_POLL_MS);
	}

	private stopSpeakingLoop() {
		if (this.speakingInterval) {
			clearInterval(this.speakingInterval);
			this.speakingInterval = null;
		}
	}

	private startStatsPolling() {
		if (this.statsInterval) return;
		this.statsInterval = setInterval(() => this.pollStats(), STATS_POLL_MS);
	}

	private stopStatsPolling() {
		if (this.statsInterval) {
			clearInterval(this.statsInterval);
			this.statsInterval = null;
		}
		this.connectionQuality = {};
	}

	/// Classifies each peer's connection off the selected candidate pair's
	/// round-trip time - a single number that's meaningful for both
	/// audio-only and video calls, unlike fishing for jitter/packet-loss
	/// fields whose shape varies by codec and browser. Self's badge is the
	/// worst of everyone else's, since a bad connection is symmetric enough
	/// that "how do they see me" isn't worth a second, separate probe.
	private async pollStats() {
		const next: Record<string, "good" | "medium" | "poor"> = {};
		for (const [peerId, pc] of this.pcs) {
			try {
				const stats = await pc.getStats();
				let rttMs: number | null = null;
				stats.forEach((report) => {
					if (
						report.type === "candidate-pair" &&
						report.state === "succeeded" &&
						typeof report.currentRoundTripTime === "number"
					) {
						rttMs = report.currentRoundTripTime * 1000;
					}
				});
				if (rttMs === null) continue;
				next[peerId] = rttMs < 150 ? "good" : rttMs < 350 ? "medium" : "poor";
			} catch {
				// stats unavailable for this peer this tick - leave it unset
			}
		}
		const values = Object.values(next);
		next[SELF_KEY] = values.includes("poor") ? "poor" : values.includes("medium") ? "medium" : "good";
		this.connectionQuality = next;
	}

	private pollSpeaking() {
		const now = Date.now();
		for (const [key, { analyser, data }] of this.analysers) {
			analyser.getByteTimeDomainData(data);
			let sumSquares = 0;
			for (let i = 0; i < data.length; i++) {
				const deviation = data[i] - 128;
				sumSquares += deviation * deviation;
			}
			const rms = Math.sqrt(sumSquares / data.length);
			if (rms > SPEAKING_THRESHOLD) this.lastSpokeAt.set(key, now);
		}

		let nextSelfSpeaking = false;
		const nextSpeaking = new Set<string>();
		for (const [key, ts] of this.lastSpokeAt) {
			if (now - ts >= SPEAKING_HOLD_MS) continue;
			if (key === SELF_KEY) nextSelfSpeaking = true;
			else nextSpeaking.add(key);
		}

		if (nextSelfSpeaking !== this.selfSpeaking) this.selfSpeaking = nextSelfSpeaking;

		const changed =
			nextSpeaking.size !== this.speakingUserIds.size ||
			[...nextSpeaking].some((id) => !this.speakingUserIds.has(id));
		if (changed) this.speakingUserIds = nextSpeaking;
	}

	private teardown() {
		stopCallRing();
		if (this.aloneTimer) {
			clearTimeout(this.aloneTimer);
			this.aloneTimer = null;
		}
		if (this.connectTimer) {
			clearTimeout(this.connectTimer);
			this.connectTimer = null;
		}
		for (const timer of this.iceRestartTimers.values()) clearTimeout(timer);
		this.iceRestartTimers.clear();
		this.iceRestartAttempted.clear();
		if (this.roomId && this.createdRoom && this.announcedMessageId && this.joinedAt) {
			this.callEndEdit = {
				roomId: this.roomId,
				messageId: this.announcedMessageId,
				durationSec: Math.max(1, Math.round((Date.now() - this.joinedAt) / 1000))
			};
		}
		this.joinedAt = 0;
		this.isDmCall = false;
		this.createdRoom = false;
		this.announcedStart = false;
		this.announcedMessageId = null;
		for (const pc of this.pcs.values()) pc.close();
		this.pcs.clear();
		this.negotiation.clear();
		this.remoteStreams.clear();
		this.remoteScreenStreams.clear();
		this.screenMids.clear();
		this.pendingCandidates.clear();
		this.localStream?.getTracks().forEach((track) => track.stop());
		this.localStream = null;
		this.localScreenStream?.getTracks().forEach((track) => track.stop());
		this.localScreenStream = null;
		this.stopSpeakingLoop();
		this.stopStatsPolling();
		this.analysers.clear();
		this.lastSpokeAt.clear();
		this.audioCtx?.close().catch(() => {});
		this.audioCtx = null;
		this.selfSpeaking = false;
		this.speakingUserIds = new Set();
		this.noiseSuppressionActive = false;
		this.roomId = null;
		this.label = "";
		this.status = "idle";
		this.participants = [];
		this.muted = false;
		this.deafened = false;
		this.mutedBeforeDeafen = false;
		this.cameraEnabled = false;
		this.screenSharing = false;
		this.notify();
	}
}

export const call = new CallStore();

if (typeof window !== "undefined") {
	window.addEventListener("keydown", (e) => {
		if (e.repeat) return;
		call.handlePushToTalkKeydown(e.code);
	});
	window.addEventListener("keyup", (e) => call.handlePushToTalkKeyup(e.code));
	window.addEventListener("blur", () => call.handlePushToTalkKeyup(call.pushToTalkKey));
}

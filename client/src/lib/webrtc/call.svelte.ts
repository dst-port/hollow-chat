import { WS_BASE_URL, fetchIceServers } from "$lib/api/client";

type Participant = { userId: string; username: string };

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

class CallStore {
	roomId = $state<string | null>(null);
	label = $state("");
	status = $state<"idle" | "connecting" | "connected">("idle");
	muted = $state(false);
	cameraEnabled = $state(false);
	screenSharing = $state(false);
	participants = $state<Participant[]>([]);

	private ws: WebSocket | null = null;
	private pcs = new Map<string, RTCPeerConnection>();
	private remoteStreams = new Map<string, MediaStream>();
	private remoteScreenStreams = new Map<string, MediaStream>();
	private screenMids = new Set<string>();
	private pendingCandidates = new Map<string, RTCIceCandidateInit[]>();
	private localStream: MediaStream | null = null;
	private localScreenStream: MediaStream | null = null;
	private iceServers: RTCIceServer[] = FALLBACK_ICE_SERVERS;
	private listeners = new Set<() => void>();

	onStreamsChanged(callback: () => void): () => void {
		this.listeners.add(callback);
		return () => this.listeners.delete(callback);
	}

	private notify() {
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

	async join(token: string, roomId: string, label: string): Promise<void> {
		if (this.roomId === roomId) return;
		if (this.roomId) await this.leave();

		this.roomId = roomId;
		this.label = label;
		this.status = "connecting";

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

		try {
			this.localStream = await navigator.mediaDevices.getUserMedia({ audio: true });
		} catch {
			this.teardown();
			throw new Error("Microphone access was denied");
		}
		this.applyMuted();
		this.notify();

		const ws = new WebSocket(`${WS_BASE_URL}/calls/${roomId}?token=${encodeURIComponent(token)}`);
		this.ws = ws;
		ws.onopen = () => {
			this.status = "connected";
		};
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
		this.applyMuted();
	}

	private applyMuted() {
		this.localStream?.getAudioTracks().forEach((track) => {
			track.enabled = !this.muted;
		});
	}

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

		try {
			const camStream = await navigator.mediaDevices.getUserMedia({ video: true });
			const track = camStream.getVideoTracks()[0];
			if (!this.localStream) this.localStream = new MediaStream();
			this.localStream.addTrack(track);

			for (const [peerId, pc] of this.pcs.entries()) {
				pc.addTrack(track, this.localStream);
				await this.renegotiate(peerId, pc);
			}

			this.cameraEnabled = true;
			this.notify();
		} catch {
			return;
		}
	}

	async toggleScreenShare(): Promise<void> {
		if (this.screenSharing) {
			this.stopScreenShareInternal();
			return;
		}

		try {
			const screenStream = await navigator.mediaDevices.getDisplayMedia({ video: true });
			const track = screenStream.getVideoTracks()[0];
			this.localScreenStream = screenStream;
			track.onended = () => this.stopScreenShareInternal();

			for (const [peerId, pc] of this.pcs.entries()) {
				const sender = pc.addTrack(track, screenStream);
				await this.renegotiate(peerId, pc);
				const transceiver = pc.getTransceivers().find((t) => t.sender === sender);
				if (transceiver?.mid) {
					this.send({ type: "track-meta", to: peerId, mid: transceiver.mid, kind: "screen" });
				}
			}

			this.screenSharing = true;
			this.notify();
		} catch {
			return;
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
		this.notify();
	}

	private async renegotiate(peerId: string, pc: RTCPeerConnection) {
		const offer = await pc.createOffer();
		await pc.setLocalDescription(offer);
		this.send({ type: "offer", to: peerId, sdp: offer.sdp ?? "" });
	}

	private send(msg: ClientMsg) {
		if (this.ws && this.ws.readyState === WebSocket.OPEN) {
			this.ws.send(JSON.stringify(msg));
		}
	}

	private createPeerConnection(userId: string): RTCPeerConnection {
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
			}
			this.notify();
		};

		pc.onicecandidate = (event) => {
			if (event.candidate) {
				this.send({ type: "ice-candidate", to: userId, candidate: JSON.stringify(event.candidate.toJSON()) });
			}
		};

		pc.onconnectionstatechange = () => {
			if (pc.connectionState === "failed" || pc.connectionState === "closed") {
				this.removePeer(userId);
			}
		};

		this.pcs.set(userId, pc);
		return pc;
	}

	private addParticipant(userId: string, username: string) {
		if (!this.participants.some((p) => p.userId === userId)) {
			this.participants = [...this.participants, { userId, username }];
		}
	}

	private removePeer(userId: string) {
		this.pcs.get(userId)?.close();
		this.pcs.delete(userId);
		this.remoteStreams.delete(userId);
		this.remoteScreenStreams.delete(userId);
		this.pendingCandidates.delete(userId);
		this.participants = this.participants.filter((p) => p.userId !== userId);
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
				for (const member of msg.members) this.addParticipant(member.user_id, member.username);
				break;
			case "peer-joined": {
				this.addParticipant(msg.user_id, msg.username);
				const pc = this.createPeerConnection(msg.user_id);
				await this.renegotiate(msg.user_id, pc);
				break;
			}
			case "peer-left":
				this.removePeer(msg.user_id);
				break;
			case "offer": {
				this.addParticipant(msg.from, msg.from_username);
				const pc = this.pcs.get(msg.from) ?? this.createPeerConnection(msg.from);
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

	private teardown() {
		for (const pc of this.pcs.values()) pc.close();
		this.pcs.clear();
		this.remoteStreams.clear();
		this.remoteScreenStreams.clear();
		this.screenMids.clear();
		this.pendingCandidates.clear();
		this.localStream?.getTracks().forEach((track) => track.stop());
		this.localStream = null;
		this.localScreenStream?.getTracks().forEach((track) => track.stop());
		this.localScreenStream = null;
		this.roomId = null;
		this.label = "";
		this.status = "idle";
		this.participants = [];
		this.muted = false;
		this.cameraEnabled = false;
		this.screenSharing = false;
		this.notify();
	}
}

export const call = new CallStore();

import { WS_BASE_URL } from "$lib/api/client";
import { encrypt, decrypt } from "$lib/crypto/aead";
import { toBase64, fromBase64, utf8Encode, utf8Decode } from "$lib/crypto/encoding";

function syncKeyStorageKey(username: string): string {
	return `hollowchat_devicesync_${username}`;
}

function seqStorageKey(username: string): string {
	return `hollowchat_devicesync_seq_${username}`;
}

function loadSeq(username: string): number {
	const raw = localStorage.getItem(seqStorageKey(username));
	const parsed = raw ? parseInt(raw, 10) : 0;
	return Number.isFinite(parsed) ? parsed : 0;
}

function saveSeq(username: string, seq: number) {
	localStorage.setItem(seqStorageKey(username), String(seq));
}

export function hasSyncKey(username: string): boolean {
	return localStorage.getItem(syncKeyStorageKey(username)) !== null;
}

export function persistSyncKey(username: string, key: Uint8Array) {
	localStorage.setItem(syncKeyStorageKey(username), toBase64(key));
}

function loadSyncKey(username: string): Uint8Array | null {
	const raw = localStorage.getItem(syncKeyStorageKey(username));
	return raw ? fromBase64(raw) : null;
}

type WireMsg = { kind: "delta"; key: string; value: string } | { kind: "claim"; peer: string; token: string };
type Envelope = { nonce: string; ciphertext: string };

const RECONNECT_DELAY_MS = 5000;
const CLAIM_WAIT_MS = 300;
const SYNC_WAIT_MS = 1500;

class DeviceSyncStore {
	private ws: WebSocket | null = null;
	private key: Uint8Array | null = null;
	private username: string | null = null;
	private token: string | null = null;
	private closedByUs = false;
	private outbox: WireMsg[] = [];
	private claimListeners = new Map<string, (theirToken: string) => void>();
	private deltaWaiters = new Map<string, () => void>();

	connect(token: string, username: string) {
		if (this.ws && this.username === username) return;
		const key = loadSyncKey(username);
		if (!key) return;

		this.closedByUs = false;
		this.key = key;
		this.username = username;
		this.token = token;
		this.openSocket();
	}

	private openSocket() {
		const token = this.token;
		const username = this.username;
		if (!token || !username) return;
		const since = loadSeq(username);
		const ws = new WebSocket(
			`${WS_BASE_URL}/devicelink?token=${encodeURIComponent(token)}&since=${since}`
		);
		this.ws = ws;
		ws.onopen = () => this.flushOutbox();
		ws.onmessage = (event) => {
			this.handleMessage(event.data as string).catch(() => {});
		};
		ws.onclose = () => {
			this.ws = null;
			if (this.closedByUs) return;
			setTimeout(() => {
				if (!this.closedByUs && this.key) this.openSocket();
			}, RECONNECT_DELAY_MS);
		};
	}

	private flushOutbox() {
		const pending = this.outbox;
		this.outbox = [];
		for (const wire of pending) this.sendWire(wire);
	}

	private async handleMessage(raw: string) {
		if (!this.key) return;
		let msg: { type?: string; payload?: string; id?: number };
		try {
			msg = JSON.parse(raw);
		} catch {
			return;
		}
		if (msg.type !== "data" || !msg.payload) return;

		let envelope: Envelope;
		try {
			envelope = JSON.parse(msg.payload);
		} catch {
			return;
		}
		if (!envelope.nonce || !envelope.ciphertext) return;

		let wire: WireMsg;
		try {
			const plaintext = await decrypt(this.key, fromBase64(envelope.nonce), fromBase64(envelope.ciphertext), new Uint8Array(0));
			wire = JSON.parse(utf8Decode(plaintext)) as WireMsg;
		} catch {
			return;
		}

		if (typeof msg.id === "number" && this.username) {
			saveSeq(this.username, Math.max(loadSeq(this.username), msg.id));
		}

		if (wire.kind === "delta") {
			if (typeof wire.key === "string" && typeof wire.value === "string") {
				localStorage.setItem(wire.key, wire.value);
				this.deltaWaiters.get(wire.key)?.();
			}
		} else if (wire.kind === "claim") {
			this.claimListeners.get(wire.peer)?.(wire.token);
		}
	}

	private sendWire(wire: WireMsg) {
		if (!this.key) return;
		const syncKey = this.key;
		encrypt(syncKey, utf8Encode(JSON.stringify(wire)), new Uint8Array(0))
			.then(({ nonce, ciphertext }) => {
				const envelope: Envelope = { nonce: toBase64(nonce), ciphertext: toBase64(ciphertext) };
				if (this.ws && this.ws.readyState === WebSocket.OPEN) {
					this.ws.send(JSON.stringify({ type: "data", payload: JSON.stringify(envelope) }));
				} else {
					this.outbox.push(wire);
				}
			})
			.catch(() => {});
	}

	private isActiveFor(username: string): boolean {
		return this.username === username && !!this.key;
	}

	broadcastChange(username: string, key: string, value: string) {
		if (!this.isActiveFor(username)) return;
		this.sendWire({ kind: "delta", key, value });
	}

	async claimNewSession(username: string, peer: string): Promise<"proceed" | "wait-for-sync"> {
		if (!this.isActiveFor(username)) return "proceed";
		const myToken = crypto.randomUUID();

		return new Promise((resolve) => {
			let resolved = false;
			const finish = (outcome: "proceed" | "wait-for-sync") => {
				if (resolved) return;
				resolved = true;
				this.claimListeners.delete(peer);
				resolve(outcome);
			};

			this.claimListeners.set(peer, (theirToken) => {
				finish(myToken > theirToken ? "proceed" : "wait-for-sync");
			});
			this.sendWire({ kind: "claim", peer, token: myToken });
			setTimeout(() => finish("proceed"), CLAIM_WAIT_MS);
		});
	}

	async waitForSync(username: string, storageKey: string): Promise<void> {
		if (!this.isActiveFor(username)) return;
		return new Promise((resolve) => {
			let resolved = false;
			const finish = () => {
				if (resolved) return;
				resolved = true;
				this.deltaWaiters.delete(storageKey);
				resolve();
			};
			this.deltaWaiters.set(storageKey, finish);
			setTimeout(finish, SYNC_WAIT_MS);
		});
	}

	disconnect() {
		this.closedByUs = true;
		this.ws?.close();
		this.ws = null;
		this.key = null;
		this.username = null;
		this.token = null;
		this.outbox = [];
		this.claimListeners.clear();
		this.deltaWaiters.clear();
	}
}

export const deviceSync = new DeviceSyncStore();

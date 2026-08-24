import { WS_BASE_URL } from "$lib/api/client";
import { generateX25519KeyPair, dh, kdf, type KeyPair } from "$lib/crypto/primitives";
import { encrypt, decrypt } from "$lib/crypto/aead";
import { toBase64, fromBase64, utf8Encode, utf8Decode } from "$lib/crypto/encoding";
import { exportLocalCryptoState, importLocalCryptoState } from "$lib/crypto/devicestate";
import { persistSyncKey, deviceSync } from "./sync";

type ServerMsg =
	| { type: "peer-joined" }
	| { type: "peer-left" }
	| { type: "data"; payload: string };

type ClientMsg = { type: "data"; payload: string };

type WireMsg =
	| { step: "pubkey"; value: string }
	| { step: "state"; nonce: string; ciphertext: string };

export type LinkPhase =
	| "idle"
	| "connecting"
	| "waiting-for-peer"
	| "confirm"
	| "sending"
	| "receiving"
	| "done"
	| "error";

class DeviceLinkStore {
	phase = $state<LinkPhase>("idle");
	fingerprint = $state<string | null>(null);
	error = $state<string | null>(null);

	private ws: WebSocket | null = null;
	private ephemeral: KeyPair | null = null;
	private sharedKey: Uint8Array | null = null;
	private token: string | null = null;
	private username: string | null = null;

	start(token: string, username: string) {
		this.reset();
		this.phase = "connecting";
		this.ephemeral = generateX25519KeyPair();
		this.token = token;
		this.username = username;

		const ws = new WebSocket(`${WS_BASE_URL}/devicelink?token=${encodeURIComponent(token)}`);
		this.ws = ws;

		ws.onopen = () => {
			this.phase = "waiting-for-peer";
			this.sendWire({ step: "pubkey", value: toBase64(this.ephemeral!.publicKey) });
		};
		ws.onmessage = (event) => {
			try {
				this.handleServerMsg(JSON.parse(event.data as string) as ServerMsg);
			} catch {
				return;
			}
		};
		ws.onerror = () => {
			this.phase = "error";
			this.error = "Could not connect";
		};
		ws.onclose = () => {
			if (this.phase !== "done") {
				this.phase = "error";
				this.error = this.error ?? "Connection closed";
			}
		};
	}

	private handleServerMsg(msg: ServerMsg) {
		if (msg.type === "peer-joined") {
			this.sendWire({ step: "pubkey", value: toBase64(this.ephemeral!.publicKey) });
			return;
		}
		if (msg.type === "peer-left") {
			if (this.phase !== "done") {
				this.phase = "error";
				this.error = "The other device disconnected";
			}
			return;
		}
		if (msg.type === "data") {
			let wire: WireMsg;
			try {
				wire = JSON.parse(msg.payload) as WireMsg;
			} catch {
				return;
			}
			if (wire.step === "pubkey") {
				this.onPeerPubkey(wire.value);
			} else if (wire.step === "state") {
				this.onEncryptedState(wire.nonce, wire.ciphertext);
			}
		}
	}

	private onPeerPubkey(theirPubB64: string) {
		if (!this.ephemeral || this.sharedKey) return;
		const shared = dh(this.ephemeral.privateKey, fromBase64(theirPubB64));
		this.sharedKey = kdf(shared, new Uint8Array(0), "hollowchat-devicelink-key", 32);
		const fpBytes = kdf(shared, new Uint8Array(0), "hollowchat-devicelink-fingerprint", 4);
		const code = new DataView(fpBytes.buffer, fpBytes.byteOffset, fpBytes.byteLength).getUint32(0) % 1000000;
		this.fingerprint = code.toString().padStart(6, "0");
		this.phase = "confirm";
	}

	async confirmAndSend(username: string) {
		if (!this.sharedKey || this.phase !== "confirm") return;
		this.phase = "sending";
		try {
			const blob = exportLocalCryptoState(username);
			const { nonce, ciphertext } = await encrypt(this.sharedKey, utf8Encode(blob), new Uint8Array(0));
			this.sendWire({ step: "state", nonce: toBase64(nonce), ciphertext: toBase64(ciphertext) });
			persistSyncKey(username, this.sharedKey);
			if (this.token) deviceSync.connect(this.token, username);
			this.phase = "done";
		} catch {
			this.phase = "error";
			this.error = "Failed to send keys";
		}
	}

	private async onEncryptedState(nonceB64: string, ciphertextB64: string) {
		if (!this.sharedKey || !this.username) return;
		this.phase = "receiving";
		try {
			const plaintext = await decrypt(this.sharedKey, fromBase64(nonceB64), fromBase64(ciphertextB64), new Uint8Array(0));
			importLocalCryptoState(utf8Decode(plaintext));
			persistSyncKey(this.username, this.sharedKey);
			if (this.token) deviceSync.connect(this.token, this.username);
			this.phase = "done";
		} catch {
			this.phase = "error";
			this.error = "Failed to import keys";
		}
	}

	private sendWire(wire: WireMsg) {
		this.send({ type: "data", payload: JSON.stringify(wire) });
	}

	private send(msg: ClientMsg) {
		if (this.ws && this.ws.readyState === WebSocket.OPEN) {
			this.ws.send(JSON.stringify(msg));
		}
	}

	reset() {
		this.ws?.close();
		this.ws = null;
		this.ephemeral = null;
		this.sharedKey = null;
		this.token = null;
		this.username = null;
		this.phase = "idle";
		this.fingerprint = null;
		this.error = null;
	}
}

export const deviceLink = new DeviceLinkStore();

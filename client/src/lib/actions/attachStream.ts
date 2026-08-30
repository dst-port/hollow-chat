import { call } from "$lib/webrtc/call.svelte";

// Some WebViews (WKWebView, WebKitGTK) don't honour the `autoplay` attribute
// reliably once srcObject is swapped in after mount - nudge them.
//
// If that nudge is rejected (user-activation from the click that started the
// call has already expired) the element is parked here and retried on the next
// real gesture - otherwise you join a call and silently hear nobody.
const blocked = new Set<HTMLMediaElement>();
let retryListening = false;

function retryBlocked() {
	for (const el of blocked) {
		const p = el.play();
		if (p && typeof p.then === "function") {
			p.then(() => blocked.delete(el)).catch(() => {});
		} else {
			blocked.delete(el);
		}
	}
}

// Also reachable from sound.ts's first-gesture unlock.
export function retryBlockedCallMedia() {
	retryBlocked();
}

function installRetryListeners() {
	if (retryListening || typeof document === "undefined") return;
	retryListening = true;
	document.addEventListener("pointerdown", retryBlocked, true);
	document.addEventListener("keydown", retryBlocked, true);
}

function kick(node: HTMLMediaElement) {
	const p = node.play();
	if (p && typeof p.then === "function") {
		p.then(() => blocked.delete(node)).catch(() => {
			blocked.add(node);
			installRetryListeners();
		});
	}
}

function applyOutputSettings(node: HTMLMediaElement) {
	node.volume = call.outputVolume;
	const sinkCapable = node as HTMLMediaElement & { setSinkId?: (id: string) => Promise<void> };
	if (call.outputDeviceId && typeof sinkCapable.setSinkId === "function") {
		sinkCapable.setSinkId(call.outputDeviceId).catch(() => {
			// device unavailable
		});
	}
	call.reapplyOutputDevice();
}

export function attachRemoteStream(node: HTMLMediaElement, userId: string) {
	function update() {
		const stream = call.getRemoteStream(userId);
		if (stream && node.srcObject !== stream) {
			node.srcObject = stream;
			kick(node);
		}
		applyOutputSettings(node);
	}
	update();
	const unsubscribe = call.onStreamsChanged(update);
	return {
		update(newUserId: string) {
			userId = newUserId;
			update();
		},
		destroy() {
			unsubscribe();
			blocked.delete(node);
		}
	};
}

export function attachLocalStream(node: HTMLMediaElement) {
	function update() {
		const stream = call.getLocalStream();
		if (stream && node.srcObject !== stream) {
			node.srcObject = stream;
			kick(node);
		}
	}
	update();
	const unsubscribe = call.onStreamsChanged(update);
	return {
		destroy() {
			unsubscribe();
			blocked.delete(node);
		}
	};
}

export function attachRemoteScreenStream(node: HTMLMediaElement, userId: string) {
	function update() {
		const stream = call.getRemoteScreenStream(userId);
		if (stream && node.srcObject !== stream) {
			node.srcObject = stream;
			kick(node);
		}
	}
	update();
	const unsubscribe = call.onStreamsChanged(update);
	return {
		update(newUserId: string) {
			userId = newUserId;
			update();
		},
		destroy() {
			unsubscribe();
			blocked.delete(node);
		}
	};
}

export function attachLocalScreenStream(node: HTMLMediaElement) {
	function update() {
		const stream = call.getLocalScreenStream();
		if (stream && node.srcObject !== stream) {
			node.srcObject = stream;
			kick(node);
		}
	}
	update();
	const unsubscribe = call.onStreamsChanged(update);
	return {
		destroy() {
			unsubscribe();
			blocked.delete(node);
		}
	};
}

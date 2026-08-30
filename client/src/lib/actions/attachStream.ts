import { call } from "$lib/webrtc/call.svelte";

// Some WebViews (WKWebView, WebKitGTK) don't honour the `autoplay` attribute
// reliably once srcObject is swapped in after mount - nudge them.
function kick(node: HTMLMediaElement) {
	const p = node.play();
	if (p && typeof p.catch === "function") p.catch(() => {});
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
		}
	};
}

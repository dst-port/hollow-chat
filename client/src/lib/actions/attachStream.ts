import { call } from "$lib/webrtc/call.svelte";

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
		if (stream && node.srcObject !== stream) node.srcObject = stream;
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
		if (stream && node.srcObject !== stream) node.srcObject = stream;
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
		if (stream && node.srcObject !== stream) node.srcObject = stream;
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
		if (stream && node.srcObject !== stream) node.srcObject = stream;
	}
	update();
	const unsubscribe = call.onStreamsChanged(update);
	return {
		destroy() {
			unsubscribe();
		}
	};
}

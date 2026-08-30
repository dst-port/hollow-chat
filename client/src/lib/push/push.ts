import { base } from "$app/paths";
import { getVapidKey, pushSubscribe, pushUnsubscribe } from "$lib/api/client";

// Web Push opt-in. The desktop (Tauri) build has no service worker / PushManager
// and simply reports "unsupported"; it already does OS notifications while
// running via notifyDesktop().

const ENABLED_KEY = "hollowchat.push.enabled";

export type EnableResult = "enabled" | "denied" | "unsupported" | "error";

export function pushSupported(): boolean {
	return (
		typeof navigator !== "undefined" &&
		"serviceWorker" in navigator &&
		typeof window !== "undefined" &&
		"PushManager" in window &&
		"Notification" in window
	);
}

export function pushEnabledLocally(): boolean {
	try {
		return localStorage.getItem(ENABLED_KEY) === "true";
	} catch {
		return false;
	}
}

function setEnabledLocally(value: boolean) {
	try {
		localStorage.setItem(ENABLED_KEY, String(value));
	} catch {
		/* storage unavailable */
	}
}

function urlBase64ToUint8Array(value: string): Uint8Array {
	const padding = "=".repeat((4 - (value.length % 4)) % 4);
	const b64 = (value + padding).replace(/-/g, "+").replace(/_/g, "/");
	const raw = atob(b64);
	const out = new Uint8Array(raw.length);
	for (let i = 0; i < raw.length; i++) out[i] = raw.charCodeAt(i);
	return out;
}

async function register(): Promise<ServiceWorkerRegistration | null> {
	try {
		return await navigator.serviceWorker.register(`${base}/service-worker.js`, { type: "module" });
	} catch {
		return null;
	}
}

export async function enablePush(token: string): Promise<EnableResult> {
	if (!pushSupported()) return "unsupported";

	const reg = await register();
	if (!reg) return "error";

	let permission = Notification.permission;
	if (permission === "default") {
		permission = await Notification.requestPermission();
	}
	if (permission !== "granted") return "denied";

	let vapid: string | null;
	try {
		vapid = (await getVapidKey()).key;
	} catch {
		return "error";
	}
	if (!vapid) return "unsupported";

	try {
		await navigator.serviceWorker.ready;
		let sub = await reg.pushManager.getSubscription();
		if (!sub) {
			sub = await reg.pushManager.subscribe({
				userVisibleOnly: true,
				applicationServerKey: urlBase64ToUint8Array(vapid)
			});
		}
		const json = sub.toJSON();
		if (!json.endpoint || !json.keys?.p256dh || !json.keys?.auth) return "error";
		await pushSubscribe(token, {
			endpoint: json.endpoint,
			keys: { p256dh: json.keys.p256dh, auth: json.keys.auth }
		});
		setEnabledLocally(true);
		return "enabled";
	} catch {
		return "error";
	}
}

export async function disablePush(token: string): Promise<void> {
	setEnabledLocally(false);
	try {
		const reg = await navigator.serviceWorker.getRegistration();
		const sub = await reg?.pushManager.getSubscription();
		if (sub) {
			await pushUnsubscribe(token, sub.endpoint).catch(() => {});
			await sub.unsubscribe().catch(() => {});
		}
	} catch {
		/* nothing to clean up */
	}
}

/**
 * Re-assert an existing opt-in on startup - the browser can silently rotate or
 * drop a push subscription, so we re-subscribe and re-register it server-side.
 */
export async function refreshPush(token: string): Promise<void> {
	if (!pushEnabledLocally() || !pushSupported()) return;
	if (Notification.permission !== "granted") return;
	await enablePush(token).catch(() => {});
}

/// <reference types="@sveltejs/kit" />
/// <reference lib="webworker" />

// Push notifications for when the app isn't open. Registered manually from
// $lib/push/push.ts (web only). Kept deliberately tiny - no offline caching.
import { base } from "$service-worker";

const sw = self as unknown as ServiceWorkerGlobalScope;

sw.addEventListener("install", () => {
	sw.skipWaiting();
});

sw.addEventListener("activate", (event) => {
	event.waitUntil(sw.clients.claim());
});

type PushData = { title?: string; body?: string; url?: string; tag?: string };

sw.addEventListener("push", (event) => {
	let data: PushData = {};
	try {
		data = (event.data?.json() as PushData) ?? {};
	} catch {
		data = { body: event.data?.text() };
	}

	const title = data.title || "HollowChat";
	event.waitUntil(
		sw.registration.showNotification(title, {
			body: data.body ?? "",
			tag: data.tag,
			renotify: Boolean(data.tag),
			icon: `${base}/favicon.png`,
			badge: `${base}/favicon.png`,
			data: { url: data.url || `${base}/` }
		})
	);
});

sw.addEventListener("notificationclick", (event) => {
	event.notification.close();
	const target = (event.notification.data as { url?: string } | undefined)?.url || `${base}/`;
	event.waitUntil(
		sw.clients.matchAll({ type: "window", includeUncontrolled: true }).then((clients) => {
			for (const client of clients) {
				if ("focus" in client) {
					void client.focus();
					return;
				}
			}
			return sw.clients.openWindow(target).then(() => undefined);
		})
	);
});

// Notification deep-links. A push notification carries a url like
//   /app/?goto=dm:<dmId>
//   /app/?goto=channel:<serverId>:<channelId>
// When the app is opened cold that lands in location.search; when it's
// already open the service worker forwards the url as a postMessage. Either
// way we parse it into a target the shell consumes once.

export type DeepLinkTarget =
	| { kind: "dm"; dmId: string }
	| { kind: "channel"; serverId: string; channelId: string };

function parseGoto(value: string | null): DeepLinkTarget | null {
	if (!value) return null;
	const parts = value.split(":");
	if (parts[0] === "dm" && parts[1]) {
		return { kind: "dm", dmId: parts[1] };
	}
	if (parts[0] === "channel" && parts[1] && parts[2]) {
		return { kind: "channel", serverId: parts[1], channelId: parts[2] };
	}
	return null;
}

function parseFromUrl(rawUrl: string): DeepLinkTarget | null {
	try {
		const url = new URL(rawUrl, "http://x");
		return parseGoto(url.searchParams.get("goto"));
	} catch {
		return null;
	}
}

function readFromLocation(): DeepLinkTarget | null {
	if (typeof window === "undefined") return null;
	const params = new URLSearchParams(window.location.search);
	const target = parseGoto(params.get("goto"));
	if (target) {
		// Strip ?goto= so a reload doesn't re-trigger it.
		params.delete("goto");
		const qs = params.toString();
		window.history.replaceState(
			{},
			"",
			window.location.pathname + (qs ? `?${qs}` : "") + window.location.hash
		);
	}
	return target;
}

class DeepLinkStore {
	target = $state<DeepLinkTarget | null>(null);
	private wired = false;

	init() {
		if (this.wired) return;
		this.wired = true;
		const fromLocation = readFromLocation();
		if (fromLocation) this.target = fromLocation;

		if (typeof navigator !== "undefined" && "serviceWorker" in navigator) {
			navigator.serviceWorker.addEventListener("message", (event) => {
				const data = event.data as { type?: string; url?: string } | undefined;
				if (data?.type === "hollowchat:goto" && data.url) {
					const parsed = parseFromUrl(data.url);
					if (parsed) this.target = parsed;
				}
			});
		}
	}

	consume(): DeepLinkTarget | null {
		const value = this.target;
		this.target = null;
		return value;
	}
}

export const deepLink = new DeepLinkStore();

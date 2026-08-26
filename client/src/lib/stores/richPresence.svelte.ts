import { listen } from "@tauri-apps/api/event";
import { session } from "$lib/stores/session.svelte";
import { setActivity, type SetActivityBody } from "$lib/api/client";

type PresencePayload = {
	application_id: string | null;
	details: string | null;
	state: string | null;
	large_text: string | null;
	large_image: string | null;
	small_image: string | null;
	small_text: string | null;
	start_timestamp: number | null;
	party_size: number | null;
	party_max: number | null;
};

const KNOWN_APPS: Record<string, string> = {
	"356875570916753438": "Rocket League",
	"379286045294002177": "PUBG",
	"401518684763586560": "Rainbow Six Siege",
	"438122941302046720": "Genshin Impact",
	"356869127241760770": "Fortnite",
	"382112353941667851": "GTA V",
	"493557426159370240": "Apex Legends",
	"365468614890782730": "Minecraft",
	"433027613177298954": "League of Legends",
	"329165918053335041": "osu!",
	"445719970763980815": "Terraria",
	"438122238692147200": "CS2",
	"1043943905241108561": "Valorant"
};

let started = false;

function resolveApplicationName(payload: PresencePayload): string | undefined {
	if (payload.large_text) return payload.large_text;
	if (payload.application_id && KNOWN_APPS[payload.application_id]) {
		return KNOWN_APPS[payload.application_id];
	}
	return payload.application_id ?? undefined;
}

/// Games that speak Discord RPC upload their cover art to Discord's
/// developer portal, which serves it back off this public CDN keyed by
/// their application id - no HollowChat-side asset handling needed.
function resolveImageUrl(payload: PresencePayload): string | undefined {
	if (!payload.application_id || !payload.large_image) return undefined;
	if (payload.large_image.startsWith("mp:")) return undefined;
	return `https://cdn.discordapp.com/app-assets/${payload.application_id}/${payload.large_image}.png`;
}

function resolveSmallImageUrl(payload: PresencePayload): string | undefined {
	if (!payload.application_id || !payload.small_image) return undefined;
	if (payload.small_image.startsWith("mp:")) return undefined;
	return `https://cdn.discordapp.com/app-assets/${payload.application_id}/${payload.small_image}.png`;
}

/// The spec says `timestamps.start` is milliseconds, but plenty of real
/// SET_ACTIVITY senders get this wrong and send seconds instead. A
/// legitimate ms-epoch value for any date after ~2001 is always >= 1e12;
/// anything smaller is almost certainly seconds that needs scaling up, or
/// a start time so far in the past it'd be useless to show anyway.
function normalizeStartMs(value: number): number {
	return value < 1e12 ? value * 1000 : value;
}

function toActivityBody(payload: PresencePayload, kind: "game" | "media"): SetActivityBody {
	return {
		application: resolveApplicationName(payload),
		details: payload.details ?? undefined,
		state: payload.state ?? undefined,
		image: kind === "game" ? resolveImageUrl(payload) : undefined,
		small_image: kind === "game" ? resolveSmallImageUrl(payload) : undefined,
		small_text: kind === "game" ? (payload.small_text ?? undefined) : undefined,
		started_at:
			kind === "game" && payload.start_timestamp
				? new Date(normalizeStartMs(payload.start_timestamp)).toISOString()
				: undefined,
		party_size: kind === "game" ? (payload.party_size ?? undefined) : undefined,
		party_max: kind === "game" ? (payload.party_max ?? undefined) : undefined,
		kind
	};
}

/**
 * Feeds one presence slot (game or media) from its own Tauri event stream,
 * deduping independently so a game activity and a browser media activity
 * never clobber each other - they're separate columns server-side, pushed
 * through the same `/profile/activity` endpoint tagged by `kind`.
 */
function bridgeSlot(event: string, kind: "game" | "media") {
	let lastSent: string | null = null;

	async function push(body: SetActivityBody) {
		const token = session.token;
		if (!token) return;
		const key = JSON.stringify(body);
		if (key === lastSent) return;
		lastSent = key;
		try {
			await setActivity(token, body);
		} catch {
			lastSent = null;
		}
	}

	listen<PresencePayload | null>(event, (e) => {
		push(e.payload ? toActivityBody(e.payload, kind) : { kind });
	}).catch(() => {});
}

/**
 * Wires up the desktop client's local Rich Presence bridges to the
 * profile's activity fields: the Discord-IPC game bridge (`rpc.rs`,
 * `rich-presence` event) and the browser extension's media bridge
 * (`media_bridge.rs`, `media-presence` event). Games that already speak
 * the Discord RPC protocol, and the media extension's supported sites,
 * both just work here without further wiring. No-ops outside the Tauri
 * shell (e.g. running the web build in a plain browser during
 * development).
 */
export function initRichPresenceBridge() {
	if (started) return;
	started = true;
	bridgeSlot("rich-presence", "game");
	bridgeSlot("media-presence", "media");
}

import { listen } from "@tauri-apps/api/event";
import { session } from "$lib/stores/session.svelte";
import { setActivity, type SetActivityBody } from "$lib/api/client";

type PresencePayload = {
	application_id: string | null;
	details: string | null;
	state: string | null;
	large_text: string | null;
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

let lastSent: string | null = null;
let started = false;

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

function resolveApplicationName(payload: PresencePayload): string | undefined {
	if (payload.large_text) return payload.large_text;
	if (payload.application_id && KNOWN_APPS[payload.application_id]) {
		return KNOWN_APPS[payload.application_id];
	}
	return payload.application_id ?? undefined;
}

function toActivityBody(payload: PresencePayload): SetActivityBody {
	return {
		application: resolveApplicationName(payload),
		details: payload.details ?? undefined,
		state: payload.state ?? undefined
	};
}

/**
 * Wires up the desktop client's local Rich Presence bridge (a
 * Discord-IPC-compatible socket run from Rust, see
 * `client/src-tauri/src/rpc.rs`) to the profile's activity fields. Games
 * that already speak the Discord RPC protocol just work here without any
 * changes on their end. No-ops outside the Tauri shell (e.g. running the
 * web build in a plain browser during development).
 */
export function initRichPresenceBridge() {
	if (started) return;
	started = true;
	listen<PresencePayload>("rich-presence", (event) => {
		push(toActivityBody(event.payload));
	}).catch(() => {
		started = false;
	});
}

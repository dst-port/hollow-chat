import { listen } from "@tauri-apps/api/event";
import { session } from "$lib/stores/session.svelte";
import { setActivity, type SetActivityBody } from "$lib/api/client";

type PresencePayload = {
	application_id: string | null;
	details: string | null;
	state: string | null;
	large_text: string | null;
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

function toActivityBody(payload: PresencePayload): SetActivityBody {
	return {
		application: payload.large_text ?? payload.application_id ?? undefined,
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

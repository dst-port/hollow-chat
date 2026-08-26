import { WS_BASE_URL } from "$lib/api/client";

export type PresenceEvent = {
	user_id: string;
	presence: string;
	status_text: string | null;
	activity_application: string | null;
	activity_details: string | null;
	activity_state: string | null;
	activity_image: string | null;
	activity_small_image: string | null;
	activity_small_text: string | null;
	activity_started_at: string | null;
	activity_party_size: number | null;
	activity_party_max: number | null;
	media_application: string | null;
	media_details: string | null;
	media_state: string | null;
};

class PresenceStore {
	byUserId = $state<Record<string, PresenceEvent>>({});

	apply(event: PresenceEvent) {
		this.byUserId = { ...this.byUserId, [event.user_id]: event };
	}

	forUser(userId: string): PresenceEvent | null {
		return this.byUserId[userId] ?? null;
	}
}

export const presenceStore = new PresenceStore();

let socket: WebSocket | null = null;
let started = false;
let reconnectTimer: ReturnType<typeof setTimeout> | null = null;

function connect(token: string) {
	const ws = new WebSocket(`${WS_BASE_URL}/gateway?token=${encodeURIComponent(token)}`);
	socket = ws;

	ws.onmessage = (event) => {
		let data: { type?: string } & Partial<PresenceEvent>;
		try {
			data = JSON.parse(event.data);
		} catch {
			return;
		}
		if (data.type === "presence-update" && data.user_id) {
			presenceStore.apply(data as PresenceEvent);
		}
	};

	ws.onclose = () => {
		if (!started) return;
		reconnectTimer = setTimeout(() => connect(token), 3000);
	};
}

export function initGatewayBridge(token: string) {
	if (started) return;
	started = true;
	connect(token);
}

export function stopGatewayBridge() {
	started = false;
	if (reconnectTimer) clearTimeout(reconnectTimer);
	socket?.close();
	socket = null;
}

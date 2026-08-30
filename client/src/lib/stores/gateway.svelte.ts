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

const TYPING_TTL_MS = 7000;

type TypingEvent = { context: string; id: string; user_id: string; username: string };

class TypingStore {
	// contextId -> username -> expiry epoch ms
	private byContext = $state<Record<string, Record<string, number>>>({});
	// bumped on a timer so `whoIsTyping` re-evaluates expiry reactively
	tick = $state(0);

	apply(event: TypingEvent) {
		const ctx = { ...(this.byContext[event.id] ?? {}) };
		ctx[event.username] = Date.now() + TYPING_TTL_MS;
		this.byContext = { ...this.byContext, [event.id]: ctx };
	}

	whoIsTyping(contextId: string, exclude?: string | null): string[] {
		void this.tick;
		const ctx = this.byContext[contextId];
		if (!ctx) return [];
		const now = Date.now();
		return Object.entries(ctx)
			.filter(([name, exp]) => exp > now && name !== exclude)
			.map(([name]) => name);
	}
}

export const typingStore = new TypingStore();
if (typeof window !== "undefined") {
	setInterval(() => (typingStore.tick = (typingStore.tick + 1) % 1_000_000), 1500);
}

let socket: WebSocket | null = null;
let started = false;
let reconnectTimer: ReturnType<typeof setTimeout> | null = null;
let lastTypingSent = 0;

export function sendTyping(context: "channel" | "dm", id: string) {
	if (!socket || socket.readyState !== WebSocket.OPEN) return;
	const now = Date.now();
	if (now - lastTypingSent < 3000) return;
	lastTypingSent = now;
	try {
		socket.send(JSON.stringify({ type: "typing", context, id }));
	} catch {
		/* socket went away between the check and the send */
	}
}

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
		} else if (data.type === "typing" && (data as unknown as TypingEvent).username) {
			typingStore.apply(data as unknown as TypingEvent);
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

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

// --- account-wide sync -------------------------------------------------
// The server sends { type: "sync", scope } whenever something in a scope
// changed for this user (on any of their devices). We keep the DB as the
// source of truth and just refetch that list. Also replayed for every
// scope right after the socket reconnects, so a client that was offline
// while things changed catches up.

export type SyncScope = "servers" | "friends" | "dms";
const SYNC_SCOPES: SyncScope[] = ["servers", "friends", "dms"];

type SyncHandler = () => void;
const syncHandlers = new Set<{ scope: SyncScope; fn: SyncHandler }>();

/** Register a refetch to run when `scope` changes. Returns an unsubscribe. */
export function onSync(scope: SyncScope, fn: SyncHandler): () => void {
	const entry = { scope, fn };
	syncHandlers.add(entry);
	return () => syncHandlers.delete(entry);
}

function emitSync(scope: string) {
	for (const h of syncHandlers) {
		if (h.scope === scope) {
			try {
				h.fn();
			} catch {
				/* a bad handler shouldn't stop the others */
			}
		}
	}
}

let socket: WebSocket | null = null;
let started = false;
let hasConnectedOnce = false;
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

	ws.onopen = () => {
		// On a *re*connect, everything may have moved while we were away -
		// refetch every scope. The first connect is covered by each view's
		// own initial load, so skip it to avoid a redundant double-fetch.
		if (hasConnectedOnce) {
			for (const scope of SYNC_SCOPES) emitSync(scope);
		}
		hasConnectedOnce = true;
	};

	ws.onmessage = (event) => {
		let data: { type?: string; scope?: string } & Partial<PresenceEvent>;
		try {
			data = JSON.parse(event.data);
		} catch {
			return;
		}
		if (data.type === "presence-update" && data.user_id) {
			presenceStore.apply(data as PresenceEvent);
		} else if (data.type === "typing" && (data as unknown as TypingEvent).username) {
			typingStore.apply(data as unknown as TypingEvent);
		} else if (data.type === "sync" && data.scope) {
			emitSync(data.scope);
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
	hasConnectedOnce = false;
	if (reconnectTimer) clearTimeout(reconnectTimer);
	socket?.close();
	socket = null;
}

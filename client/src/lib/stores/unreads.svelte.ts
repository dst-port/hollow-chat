import * as api from "$lib/api/client";
import { onGatewayEvent } from "./gateway.svelte";

type Context = "channel" | "dm";

/**
 * Unread counts per channel / DM. Server tracks a "last read" timestamp
 * (see read_state); this store hydrates from GET /unreads, then keeps
 * itself current off the same "message-created" gateway pushes ChatView
 * uses - bumping any channel/DM that isn't the one on screen.
 */
class UnreadStore {
	channels = $state<Record<string, number>>({});
	dms = $state<Record<string, number>>({});

	private token: string | null = null;
	private started = false;
	private activeKey: string | null = null;

	init(token: string) {
		this.token = token;
		if (this.started) {
			this.refresh();
			return;
		}
		this.started = true;
		this.refresh();

		onGatewayEvent("message-created", (d) => {
			const ctx = d.context as Context;
			const id = d.channel_id as string;
			if (!id || (ctx !== "channel" && ctx !== "dm")) return;
			if (`${ctx}:${id}` === this.activeKey) return; // you're looking at it
			const bucket = ctx === "channel" ? this.channels : this.dms;
			bucket[id] = (bucket[id] ?? 0) + 1;
		});
		onGatewayEvent("reconnected", () => this.refresh());
	}

	/** Called by ChatView when a conversation is open on screen. */
	setActive(context: Context | null, id: string | null) {
		this.activeKey = context && id ? `${context}:${id}` : null;
		if (context && id) this.markRead(context, id);
	}

	markRead(context: Context, id: string, messageId?: string) {
		const bucket = context === "channel" ? this.channels : this.dms;
		if (bucket[id]) delete bucket[id];
		const token = this.token;
		if (token) api.markRead(token, context, id, messageId).catch(() => {});
	}

	refresh() {
		const token = this.token;
		if (!token) return;
		api
			.getUnreads(token)
			.then((r) => {
				const ch: Record<string, number> = {};
				for (const c of r.channels) ch[c.channel_id] = c.unread;
				const dm: Record<string, number> = {};
				for (const d of r.dms) dm[d.dm_channel_id] = d.unread;
				if (this.activeKey) {
					const [ctx, id] = this.activeKey.split(":");
					if (ctx === "channel") delete ch[id];
					else delete dm[id];
				}
				this.channels = ch;
				this.dms = dm;
			})
			.catch(() => {});
	}

	reset() {
		this.channels = {};
		this.dms = {};
		this.activeKey = null;
		this.started = false;
		this.token = null;
	}

	channelUnread(channelId: string): number {
		return this.channels[channelId] ?? 0;
	}

	dmUnread(dmId: string): number {
		return this.dms[dmId] ?? 0;
	}

	get totalDm(): number {
		let n = 0;
		for (const v of Object.values(this.dms)) n += v;
		return n;
	}
}

export const unreads = new UnreadStore();

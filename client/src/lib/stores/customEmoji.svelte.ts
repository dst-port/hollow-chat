import { listCustomEmoji, resolveUrl, type CustomEmoji } from "$lib/api/client";

class CustomEmojiStore {
	byServerId = $state<Record<string, CustomEmoji[]>>({});
	private loading = new Set<string>();

	load(token: string, serverId: string) {
		if (this.byServerId[serverId] || this.loading.has(serverId)) return;
		this.loading.add(serverId);
		listCustomEmoji(token, serverId)
			.then((emoji) => {
				this.byServerId = { ...this.byServerId, [serverId]: emoji };
			})
			.catch(() => {})
			.finally(() => this.loading.delete(serverId));
	}

	forServer(serverId: string | null): CustomEmoji[] {
		if (!serverId) return [];
		return this.byServerId[serverId] ?? [];
	}

	mapFor(serverId: string | null, token?: string | null): Record<string, string> {
		const map: Record<string, string> = {};
		for (const emoji of this.forServer(serverId)) {
			map[emoji.name.toLowerCase()] = resolveUrl(emoji.image_url, token);
		}
		return map;
	}

	invalidate(serverId: string) {
		const next = { ...this.byServerId };
		delete next[serverId];
		this.byServerId = next;
	}
}

export const customEmojiStore = new CustomEmojiStore();

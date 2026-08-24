import * as api from "$lib/api/client";
import type { ApiBadge } from "$lib/api/client";

class BadgeStore {
	catalog = $state<Record<string, ApiBadge>>({});
	byUsername = $state<Record<string, string[]>>({});

	private catalogLoaded = false;
	private catalogLoading: Promise<void> | null = null;
	private userLoading = new Map<string, Promise<void>>();

	private loadCatalog(token: string): Promise<void> {
		if (this.catalogLoaded) return Promise.resolve();
		if (this.catalogLoading) return this.catalogLoading;

		this.catalogLoading = api
			.badgeCatalog(token)
			.then((badges) => {
				const map: Record<string, ApiBadge> = {};
				for (const badge of badges) map[badge.slug] = badge;
				this.catalog = map;
				this.catalogLoaded = true;
			})
			.catch(() => {})
			.finally(() => {
				this.catalogLoading = null;
			});

		return this.catalogLoading;
	}

	async loadForUser(token: string, username: string): Promise<void> {
		await this.loadCatalog(token);

		const inFlight = this.userLoading.get(username);
		if (inFlight) return inFlight;

		const promise = api
			.userBadges(token, username)
			.then((slugs) => {
				this.byUsername = { ...this.byUsername, [username]: slugs };
			})
			.catch(() => {})
			.finally(() => {
				this.userLoading.delete(username);
			});

		this.userLoading.set(username, promise);
		return promise;
	}

	forUser(username: string): string[] {
		return this.byUsername[username] ?? [];
	}
}

export const badgeStore = new BadgeStore();

import * as api from "$lib/api/client";
import type { ApiProfile } from "$lib/api/client";

class ProfileStore {
	byUsername = $state<Record<string, ApiProfile>>({});

	private loading = new Map<string, Promise<void>>();

	async load(token: string, username: string): Promise<void> {
		const inFlight = this.loading.get(username);
		if (inFlight) return inFlight;

		const promise = api
			.fetchProfile(token, username)
			.then((profile) => {
				this.byUsername = { ...this.byUsername, [username]: profile };
			})
			.catch(() => {})
			.finally(() => {
				this.loading.delete(username);
			});

		this.loading.set(username, promise);
		return promise;
	}

	set(profile: ApiProfile) {
		this.byUsername = { ...this.byUsername, [profile.username]: profile };
	}

	forUser(username: string): ApiProfile | null {
		return this.byUsername[username] ?? null;
	}
}

export const profileStore = new ProfileStore();

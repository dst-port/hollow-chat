import { me } from "$lib/api/client";
import { ensureIdentity } from "$lib/crypto/identity";

const STORAGE_KEY = "hollowchat_session";

type StoredSession = {
	token: string;
	username: string;
};

class SessionStore {
	token = $state<string | null>(null);
	username = $state<string | null>(null);
	userId = $state<string | null>(null);
	ready = $state(false);

	constructor() {
		this.restore();
	}

	async restore() {
		const raw = localStorage.getItem(STORAGE_KEY);
		if (!raw) {
			this.ready = true;
			return;
		}

		try {
			const stored: StoredSession = JSON.parse(raw);
			const info = await me(stored.token);
			this.token = stored.token;
			this.username = stored.username;
			this.userId = info.id;
			ensureIdentity(stored.token, stored.username).catch(() => {});
		} catch {
			localStorage.removeItem(STORAGE_KEY);
		}

		this.ready = true;
	}

	set(token: string, username: string) {
		this.token = token;
		this.username = username;
		localStorage.setItem(STORAGE_KEY, JSON.stringify({ token, username }));
		ensureIdentity(token, username).catch(() => {});
		me(token)
			.then((info) => {
				this.userId = info.id;
			})
			.catch(() => {});
	}

	clear() {
		this.token = null;
		this.username = null;
		this.userId = null;
		localStorage.removeItem(STORAGE_KEY);
	}

	get isAuthenticated() {
		return this.token !== null;
	}
}

export const session = new SessionStore();

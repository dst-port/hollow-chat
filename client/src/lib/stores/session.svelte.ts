import { me } from "$lib/api/client";
import { ensureIdentity, hasLocalIdentity } from "$lib/crypto/identity";

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
	needsDeviceSetup = $state(false);

	constructor() {
		this.restore();
	}

	private bootstrapIdentity(token: string, username: string) {
		if (hasLocalIdentity(username)) {
			ensureIdentity(token, username).catch(() => {});
		} else {
			this.needsDeviceSetup = true;
		}
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
			this.bootstrapIdentity(stored.token, stored.username);
		} catch {
			localStorage.removeItem(STORAGE_KEY);
		}

		this.ready = true;
	}

	set(token: string, username: string, isNewAccount = false) {
		this.token = token;
		this.username = username;
		localStorage.setItem(STORAGE_KEY, JSON.stringify({ token, username }));
		if (isNewAccount) {
			ensureIdentity(token, username).catch(() => {});
		} else {
			this.bootstrapIdentity(token, username);
		}
		me(token)
			.then((info) => {
				this.userId = info.id;
			})
			.catch(() => {});
	}

	completeDeviceSetup() {
		this.needsDeviceSetup = false;
	}

	clear() {
		this.token = null;
		this.username = null;
		this.userId = null;
		this.needsDeviceSetup = false;
		localStorage.removeItem(STORAGE_KEY);
	}

	get isAuthenticated() {
		return this.token !== null;
	}
}

export const session = new SessionStore();

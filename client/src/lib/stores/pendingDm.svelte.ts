class PendingDmStore {
	username = $state<string | null>(null);

	request(username: string) {
		this.username = username;
	}

	consume(): string | null {
		const value = this.username;
		this.username = null;
		return value;
	}
}

export const pendingDm = new PendingDmStore();

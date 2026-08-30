class PendingDmStore {
	username = $state<string | null>(null);
	dmId = $state<string | null>(null);

	request(username: string) {
		this.username = username;
	}

	/** Open a specific DM channel by id (used by notification deep-links). */
	requestId(dmId: string) {
		this.dmId = dmId;
	}

	consume(): string | null {
		const value = this.username;
		this.username = null;
		return value;
	}

	consumeId(): string | null {
		const value = this.dmId;
		this.dmId = null;
		return value;
	}
}

export const pendingDm = new PendingDmStore();

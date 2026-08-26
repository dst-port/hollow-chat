const STORAGE_KEY = "hollowchat.notifications.mentions";

function loadStored(): boolean {
	try {
		const raw = localStorage.getItem(STORAGE_KEY);
		return raw === null ? true : raw === "true";
	} catch {
		return true;
	}
}

class NotificationSettingsStore {
	mentionsEnabled = $state(loadStored());

	setMentionsEnabled(value: boolean) {
		this.mentionsEnabled = value;
		try {
			localStorage.setItem(STORAGE_KEY, String(value));
		} catch {
			// storage unavailable, setting just won't survive a reload
		}
	}
}

export const notificationSettings = new NotificationSettingsStore();

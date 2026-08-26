import type { Update } from "@tauri-apps/plugin-updater";

type UpdateStatus = "idle" | "checking" | "available" | "downloading" | "ready" | "error" | "up-to-date";

class UpdaterStore {
	status = $state<UpdateStatus>("idle");
	version = $state<string | null>(null);
	progress = $state(0);
	error = $state<string | null>(null);

	private update: Update | null = null;

	async check(): Promise<void> {
		if (this.status === "checking" || this.status === "downloading") return;
		this.status = "checking";
		this.error = null;
		try {
			const { check } = await import("@tauri-apps/plugin-updater");
			const update = await check();
			if (!update) {
				this.status = "up-to-date";
				return;
			}
			this.update = update;
			this.version = update.version;
			this.status = "available";
		} catch (err) {
			this.status = "error";
			this.error = err instanceof Error ? err.message : "Update check failed";
		}
	}

	async downloadAndInstall(): Promise<void> {
		if (!this.update || this.status !== "available") return;
		this.status = "downloading";
		this.progress = 0;
		let total = 0;
		let downloaded = 0;
		try {
			await this.update.downloadAndInstall((event: { event: string; data?: { contentLength?: number; chunkLength?: number } }) => {
				if (event.event === "Started") {
					total = event.data?.contentLength ?? 0;
				} else if (event.event === "Progress") {
					downloaded += event.data?.chunkLength ?? 0;
					this.progress = total > 0 ? Math.min(1, downloaded / total) : 0;
				} else if (event.event === "Finished") {
					this.progress = 1;
				}
			});
			this.status = "ready";
		} catch (err) {
			this.status = "error";
			this.error = err instanceof Error ? err.message : "Update failed to download";
		}
	}

	async restart(): Promise<void> {
		const { relaunch } = await import("@tauri-apps/plugin-process");
		await relaunch();
	}

	dismiss(): void {
		if (this.status === "available" || this.status === "up-to-date" || this.status === "error") {
			this.status = "idle";
		}
	}
}

export const updater = new UpdaterStore();

let started = false;

export function initAutoUpdateCheck(): void {
	if (started) return;
	started = true;
	updater.check();
	setInterval(() => updater.check(), 4 * 60 * 60 * 1000);
}

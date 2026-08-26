let permissionChecked = false;
let permissionGranted = false;

async function ensurePermission(): Promise<boolean> {
	if (permissionChecked) return permissionGranted;
	permissionChecked = true;
	try {
		const { isPermissionGranted, requestPermission } = await import("@tauri-apps/plugin-notification");
		permissionGranted = await isPermissionGranted();
		if (!permissionGranted) {
			const result = await requestPermission();
			permissionGranted = result === "granted";
		}
	} catch {
		permissionGranted = false;
	}
	return permissionGranted;
}

/**
 * OS-level desktop notification - only fires while the window is unfocused,
 * so it doesn't pile up on top of a toast you're already looking at. No-op
 * outside the Tauri shell (e.g. a plain browser during development).
 */
export async function notifyDesktop(title: string, body: string): Promise<void> {
	try {
		const { getCurrentWindow } = await import("@tauri-apps/api/window");
		const focused = await getCurrentWindow().isFocused();
		if (focused) return;
	} catch {
		return;
	}

	const granted = await ensurePermission();
	if (!granted) return;

	try {
		const { sendNotification } = await import("@tauri-apps/plugin-notification");
		sendNotification({ title, body });
	} catch {
		// notification plugin unavailable - nothing to do
	}
}

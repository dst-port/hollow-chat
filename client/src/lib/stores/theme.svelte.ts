const STORAGE_KEY = "hollowchat.theme";

export type ThemeMode = "default" | "custom";

export type ThemeColorKey =
	| "void"
	| "rail"
	| "sidebar"
	| "panel"
	| "hover"
	| "active"
	| "ink"
	| "ink-dim"
	| "ink-faint"
	| "accent-fill"
	| "accent-fill-ink"
	| "online"
	| "idle"
	| "danger";

export const DEFAULT_COLORS: Record<ThemeColorKey, string> = {
	void: "#100e0d",
	rail: "#171412",
	sidebar: "#1d1917",
	panel: "#241f1c",
	hover: "#362e29",
	active: "#4a3f37",
	ink: "#f3efe9",
	"ink-dim": "#ada39b",
	"ink-faint": "#756b63",
	"accent-fill": "#e8ded2",
	"accent-fill-ink": "#1c1815",
	online: "#3ba55d",
	idle: "#d3a53a",
	danger: "#e0524f"
};

export const COLOR_GROUPS: { label: string; keys: ThemeColorKey[] }[] = [
	{ label: "Backgrounds", keys: ["void", "rail", "sidebar", "panel", "hover", "active"] },
	{ label: "Text", keys: ["ink", "ink-dim", "ink-faint"] },
	{ label: "Accent", keys: ["accent-fill", "accent-fill-ink"] },
	{ label: "Status", keys: ["online", "idle", "danger"] }
];

export const COLOR_LABELS: Record<ThemeColorKey, string> = {
	void: "Window Background",
	rail: "Server Rail",
	sidebar: "Channel Sidebar",
	panel: "Cards & Modals",
	hover: "Hover State",
	active: "Active State",
	ink: "Primary Text",
	"ink-dim": "Secondary Text",
	"ink-faint": "Faint Text",
	"accent-fill": "Accent",
	"accent-fill-ink": "Text on Accent",
	online: "Online Status",
	idle: "Idle Status",
	danger: "Danger / Errors"
};

export type ThemeSettings = {
	mode: ThemeMode;
	colors: Record<ThemeColorKey, string>;
};

function defaultSettings(): ThemeSettings {
	return { mode: "default", colors: { ...DEFAULT_COLORS } };
}

function loadStored(): ThemeSettings {
	try {
		const raw = localStorage.getItem(STORAGE_KEY);
		if (!raw) return defaultSettings();
		const parsed = JSON.parse(raw);
		const colors = { ...DEFAULT_COLORS };
		if (parsed?.colors && typeof parsed.colors === "object") {
			for (const key of Object.keys(DEFAULT_COLORS) as ThemeColorKey[]) {
				if (typeof parsed.colors[key] === "string") colors[key] = parsed.colors[key];
			}
		}
		return {
			mode: parsed?.mode === "custom" ? "custom" : "default",
			colors
		};
	} catch {
		return defaultSettings();
	}
}

class ThemeStore {
	settings = $state<ThemeSettings>(defaultSettings());

	init() {
		this.settings = loadStored();
		this.apply();
	}

	apply() {
		if (typeof document === "undefined") return;
		const root = document.documentElement;
		for (const key of Object.keys(DEFAULT_COLORS) as ThemeColorKey[]) {
			if (this.settings.mode === "custom") {
				root.style.setProperty(`--${key}`, this.settings.colors[key]);
			} else {
				root.style.removeProperty(`--${key}`);
			}
		}
	}

	setMode(mode: ThemeMode) {
		this.settings = { ...this.settings, mode };
		this.persist();
		this.apply();
	}

	setColor(key: ThemeColorKey, hex: string) {
		this.settings = { ...this.settings, colors: { ...this.settings.colors, [key]: hex } };
		this.persist();
		this.apply();
	}

	resetColors() {
		this.settings = { ...this.settings, colors: { ...DEFAULT_COLORS } };
		this.persist();
		this.apply();
	}

	private persist() {
		try {
			localStorage.setItem(STORAGE_KEY, JSON.stringify(this.settings));
		} catch {
			// storage unavailable, custom theme just won't survive a reload
		}
	}
}

export const themeStore = new ThemeStore();

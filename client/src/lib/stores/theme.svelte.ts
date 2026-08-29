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

export type ThemePreset = {
	id: string;
	label: string;
	colors: Record<ThemeColorKey, string>;
};

export const THEME_PRESETS: ThemePreset[] = [
	{
		id: "midnight",
		label: "Midnight Blue",
		colors: {
			void: "#0a0e14",
			rail: "#0d1219",
			sidebar: "#111826",
			panel: "#161f30",
			hover: "#20304a",
			active: "#2b3f60",
			ink: "#eaf0fb",
			"ink-dim": "#a3b1cc",
			"ink-faint": "#6b7793",
			"accent-fill": "#5b8cff",
			"accent-fill-ink": "#08111f",
			online: "#3ba55d",
			idle: "#d3a53a",
			danger: "#e0524f"
		}
	},
	{
		id: "forest",
		label: "Forest",
		colors: {
			void: "#0c1410",
			rail: "#0f1813",
			sidebar: "#141f18",
			panel: "#1a281f",
			hover: "#26392b",
			active: "#324c3a",
			ink: "#eaf3ea",
			"ink-dim": "#a4bba9",
			"ink-faint": "#6d8574",
			"accent-fill": "#5fb87b",
			"accent-fill-ink": "#0b1a10",
			online: "#3ba55d",
			idle: "#d3a53a",
			danger: "#e0524f"
		}
	},
	{
		id: "crimson",
		label: "Crimson",
		colors: {
			void: "#150c0d",
			rail: "#190f10",
			sidebar: "#201314",
			panel: "#28181a",
			hover: "#3d2325",
			active: "#552e31",
			ink: "#f5eaea",
			"ink-dim": "#c2a3a5",
			"ink-faint": "#8c6a6c",
			"accent-fill": "#e5555f",
			"accent-fill-ink": "#1c0b0c",
			online: "#3ba55d",
			idle: "#d3a53a",
			danger: "#ff6b6b"
		}
	},
	{
		id: "lavender",
		label: "Lavender",
		colors: {
			void: "#100e17",
			rail: "#14111c",
			sidebar: "#1a1624",
			panel: "#211c2e",
			hover: "#302947",
			active: "#40385f",
			ink: "#f0ecfa",
			"ink-dim": "#b6abd1",
			"ink-faint": "#7d739a",
			"accent-fill": "#a184ff",
			"accent-fill-ink": "#150f24",
			online: "#3ba55d",
			idle: "#d3a53a",
			danger: "#e0524f"
		}
	},
	{
		id: "paper",
		label: "Paper Light",
		colors: {
			void: "#f3efe6",
			rail: "#ece6d9",
			sidebar: "#e7e0d1",
			panel: "#ffffff",
			hover: "#e0d7c4",
			active: "#d3c7ac",
			ink: "#231f1a",
			"ink-dim": "#544c3f",
			"ink-faint": "#877b66",
			"accent-fill": "#b5652f",
			"accent-fill-ink": "#fdf6ec",
			online: "#2f8f4f",
			idle: "#b8862c",
			danger: "#c94441"
		}
	}
];

export type ThemeSettings = {
	mode: ThemeMode;
	colors: Record<ThemeColorKey, string>;
	presetId: string | null;
};

function defaultSettings(): ThemeSettings {
	return { mode: "default", colors: { ...DEFAULT_COLORS }, presetId: null };
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
			colors,
			presetId: typeof parsed?.presetId === "string" ? parsed.presetId : null
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
		this.settings = {
			...this.settings,
			colors: { ...this.settings.colors, [key]: hex },
			presetId: null
		};
		this.persist();
		this.apply();
	}

	applyPreset(id: string) {
		const preset = THEME_PRESETS.find((p) => p.id === id);
		if (!preset) return;
		this.settings = { mode: "custom", colors: { ...preset.colors }, presetId: preset.id };
		this.persist();
		this.apply();
	}

	resetColors() {
		this.settings = { ...this.settings, colors: { ...DEFAULT_COLORS }, presetId: null };
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

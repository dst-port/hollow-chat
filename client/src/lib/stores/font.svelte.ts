const STORAGE_KEY = "hollowchat.font";
const CUSTOM_LINK_EL_ID = "hollowchat-custom-font-link";

export type FontId = "default" | "inter" | "poppins" | "comfortaa" | "jetbrains-mono" | "merriweather";
export type FontMode = "default" | "preset" | "link";

export const FONT_STACKS: Record<FontId, string> = {
	default: "-apple-system, BlinkMacSystemFont, \"Segoe UI\", Roboto, Helvetica, Arial, sans-serif",
	inter: "\"Inter\", -apple-system, BlinkMacSystemFont, sans-serif",
	poppins: "\"Poppins\", -apple-system, BlinkMacSystemFont, sans-serif",
	comfortaa: "\"Comfortaa\", -apple-system, BlinkMacSystemFont, sans-serif",
	"jetbrains-mono": "\"JetBrains Mono\", ui-monospace, monospace",
	merriweather: "\"Merriweather\", Georgia, serif"
};

export const FONT_LABELS: Record<FontId, string> = {
	default: "Default",
	inter: "Inter",
	poppins: "Poppins",
	comfortaa: "Comfortaa (Rounded)",
	"jetbrains-mono": "JetBrains Mono",
	merriweather: "Merriweather (Serif)"
};

export const PRESET_FONT_IDS: FontId[] = ["inter", "poppins", "comfortaa", "jetbrains-mono", "merriweather"];

/**
 * Distinctive display faces offered only for the nameplate (not the UI
 * font) — the point is a name that stands out, not another sans-serif.
 */
export type NameFontId =
	| "cinzel"
	| "orbitron"
	| "unifraktur"
	| "pacifico"
	| "silkscreen"
	| "monoton"
	| "rubik-mono"
	| "caveat";

export const NAME_FONT_STACKS: Record<NameFontId, string> = {
	cinzel: '"Cinzel", Georgia, serif',
	orbitron: '"Orbitron", "Segoe UI", sans-serif',
	unifraktur: '"UnifrakturMaguntia", Georgia, serif',
	pacifico: '"Pacifico", cursive',
	silkscreen: '"Silkscreen", "Courier New", monospace',
	monoton: '"Monoton", cursive',
	"rubik-mono": '"Rubik Mono One", "Arial Black", sans-serif',
	caveat: '"Caveat", cursive'
};

export const NAME_FONT_LABELS: Record<NameFontId, string> = {
	cinzel: "Cinzel",
	orbitron: "Orbitron",
	unifraktur: "Gothic",
	pacifico: "Pacifico",
	silkscreen: "Pixel",
	monoton: "Monoton",
	"rubik-mono": "Heavy",
	caveat: "Handwritten"
};

export const NAME_FONT_IDS: NameFontId[] = [
	"cinzel",
	"orbitron",
	"unifraktur",
	"pacifico",
	"silkscreen",
	"monoton",
	"rubik-mono",
	"caveat"
];

/**
 * Font stack for a profile's chosen nameplate font, or undefined when unset
 * / invalid (falls back to the surrounding element's font). Safe to drop
 * straight into `style:font-family` on any display-name element.
 */
export function nameFontStack(font: string | null | undefined): string | undefined {
	if (!font || font === "default") return undefined;
	return NAME_FONT_STACKS[font as NameFontId];
}

type FontSettings = {
	mode: FontMode;
	presetId: FontId;
	customUrl: string;
	customFamily: string;
};

function defaultSettings(): FontSettings {
	return { mode: "default", presetId: "inter", customUrl: "", customFamily: "" };
}

function loadStored(): FontSettings {
	try {
		const raw = localStorage.getItem(STORAGE_KEY);
		if (!raw) return defaultSettings();

		// Pre-existing installs stored a bare FontId string.
		if (raw in FONT_STACKS) {
			return raw === "default"
				? defaultSettings()
				: { mode: "preset", presetId: raw as FontId, customUrl: "", customFamily: "" };
		}

		const parsed = JSON.parse(raw);
		const settings = defaultSettings();
		if (parsed?.mode === "preset" || parsed?.mode === "link") settings.mode = parsed.mode;
		if (typeof parsed?.presetId === "string" && parsed.presetId in FONT_STACKS) {
			settings.presetId = parsed.presetId;
		}
		if (typeof parsed?.customUrl === "string") settings.customUrl = parsed.customUrl;
		if (typeof parsed?.customFamily === "string") settings.customFamily = parsed.customFamily;
		return settings;
	} catch {
		return defaultSettings();
	}
}

class FontStore {
	settings = $state<FontSettings>(defaultSettings());

	get current(): FontMode {
		return this.settings.mode;
	}

	init() {
		this.settings = loadStored();
		this.apply();
	}

	apply() {
		if (typeof document === "undefined") return;
		const root = document.documentElement;

		if (this.settings.mode === "default") {
			root.style.removeProperty("--font-body");
			this.removeCustomLink();
		} else if (this.settings.mode === "preset") {
			root.style.setProperty("--font-body", FONT_STACKS[this.settings.presetId]);
			this.removeCustomLink();
		} else {
			this.ensureCustomLink(this.settings.customUrl);
			const family = this.settings.customFamily.trim();
			root.style.setProperty(
				"--font-body",
				family ? `"${family}", -apple-system, BlinkMacSystemFont, sans-serif` : ""
			);
		}
	}

	private ensureCustomLink(url: string) {
		const existing = document.getElementById(CUSTOM_LINK_EL_ID) as HTMLLinkElement | null;
		if (!url.trim()) {
			existing?.remove();
			return;
		}
		if (existing) {
			if (existing.href !== url) existing.href = url;
			return;
		}
		const link = document.createElement("link");
		link.id = CUSTOM_LINK_EL_ID;
		link.rel = "stylesheet";
		link.href = url;
		document.head.appendChild(link);
	}

	private removeCustomLink() {
		document.getElementById(CUSTOM_LINK_EL_ID)?.remove();
	}

	setMode(mode: FontMode) {
		this.settings = { ...this.settings, mode };
		this.persist();
		this.apply();
	}

	setPreset(id: FontId) {
		this.settings = { ...this.settings, mode: "preset", presetId: id };
		this.persist();
		this.apply();
	}

	setCustom(family: string, url: string) {
		this.settings = { ...this.settings, mode: "link", customFamily: family, customUrl: url };
		this.persist();
		this.apply();
	}

	private persist() {
		try {
			localStorage.setItem(STORAGE_KEY, JSON.stringify(this.settings));
		} catch {
			// storage unavailable, choice just won't survive a reload
		}
	}
}

export const fontStore = new FontStore();

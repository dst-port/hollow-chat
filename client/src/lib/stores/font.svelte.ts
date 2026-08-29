const STORAGE_KEY = "hollowchat.font";

export type FontId = "default" | "inter" | "poppins" | "comfortaa" | "jetbrains-mono" | "merriweather";

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

function loadStored(): FontId {
	try {
		const raw = localStorage.getItem(STORAGE_KEY);
		return raw && raw in FONT_STACKS ? (raw as FontId) : "default";
	} catch {
		return "default";
	}
}

class FontStore {
	current = $state<FontId>("default");

	init() {
		this.current = loadStored();
		this.apply();
	}

	apply() {
		if (typeof document === "undefined") return;
		const root = document.documentElement;
		if (this.current === "default") {
			root.style.removeProperty("--font-body");
		} else {
			root.style.setProperty("--font-body", FONT_STACKS[this.current]);
		}
	}

	set(id: FontId) {
		this.current = id;
		try {
			localStorage.setItem(STORAGE_KEY, id);
		} catch {
			// storage unavailable, choice just won't survive a reload
		}
		this.apply();
	}
}

export const fontStore = new FontStore();

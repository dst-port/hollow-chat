import bg from "./locales/bg.json";
import cs from "./locales/cs.json";
import da from "./locales/da.json";
import de from "./locales/de.json";
import el from "./locales/el.json";
import en from "./locales/en.json";
import enGB from "./locales/en-GB.json";
import es from "./locales/es.json";
import es419 from "./locales/es-419.json";
import fi from "./locales/fi.json";
import fr from "./locales/fr.json";
import hi from "./locales/hi.json";
import hr from "./locales/hr.json";
import hu from "./locales/hu.json";
import id from "./locales/id.json";
import it from "./locales/it.json";
import ja from "./locales/ja.json";
import ko from "./locales/ko.json";
import lt from "./locales/lt.json";
import nl from "./locales/nl.json";
import no from "./locales/no.json";
import pl from "./locales/pl.json";
import ptBR from "./locales/pt-BR.json";
import ro from "./locales/ro.json";
import ru from "./locales/ru.json";
import sv from "./locales/sv-SE.json";
import th from "./locales/th.json";
import tr from "./locales/tr.json";
import uk from "./locales/uk.json";
import vi from "./locales/vi.json";
import zhCN from "./locales/zh-CN.json";
import zhTW from "./locales/zh-TW.json";

const STORAGE_KEY = "hollowchat.lang";

export type LocaleCode =
	| "en"
	| "en-GB"
	| "bg"
	| "cs"
	| "da"
	| "de"
	| "el"
	| "es"
	| "es-419"
	| "fi"
	| "fr"
	| "hi"
	| "hr"
	| "hu"
	| "id"
	| "it"
	| "ja"
	| "ko"
	| "lt"
	| "nl"
	| "no"
	| "pl"
	| "pt-BR"
	| "ro"
	| "ru"
	| "sv-SE"
	| "th"
	| "tr"
	| "uk"
	| "vi"
	| "zh-CN"
	| "zh-TW";

/** Shown in the language picker, ordered by endonym. */
export const LOCALES: { code: LocaleCode; label: string; english: string }[] = [
	{ code: "en", label: "English", english: "English" },
	{ code: "en-GB", label: "English (UK)", english: "English (UK)" },
	{ code: "bg", label: "Български", english: "Bulgarian" },
	{ code: "cs", label: "Čeština", english: "Czech" },
	{ code: "da", label: "Dansk", english: "Danish" },
	{ code: "de", label: "Deutsch", english: "German" },
	{ code: "el", label: "Ελληνικά", english: "Greek" },
	{ code: "es", label: "Español", english: "Spanish" },
	{ code: "es-419", label: "Español (Latinoamérica)", english: "Spanish (Latin America)" },
	{ code: "fi", label: "Suomi", english: "Finnish" },
	{ code: "fr", label: "Français", english: "French" },
	{ code: "hi", label: "हिन्दी", english: "Hindi" },
	{ code: "hr", label: "Hrvatski", english: "Croatian" },
	{ code: "hu", label: "Magyar", english: "Hungarian" },
	{ code: "id", label: "Bahasa Indonesia", english: "Indonesian" },
	{ code: "it", label: "Italiano", english: "Italian" },
	{ code: "ja", label: "日本語", english: "Japanese" },
	{ code: "ko", label: "한국어", english: "Korean" },
	{ code: "lt", label: "Lietuvių", english: "Lithuanian" },
	{ code: "nl", label: "Nederlands", english: "Dutch" },
	{ code: "no", label: "Norsk", english: "Norwegian" },
	{ code: "pl", label: "Polski", english: "Polish" },
	{ code: "pt-BR", label: "Português do Brasil", english: "Portuguese (Brazil)" },
	{ code: "ro", label: "Română", english: "Romanian" },
	{ code: "ru", label: "Русский", english: "Russian" },
	{ code: "sv-SE", label: "Svenska", english: "Swedish" },
	{ code: "th", label: "ไทย", english: "Thai" },
	{ code: "tr", label: "Türkçe", english: "Turkish" },
	{ code: "uk", label: "Українська", english: "Ukrainian" },
	{ code: "vi", label: "Tiếng Việt", english: "Vietnamese" },
	{ code: "zh-CN", label: "简体中文", english: "Chinese (Simplified)" },
	{ code: "zh-TW", label: "繁體中文", english: "Chinese (Traditional)" }
];

type Dict = Record<string, string>;

const DICTS: Record<LocaleCode, Dict> = {
	en: en as Dict,
	"en-GB": enGB as Dict,
	bg: bg as Dict,
	cs: cs as Dict,
	da: da as Dict,
	de: de as Dict,
	el: el as Dict,
	es: es as Dict,
	"es-419": es419 as Dict,
	fi: fi as Dict,
	fr: fr as Dict,
	hi: hi as Dict,
	hr: hr as Dict,
	hu: hu as Dict,
	id: id as Dict,
	it: it as Dict,
	ja: ja as Dict,
	ko: ko as Dict,
	lt: lt as Dict,
	nl: nl as Dict,
	no: no as Dict,
	pl: pl as Dict,
	"pt-BR": ptBR as Dict,
	ro: ro as Dict,
	ru: ru as Dict,
	"sv-SE": sv as Dict,
	th: th as Dict,
	tr: tr as Dict,
	uk: uk as Dict,
	vi: vi as Dict,
	"zh-CN": zhCN as Dict,
	"zh-TW": zhTW as Dict
};

const EN = DICTS.en;

function detect(): LocaleCode {
	try {
		const saved = localStorage.getItem(STORAGE_KEY);
		if (saved && saved in DICTS) return saved as LocaleCode;
	} catch {
		// storage unavailable — fall through to browser detection
	}
	if (typeof navigator !== "undefined") {
		const candidates = navigator.languages?.length ? navigator.languages : [navigator.language];
		for (const raw of candidates) {
			if (!raw) continue;
			const lc = raw.toLowerCase();
			// exact region match first (pt-br, zh-cn, zh-tw, es-419, en-gb)
			const exact = LOCALES.find((l) => l.code.toLowerCase() === lc);
			if (exact) return exact.code;
			if (lc === "pt" || lc.startsWith("pt-")) return "pt-BR";
			if (lc.startsWith("zh-hant") || lc === "zh-tw" || lc === "zh-hk" || lc === "zh-mo") return "zh-TW";
			if (lc.startsWith("zh")) return "zh-CN";
			if (lc === "es" || lc.startsWith("es-")) return lc === "es-es" ? "es" : "es-419";
			if (lc === "nb" || lc === "nn" || lc.startsWith("no")) return "no";
			const base = lc.split("-")[0];
			const hit = LOCALES.find((l) => l.code.toLowerCase().split("-")[0] === base);
			if (hit) return hit.code;
		}
	}
	return "en";
}

type PluralCategory = "one" | "few" | "many" | "other";

/**
 * CLDR cardinal plural category, integer-count fast path (v=0, f=0).
 * Covers every locale we ship; anything unlisted collapses to one/other.
 */
function pluralCategory(locale: LocaleCode, n: number): PluralCategory {
	const i = Math.abs(Math.trunc(n));
	const mod10 = i % 10;
	const mod100 = i % 100;

	switch (locale) {
		// No grammatical plural — a single form.
		case "ja":
		case "ko":
		case "th":
		case "vi":
		case "id":
		case "zh-CN":
		case "zh-TW":
			return "other";

		// East-Slavic: one / few / many.
		case "ru":
		case "uk":
			if (mod10 === 1 && mod100 !== 11) return "one";
			if (mod10 >= 2 && mod10 <= 4 && (mod100 < 12 || mod100 > 14)) return "few";
			return "many";

		// Polish: one / few / many.
		case "pl":
			if (i === 1) return "one";
			if (mod10 >= 2 && mod10 <= 4 && (mod100 < 12 || mod100 > 14)) return "few";
			return "many";

		// Czech: one / few / other (integers).
		case "cs":
			if (i === 1) return "one";
			if (i >= 2 && i <= 4) return "few";
			return "other";

		// Croatian: one / few / other.
		case "hr":
			if (mod10 === 1 && mod100 !== 11) return "one";
			if (mod10 >= 2 && mod10 <= 4 && (mod100 < 12 || mod100 > 14)) return "few";
			return "other";

		// Lithuanian: one / few / other.
		case "lt":
			if (mod10 === 1 && (mod100 < 11 || mod100 > 19)) return "one";
			if (mod10 >= 2 && mod10 <= 9 && (mod100 < 11 || mod100 > 19)) return "few";
			return "other";

		// Romanian: one / few / other.
		case "ro":
			if (i === 1) return "one";
			if (i === 0 || (i !== 1 && mod100 >= 1 && mod100 <= 19)) return "few";
			return "other";

		// French, Brazilian Portuguese, Hindi: 0 and 1 are "one".
		case "fr":
		case "pt-BR":
		case "hi":
			return i === 0 || i === 1 ? "one" : "other";

		// Everything else: one / other.
		default:
			return i === 1 ? "one" : "other";
	}
}

function interpolate(str: string, vars?: Record<string, string | number>): string {
	if (!vars) return str;
	return str.replace(/\{(\w+)\}/g, (match, key) => (key in vars ? String(vars[key]) : match));
}

class I18nStore {
	lang = $state<LocaleCode>("en");

	init() {
		this.lang = detect();
		this.applyDocLang();
	}

	set(code: LocaleCode) {
		if (!(code in DICTS)) return;
		this.lang = code;
		try {
			localStorage.setItem(STORAGE_KEY, code);
		} catch {
			// choice just won't survive a reload
		}
		this.applyDocLang();
	}

	private applyDocLang() {
		if (typeof document !== "undefined") document.documentElement.lang = this.lang;
	}
}

export const i18n = new I18nStore();

/**
 * Translate `key` for the active locale, falling back to English, then to the
 * key itself. `{placeholder}` tokens are filled from `vars`.
 */
export function t(key: string, vars?: Record<string, string | number>): string {
	const dict = DICTS[i18n.lang] ?? EN;
	const str = dict[key] ?? EN[key] ?? key;
	return interpolate(str, vars);
}

/**
 * Plural-aware translate. Looks up `${key}.${category}` (e.g. `members.count.one`),
 * falling back through `other`, English, and the bare key. `count` is available
 * to the string as `{count}` plus anything else in `vars`.
 */
export function tp(key: string, count: number, vars?: Record<string, string | number>): string {
	const lang = i18n.lang;
	const dict = DICTS[lang] ?? EN;
	const cat = pluralCategory(lang, count);
	const str =
		dict[`${key}.${cat}`] ??
		dict[`${key}.other`] ??
		EN[`${key}.${pluralCategory("en", count)}`] ??
		EN[`${key}.other`] ??
		key;
	return interpolate(str, { count, ...vars });
}

import { describe, it, expect } from "vitest";
import en from "./locales/en.json";
import enGB from "./locales/en-GB.json";
import bg from "./locales/bg.json";
import cs from "./locales/cs.json";
import da from "./locales/da.json";
import de from "./locales/de.json";
import el from "./locales/el.json";
import es from "./locales/es.json";
import es419 from "./locales/es-419.json";
import fi from "./locales/fi.json";
import fr from "./locales/fr.json";
import hi from "./locales/hi.json";
import hr from "./locales/hr.json";
import hu from "./locales/hu.json";
import id from "./locales/id.json";
import itLocale from "./locales/it.json";
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

const LOCALES: Record<string, Record<string, string>> = {
	"en-GB": enGB, bg, cs, da, de, el, es, "es-419": es419, fi, fr, hi, hr, hu, id, it: itLocale,
	ja, ko, lt, nl, no, pl, "pt-BR": ptBR, ro, ru, "sv-SE": sv, th, tr, uk, vi,
	"zh-CN": zhCN, "zh-TW": zhTW
};

const PLURAL_SUFFIX = /\.(one|few|many|other)$/;

/** CLDR plural categories each locale must supply for a plural base key. */
const THREE_FORM_SLAVIC = ["one", "few", "many", "other"]; // ru, uk, pl
const FEW_OTHER = ["one", "few", "other"]; // cs, hr, lt, ro
const ONE_OTHER = ["one", "other"]; // everything else

const REQUIRED_CATEGORIES: Record<string, string[]> = {
	ru: THREE_FORM_SLAVIC,
	uk: THREE_FORM_SLAVIC,
	pl: THREE_FORM_SLAVIC,
	cs: FEW_OTHER,
	hr: FEW_OTHER,
	lt: FEW_OTHER,
	ro: FEW_OTHER
};

const enKeys = Object.keys(en);
const enSimpleKeys = enKeys.filter((k) => !PLURAL_SUFFIX.test(k));
const enPluralBases = [
	...new Set(enKeys.filter((k) => PLURAL_SUFFIX.test(k)).map((k) => k.replace(PLURAL_SUFFIX, "")))
];

describe("locale files", () => {
	for (const [code, dict] of Object.entries(LOCALES)) {
		describe(code, () => {
			it("has every simple key from en", () => {
				const missing = enSimpleKeys.filter((k) => !(k in dict));
				expect(missing, `missing keys in ${code}`).toEqual([]);
			});

			it("has the required plural categories for every plural base", () => {
				const cats = REQUIRED_CATEGORIES[code] ?? ONE_OTHER;
				const missing: string[] = [];
				for (const base of enPluralBases) {
					for (const cat of cats) {
						if (!(`${base}.${cat}` in dict)) missing.push(`${base}.${cat}`);
					}
				}
				expect(missing, `missing plural forms in ${code}`).toEqual([]);
			});

			it("has no keys that don't exist in en", () => {
				const enKeySet = new Set(enKeys);
				const extra = Object.keys(dict).filter(
					(k) => !enKeySet.has(k) && !PLURAL_SUFFIX.test(k)
				);
				expect(extra, `unexpected keys in ${code}`).toEqual([]);
			});

			it("has no empty translations", () => {
				const empty = Object.entries(dict)
					.filter(([, v]) => typeof v !== "string" || v.trim() === "")
					.map(([k]) => k);
				expect(empty, `empty values in ${code}`).toEqual([]);
			});

			it("preserves {placeholder} tokens from en", () => {
				const tokensOf = (s: string) => (s.match(/\{\w+\}/g) ?? []).sort();
				const mismatched: string[] = [];
				for (const key of enSimpleKeys) {
					const enTokens = tokensOf(en[key as keyof typeof en] as string);
					if (enTokens.length === 0) continue;
					const localVal = dict[key];
					if (typeof localVal !== "string") continue;
					if (JSON.stringify(tokensOf(localVal)) !== JSON.stringify(enTokens)) {
						mismatched.push(key);
					}
				}
				expect(mismatched, `placeholder mismatch in ${code}`).toEqual([]);
			});
		});
	}
});

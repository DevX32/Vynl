import { de } from "@lib/locales/de";
import { en } from "@lib/locales/en";
import { es } from "@lib/locales/es";

export type Locale = "en" | "es" | "de";

export const LOCALES: ReadonlyArray<{ id: Locale; label: string }> = [
  { id: "en", label: "English" },
  { id: "es", label: "Español" },
  { id: "de", label: "Deutsch" },
];

type Messages = typeof en;

const locales: Record<Locale, Messages> = { en, es, de };

function isLocale(value: string | null): value is Locale {
  return value === "en" || value === "es" || value === "de";
}

function detectLocale(): Locale {
  if (typeof localStorage !== "undefined") {
    const stored = localStorage.getItem("vynl.locale");
    if (isLocale(stored)) return stored;
  }
  if (typeof navigator !== "undefined") {
    for (const language of navigator.languages ?? [navigator.language]) {
      const base = language.toLowerCase().split("-")[0];
      if (isLocale(base)) return base;
    }
  }
  return "en";
}

const initialLocale = detectLocale();
let _locale = $state<Locale>(initialLocale);

if (typeof document !== "undefined") {
  document.documentElement.lang = initialLocale;
}

export function getLocale(): Locale {
  return _locale;
}

export function setLocale(locale: Locale): void {
  _locale = locale;
  if (typeof document !== "undefined") document.documentElement.lang = locale;
  if (typeof localStorage !== "undefined") {
    localStorage.setItem("vynl.locale", locale);
  }
  if (typeof document !== "undefined") {
    document.documentElement.lang = locale;
  }
}

function get(obj: unknown, path: string): unknown {
  let cur: unknown = obj;
  for (const seg of path.split(".")) {
    if (cur == null || typeof cur !== "object") return undefined;
    cur = (cur as Record<string, unknown>)[seg];
  }
  return cur;
}

function interpolate(
  value: string,
  params?: Record<string, string | number>,
): string {
  if (!params) return value;
  return value.replace(/\{(\w+)\}/g, (_, key: string) =>
    key in params ? String(params[key]) : `{${key}}`,
  );
}

export function t(
  key: string,
  params?: Record<string, string | number>,
): string {
  const val = get(locales[_locale], key) ?? get(locales.en, key) ?? key;
  if (typeof val !== "string") return key;
  return interpolate(val, params);
}

const pluralRules: Partial<Record<Locale, Intl.PluralRules>> = {};

function pluralCategory(locale: Locale, count: number): Intl.LDMLPluralRule {
  (pluralRules[locale] ??= new Intl.PluralRules(locale));
  return pluralRules[locale].select(count);
}

export function tp(
  key: string,
  count: number,
  params?: Record<string, string | number>,
): string {
  const category = pluralCategory(_locale, count);
  const merged = { count, ...params };
  const preferred = `${key}_${category}`;
  const fallback = `${key}_other`;
  const val =
    get(locales[_locale], preferred) ??
    get(locales[_locale], fallback) ??
    get(locales.en, preferred) ??
    get(locales.en, fallback) ??
    get(locales[_locale], key) ??
    get(locales.en, key) ??
    key;
  if (typeof val !== "string") return key;
  return interpolate(val, merged);
}

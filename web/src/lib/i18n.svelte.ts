import { en } from "@lib/locales/en";

type Locale = "en";

const locales: Record<Locale, Record<string, unknown>> = { en };

let _locale = $state<Locale>(
  (localStorage.getItem("vynl.locale") as Locale) || "en",
);

function get(obj: unknown, path: string): unknown {
  let cur: unknown = obj;
  for (const seg of path.split(".")) {
    if (cur == null || typeof cur !== "object") return undefined;
    cur = (cur as Record<string, unknown>)[seg];
  }
  return cur;
}

export function t(
  key: string,
  params?: Record<string, string | number>,
): string {
  const val = get(locales[_locale], key) ?? get(locales.en, key) ?? key;
  if (typeof val !== "string") return key;
  if (!params) return val;
  return val.replace(/\{(\w+)\}/g, (_, k: string) =>
    k in params ? String(params[k]) : `{${k}}`,
  );
}

import { convertFileSrc } from "@tauri-apps/api/core";
import { t } from "@lib/i18n";

function assetUrl(path: string | null | undefined): string {
  if (!path) return "";
  if (/^https?:\/\//i.test(path)) return path;
  return convertFileSrc(path);
}

export function blobSrc(
  node: HTMLImageElement,
  path: string | null | undefined,
): { update: (newPath: string | null | undefined) => void; destroy: () => void } {
  function load(p: string | null | undefined): void {
    if (!p) {
      node.removeAttribute("src");
      return;
    }
    node.src = assetUrl(p);
  }

  load(path);

  return {
    update(newPath: string | null | undefined) {
      load(newPath);
    },
    destroy() {
      const src = node.getAttribute("src");
      if (src && src.startsWith("blob:")) {
        URL.revokeObjectURL(src);
      }
    },
  };
}

export function fmtTime(
  sec: number | null | undefined,
  fallback = "0:00",
): string {
  if (sec == null || !isFinite(sec) || sec <= 0) return fallback;
  const m = Math.floor(sec / 60);
  const s = Math.floor(sec % 60);
  return `${m}:${String(s).padStart(2, "0")}`;
}

export function fmtDuration(sec: number): string {
  const h = Math.floor(sec / 3600);
  const m = Math.floor((sec % 3600) / 60);
  const s = Math.floor(sec % 60);
  if (h > 0) {
    return `${h}${t("format.h")} ${m}${t("format.m")} ${s}${t("format.s")}`;
  }
  if (m > 0) {
    return `${m}${t("format.m")} ${s}${t("format.s")}`;
  }
  return `${s}${t("format.s")}`;
}

export function fmtTotalDuration(sec: number): string {
  const h = Math.floor(sec / 3600);
  const m = Math.floor((sec % 3600) / 60);
  return h > 0
    ? `${h} ${t("format.hr")} ${m} ${t("format.min")}`
    : `${m} ${t("format.min")}`;
}

export function totalSeconds(tracks: { duration?: number | null }[]): number {
  return tracks.reduce((acc, t) => acc + (t.duration ?? 0), 0);
}

export function matchesQuery(query: string, text: string): boolean {
  return query.split(/\s+/).every((w) => text.includes(w));
}

const SUFFIX_PATTERNS = ["io", "os", "js", "ts", "ai", "ui", "vr", "ar", "tv", "fm", "hq", "x", "y", "z"].sort(
  (a, b) => b.length - a.length,
);

function capitalizeWithSuffix(word: string): string {
  const lower = word.toLowerCase();
  for (const suffix of SUFFIX_PATTERNS) {
    if (lower.length > suffix.length && lower.endsWith(suffix)) {
      const base = word.slice(0, word.length - suffix.length);
      return base.charAt(0).toUpperCase() + base.slice(1).toLowerCase() + suffix.toUpperCase();
    }
  }
  return word.charAt(0).toUpperCase() + word.slice(1).toLowerCase();
}

export function toPascalCase(text: string): string {
  const words = text.split(/\s+/).filter(Boolean);
  if (words.length === 0) return text;
  return words.map(capitalizeWithSuffix).join("");
}
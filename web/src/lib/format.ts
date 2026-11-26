import { t } from "@lib/i18n";

export { blobSrc, clearResolvedCovers } from "@lib/blob-src";

export function stripAlbumSuffix(
  title: string,
  albumName: string | null | undefined,
): string {
  if (!albumName) return title;
  const cut = title.lastIndexOf(" (");
  if (cut <= 0 || !title.endsWith(")")) return title;
  const inner = title.slice(cut + 2, -1);
  if (
    albumName === inner ||
    albumName.startsWith(`${inner} `) ||
    albumName.startsWith(`${inner}|`) ||
    albumName.startsWith(`${inner} (`)
  ) {
    return title.slice(0, cut).trim();
  }
  return title;
}

interface WikiSplit {
  body: string;
  attribution: string | null;
}

export function splitWikiAttribution(value: string): WikiSplit {
  const marker = /(?:^|\n)\s*from\s+wikipedia\b/i.exec(value);
  if (!marker) return { body: value, attribution: null };
  const body = value.slice(0, marker.index).trim();
  const attribution = value.slice(marker.index).trim();
  if (!body) return { body: attribution, attribution: null };
  return { body, attribution };
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
  const safe = Number.isFinite(sec) && sec > 0 ? sec : 0;
  const h = Math.floor(safe / 3600);
  const m = Math.floor((safe % 3600) / 60);
  const s = Math.floor(safe % 60);
  if (h > 0) {
    return `${h}${t("format.h")} ${m}${t("format.m")} ${s}${t("format.s")}`;
  }
  if (m > 0) {
    return `${m}${t("format.m")} ${s}${t("format.s")}`;
  }
  return `${s}${t("format.s")}`;
}

export function fmtTotalDuration(sec: number): string {
  const safe = Number.isFinite(sec) && sec > 0 ? sec : 0;
  const h = Math.floor(safe / 3600);
  const m = Math.floor((safe % 3600) / 60);
  return h > 0
    ? `${h} ${t("format.hr")} ${m} ${t("format.min")}`
    : `${m} ${t("format.min")}`;
}

export function totalSeconds(tracks: { duration?: number | null }[]): number {
  return tracks.reduce((acc, t) => {
    const d = t.duration;
    return acc + (typeof d === "number" && Number.isFinite(d) ? d : 0);
  }, 0);
}

export function matchesQuery(query: string, text: string): boolean {
  return query.split(/\s+/).every((w) => text.includes(w));
}

function normalizeSongPart(value: string): string {
  return value
    .toLowerCase()
    .normalize("NFKD")
    .replace(/[\u0300-\u036f]/g, "")
    .replace(/[^a-z0-9]+/g, " ")
    .trim();
}

export function songKey(title: string, artist: string): string {
  return `${normalizeSongPart(title)}|${normalizeSongPart(artist)}`;
}

const SONG_MATCH_TOLERANCE = 3;

export function isSameSong(
  a: { title: string; artist: string; duration: number },
  b: { title: string; artist: string; duration: number },
): boolean {
  if (a.duration <= 0 || b.duration <= 0) return false;
  if (Math.abs(a.duration - b.duration) > SONG_MATCH_TOLERANCE) return false;
  return songKey(a.title, a.artist) === songKey(b.title, b.artist);
}

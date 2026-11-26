import { convertFileSrc } from "@tauri-apps/api/core";
import { t } from "@lib/i18n";
import { vynl } from "@lib/vynl";

const MAX_CACHE_ENTRIES = 500;
const remote = new Map<string, Promise<string>>();
const resolved = new Map<string, string>();

export function clearResolvedCovers(): void {
  resolved.clear();
  remote.clear();
}

function evictIfNeeded(map: Map<string, unknown>): void {
  while (map.size > MAX_CACHE_ENTRIES) {
    const oldest = map.keys().next().value;
    if (oldest === undefined) break;
    map.delete(oldest);
  }
}

function upsizeCover(url: string): string {
  const trimmed = url.trim();
  const start = trimmed.indexOf("=w");
  if (start < 0) return trimmed;

  let i = start + 2;
  while (i < trimmed.length && trimmed[i] >= "0" && trimmed[i] <= "9") i++;
  const width = Number.parseInt(trimmed.slice(start + 2, i), 10);
  if (!Number.isFinite(width) || !trimmed.slice(i).startsWith("-h")) return trimmed;

  let j = i + 2;
  const hStart = j;
  while (j < trimmed.length && trimmed[j] >= "0" && trimmed[j] <= "9") j++;
  if (j === hStart) return trimmed;
  const height = Number.parseInt(trimmed.slice(hStart, j), 10);
  if (!Number.isFinite(height)) return trimmed;

  const seen = new Set<string>();
  const params: string[] = [];
  for (const part of trimmed.slice(j).split("-")) {
    if (part === "" || seen.has(part)) continue;
    seen.add(part);
    params.push(part);
  }

  const nw = Math.max(width, 512);
  const nh = Math.max(height, 512);
  const dashCount = (trimmed.slice(j).match(/-/g) ?? []).length;
  if (nw === width && nh === height && params.length === dashCount) return trimmed;

  return `${trimmed.slice(0, start)}=w${nw}-h${nh}${params.map((p) => `-${p}`).join("")}`;
}

function cacheRemote(url: string): Promise<string> {
  let pending = remote.get(url);
  if (!pending) {
    pending = vynl
      .cacheCover(url)
      .then((local) => {
        remote.delete(url);
        return local;
      })
      .catch((error) => {
        remote.delete(url);
        throw error;
      });
    remote.set(url, pending);
    evictIfNeeded(remote);
  }
  return pending;
}

export function blobSrc(
  node: HTMLImageElement,
  path: string | null | undefined,
): { update: (newPath: string | null | undefined) => void; destroy: () => void } {
  let token = 0;

  function srcFor(p: string): string {
    if (/^https?:\/\//i.test(p)) {
      const src = upsizeCover(p);
      const known = resolved.get(src);
      if (known) return convertFileSrc(known);
      node.referrerPolicy = "no-referrer";
      void cacheRemote(src).then(
        (local) => {
          resolved.set(src, local);
          evictIfNeeded(resolved);
        },
        () => {},
      );
      return src;
    }
    return convertFileSrc(p);
  }

  function load(p: string | null | undefined): void {
    const mine = ++token;
    if (!p) {
      node.removeAttribute("src");
      return;
    }
    const src = srcFor(p);
    if (node.getAttribute("src") === src || node.src === src) return;

    const preload = new Image();
    if (/^https?:\/\//i.test(src)) preload.referrerPolicy = "no-referrer";
    const apply = () => {
      if (mine === token) node.src = src;
    };
    preload.onload = apply;
    preload.onerror = apply;
    preload.src = src;
  }

  load(path);

  return {
    update(newPath: string | null | undefined) {
      load(newPath);
    },
    destroy() {
      token++;
    },
  };
}

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

import type { LyricsResult } from "@lib/types";
import { evictOldest } from "@lib/cache";
import { vynl } from "@lib/vynl";
import { getCurrentTrack, getCurrentId } from "@state/now-playing.svelte";
import { getSmoothTime } from "@state/media-clock.svelte";
import type { LyricLine, Word } from "@lib/lyrics/parse";
import { HAS_STAMP_RE, lyricsKindOf, plainLine, toLyricLines } from "@lib/lyrics/parse";

export type { Word, LyricLine } from "@lib/lyrics/parse";
export { isInstrumental, lyricsKindOf, toLyricLines } from "@lib/lyrics/parse";

type CacheEntry = { result: LyricsResult; lines: LyricLine[] };

const MAX_CACHE = 500;
const LAST_LINE_GRACE_SECS = 10;
const LAST_WORD_FALLBACK_SECS = 0.45;

let loading = $state(false);
let result = $state<LyricsResult | null>(null);
let lines = $state<LyricLine[]>([]);
let loadError = $state<string | null>(null);

const cache = new Map<string, CacheEntry>();
const embedded = new Set<string>();
const prefetching = new Set<string>();
let activeId: string | null = null;
let loadingId: string | null = null;
let loadVersion = 0;

function clearState(): void {
  loading = false;
  result = null;
  lines = [];
  loadError = null;
}

function setFromCache(entry: CacheEntry): void {
  loading = false;
  result = entry.result;
  lines = entry.lines;
  loadError = null;
}

function storeCache(path: string, res: LyricsResult, lyricLines: LyricLine[]): void {
  cache.set(path, { result: res, lines: lyricLines });
  evictOldest(cache, MAX_CACHE);
}

function tryEmbed(res: LyricsResult, path: string): void {
  if (res.source !== "remote" || res.kind === "ttml" || embedded.has(path)) return;
  const text =
    res.kind === "lrc" || HAS_STAMP_RE.test(res.text)
      ? res.text.split(/\r?\n/).map(plainLine).filter(Boolean).join("\n")
      : res.text.trim();
  if (!text) return;
  embedded.add(path);
  void vynl.lyricsEmbed({ file: path, text }).catch(() => embedded.delete(path));
}

async function persistManualLyrics(path: string, res: LyricsResult): Promise<void> {
  try {
    await vynl.lyricsSave({ file: path, kind: res.kind, text: res.text });
  } catch (error) {
    console.warn("Failed to save lyrics sidecar", error);
  }
  if (res.kind === "ttml") return;
  const plain = res.text.split(/\r?\n/).map(plainLine).filter(Boolean).join("\n");
  if (!plain) return;
  try {
    await vynl.lyricsEmbed({ file: path, text: plain });
  } catch (error) {
    console.warn("Failed to embed lyrics", error);
  }
}

export function prefetchLyrics(
  track:
    | {
        path: string;
        title: string;
        artist: string;
        album: string;
        duration: number;
        lyrics?: string | null;
      }
    | null
    | undefined,
): void {
  const path = track?.path;
  if (!path || cache.has(path) || prefetching.has(path)) return;
  const current = getCurrentTrack();
  const currentId = getCurrentId();
  if (current?.path === path && (activeId === currentId || loadingId === currentId)) return;
  prefetching.add(path);
  void (async () => {
    try {
      const embeddedText = track.lyrics?.trim() ?? "";
      const embeddedKind = embeddedText ? lyricsKindOf(embeddedText) : null;
      let res = await vynl.lyricsLocal(path);
      if (!res && embeddedText && embeddedKind && embeddedKind !== "txt") {
        res = { kind: embeddedKind, text: embeddedText, source: "local" };
      }
      if (!res) {
        res = await vynl.lyricsFetch({
          title: track.title,
          artist: track.artist,
          album: track.album,
          duration: track.duration,
        });
        if (res) tryEmbed(res, path);
      }
      if (!res && embeddedText) {
        res = { kind: "txt", text: embeddedText, source: "local" };
      }
      if (res && !cache.has(path)) storeCache(path, res, toLyricLines(res));
    } catch {
    } finally {
      prefetching.delete(path);
    }
  })();
}

export function loadLyrics(id: string | null): void {
  if (!id) {
    loadVersion++;
    activeId = null;
    loadingId = null;
    clearState();
    return;
  }

  const track = getCurrentTrack();
  if (!track) {
    loadVersion++;
    activeId = null;
    loadingId = null;
    clearState();
    return;
  }

  if (loadingId === id) return;
  if (activeId === id && result) return;

  const version = ++loadVersion;
  activeId = id;
  loadingId = id;
  result = null;
  lines = [];
  loadError = null;

  const cached = cache.get(track.path);
  if (cached) {
    setFromCache(cached);
    loadingId = null;
    return;
  }

  loading = true;
  void (async () => {
    try {
      const embeddedText = track.lyrics?.trim() ?? "";
      const embeddedKind = embeddedText ? lyricsKindOf(embeddedText) : null;

      let res = await vynl.lyricsLocal(track.path);
      if (version !== loadVersion) return;

      if (!res && embeddedText && embeddedKind && embeddedKind !== "txt") {
        res = { kind: embeddedKind, text: embeddedText, source: "local" };
      }

      if (!res) {
        res = await vynl.lyricsFetch({
          title: track.title,
          artist: track.artist,
          album: track.album,
          duration: track.duration,
        });
        if (version !== loadVersion) return;
        if (res) tryEmbed(res, track.path);
      }

      if (!res && embeddedText) {
        res = { kind: "txt", text: embeddedText, source: "local" };
      }

      if (version !== loadVersion) return;
      result = res;
      lines = res ? toLyricLines(res) : [];
      loadError = null;
      if (res) storeCache(track.path, res, lines);
      else activeId = null;
    } catch (error) {
      if (version !== loadVersion) return;
      const embeddedText = track.lyrics?.trim() ?? "";
      const res = embeddedText
        ? ({ kind: lyricsKindOf(embeddedText), text: embeddedText, source: "local" } as LyricsResult)
        : null;
      result = res;
      lines = res ? toLyricLines(res) : [];
      loadError =
        res
          ? null
          : error instanceof Error && error.message
            ? error.message
            : "could not load lyrics";
      if (res) storeCache(track.path, res, lines);
      else activeId = null;
    } finally {
      if (loadingId === id) loadingId = null;
      if (version === loadVersion) loading = false;
    }
  })();
}

export function setLyricsManual(res: LyricsResult, parsedLines: LyricLine[]): void {
  loadVersion++;
  loading = false;
  loadingId = null;
  result = res;
  lines = parsedLines;
  loadError = null;
  const track = getCurrentTrack();
  if (!track) return;
  activeId = getCurrentId();
  storeCache(track.path, res, parsedLines);
  void persistManualLyrics(track.path, res);
}

function findActiveLineIndex(lyricLines: LyricLine[], t: number): number {
  let lo = 0;
  let hi = lyricLines.length - 1;
  let idx = -1;
  while (lo <= hi) {
    const mid = (lo + hi) >> 1;
    const time = lyricLines[mid].time;
    if (time >= 0 && time <= t) {
      idx = mid;
      lo = mid + 1;
    } else if (time > t) {
      hi = mid - 1;
    } else {
      lo = mid + 1;
    }
  }
  if (idx >= 0 && idx === lyricLines.length - 1 && t - lyricLines[idx].time > LAST_LINE_GRACE_SECS) {
    return -1;
  }
  return idx;
}

function findActiveWordIndex(words: Word[], t: number): number {
  let lo = 0;
  let hi = words.length - 1;
  let idx = -1;
  while (lo <= hi) {
    const mid = (lo + hi) >> 1;
    if (words[mid].time <= t) {
      idx = mid;
      lo = mid + 1;
    } else {
      hi = mid - 1;
    }
  }
  return idx;
}

function wordEndAt(words: Word[], i: number, lineEnd?: number): number {
  const w = words[i];
  if (!w) return 0;
  if (w.end !== undefined && w.end > w.time) return w.end;
  const next = words[i + 1];
  if (next && next.time > w.time) return next.time;
  if (lineEnd !== undefined && lineEnd > w.time) return lineEnd;
  return w.time + LAST_WORD_FALLBACK_SECS;
}

export function wordFill(words: Word[], i: number, t: number, lineEnd?: number): number {
  const w = words[i];
  if (!w || t <= w.time) return 0;
  const end = wordEndAt(words, i, lineEnd);
  const span = end - w.time;
  if (span <= 0) return 1;
  const fill = (t - w.time) / span;
  return fill < 0 ? 0 : fill > 1 ? 1 : fill;
}

const synced = $derived(lines.some((l) => l.time >= 0));
const activeIndex = $derived(synced ? findActiveLineIndex(lines, getSmoothTime()) : -1);
const activeWordIndex = $derived.by(() => {
  if (!synced || activeIndex < 0) return -1;
  const words = lines[activeIndex]?.words;
  return words?.length ? findActiveWordIndex(words, getSmoothTime()) : -1;
});

export function getLyricsLoading(): boolean {
  return loading;
}
export function getLyricsError(): string | null {
  return loadError;
}
export function getLyricsResult(): LyricsResult | null {
  return result;
}
export function getLyricsLines(): LyricLine[] {
  return lines;
}
export function getLyricsSynced(): boolean {
  return synced;
}
export function getLyricsActiveIndex(): number {
  return activeIndex;
}
export function getLyricsActiveWordIndex(): number {
  return activeWordIndex;
}

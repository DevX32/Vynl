import type { LyricsResult } from "@lib/types";
import { vynl } from "@lib/vynl";
import { runLyricsProviders } from "@lib/plugins/providers";
import { getCurrentTrack, getCurrentTime } from "@state/now-playing.svelte";

type Word = { time: number; text: string };
export type LyricLine = { time: number; text: string; words?: Word[] };

let _loading = $state(false);
let _result = $state<LyricsResult | null>(null);
let _lines = $state<LyricLine[]>([]);

const MAX_CACHE_SIZE = 500;
const _cache = new Map<string, { result: LyricsResult; lines: LyricLine[] }>();
let _activeId: string | null = null;
let _loadVersion = 0;

function parseEnhancedWords(body: string): Word[] | null {
  const wordTagRe = /<(\d{1,2}):(\d{2})(?:[.:](\d{1,3}))?>/g;
  const wordTimestamps: number[] = [];
  let wm: RegExpExecArray | null;
  wordTagRe.lastIndex = 0;
  while ((wm = wordTagRe.exec(body))) {
    const min = Number(wm[1]);
    const sec = Number(wm[2]);
    const frac = wm[3] ? Number(wm[3].padEnd(3, "0")) / 1000 : 0;
    wordTimestamps.push(min * 60 + sec + frac);
  }
  if (wordTimestamps.length === 0) return null;
  const textSegments = body.split(/<\d{1,2}:\d{2}(?:[.:]\d{1,3})?>/);
  const words: Word[] = [];
  for (let i = 0; i < wordTimestamps.length; i++) {
    const text = (textSegments[i + 1] ?? "").trim();
    if (text) words.push({ time: wordTimestamps[i], text });
  }
  return words.length > 0 ? words : null;
}

const METADATA_TAG_RE =
  /^\[(ar|ti|al|au|by|offset|re|ve|length|created|tool|version|application):/i;

function parseLrc(text: string): LyricLine[] {
  const out: LyricLine[] = [];
  const re = /\[(\d{1,2}):(\d{1,2})(?:[.:](\d{1,3}))?\]/g;
  for (const raw of text.split(/\r?\n/)) {
    if (METADATA_TAG_RE.test(raw.trim())) continue;

    const stamps: number[] = [];
    let m: RegExpExecArray | null;
    re.lastIndex = 0;
    while ((m = re.exec(raw))) {
      const min = Number(m[1]);
      const sec = Number(m[2]);
      const frac = m[3] ? Number(m[3].padEnd(3, "0")) / 1000 : 0;
      stamps.push(min * 60 + sec + frac);
    }
    const body = raw.replace(re, "").trim();

    if (stamps.length === 0) {
      if (!body) continue;
      out.push({ time: -1, text: body });
      continue;
    }

    const words = parseEnhancedWords(body);
    const cleanBody = body
      .replace(/<\d{1,2}:\d{2}(?:[.:]\d{1,3})?>/g, "")
      .trim();
    for (const s of stamps)
      out.push({
        time: s,
        text: cleanBody,
        words: words ?? undefined,
      });
  }
  return out.sort((a, b) => a.time - b.time);
}

function toLines(res: LyricsResult): LyricLine[] {
  return res.kind === "lrc"
    ? parseLrc(res.text)
    : res.text.split(/\r?\n/).map((x) => ({ time: -1, text: x }));
}

function clearLyricsState(): void {
  _loading = false;
  _result = null;
  _lines = [];
}

function checkCache(t: { path: string }): boolean {
  const cached = _cache.get(t.path);
  if (!cached) return false;
  _loading = false;
  _result = cached.result;
  _lines = cached.lines;
  return true;
}

function storeInCache(path: string, res: LyricsResult, lines: LyricLine[]): void {
  _cache.set(path, { result: res, lines });
  if (_cache.size > MAX_CACHE_SIZE) {
    const firstKey = _cache.keys().next().value;
    if (firstKey !== undefined) _cache.delete(firstKey);
  }
}

function tryParseEmbeddedLyrics(res: LyricsResult, t: { path: string }): void {
  if (res.source !== "remote") return;
  const embedText =
    res.kind === "lrc"
      ? res.text.replace(/^\[\d{1,2}:\d{2}(?:[.:]\d{1,3})?\]\s*/gm, "").trim()
      : res.text;
  if (embedText) void vynl.lyricsEmbed({ file: t.path, text: embedText }).catch(() => {});
}

export function loadLyrics(id: string | null): void {
  if (_activeId === id) return;
  _activeId = id;
  const version = ++_loadVersion;

  if (!id) { clearLyricsState(); return; }
  const t = getCurrentTrack();
  if (!t) { clearLyricsState(); return; }
  if (checkCache(t)) return;

  _loading = true;
  void (async () => {
    const embeddedLyrics = t.lyrics?.trim() ?? null;
    const embeddedIsLrc = embeddedLyrics
      ? /\[\d{1,2}:\d{2}/.test(embeddedLyrics)
      : false;

    if (embeddedLyrics && embeddedIsLrc) {
      const res: LyricsResult = { kind: "lrc", text: embeddedLyrics, source: "local" };
      if (version !== _loadVersion) return;
      _loading = false;
      _result = res;
      _lines = toLines(res);
      storeInCache(t.path, res, _lines);
      return;
    }

    const local = await vynl.lyricsLocal(t.path);
    if (version !== _loadVersion) return;
    let res = local;
    if (!res) {
      const viaPlugin = await runLyricsProviders({
        title: t.title,
        artist: t.artist,
        album: t.album,
        duration: t.duration,
      });
      if (version !== _loadVersion) return;
      if (viaPlugin) {
        res = { kind: viaPlugin.kind, text: viaPlugin.text, source: "remote" };
        tryParseEmbeddedLyrics(res, t);
      }
    }
    if (!res) {
      res = await vynl.lyricsFetch({
        title: t.title,
        artist: t.artist,
        album: t.album,
        duration: t.duration,
      });
      if (res) tryParseEmbeddedLyrics(res, t);
    }
    if (version !== _loadVersion) return;
    if (!res && embeddedLyrics) {
      res = { kind: "txt", text: embeddedLyrics, source: "local" };
    }
    _loading = false;
    _result = res;
    _lines = res ? toLines(res) : [];
    if (res) storeInCache(t.path, res, _lines);
  })();
}

function findActiveLineIndex(lines: LyricLine[], t: number): number {
  let idx = -1;
  for (let i = 0; i < lines.length; i++) {
    if (lines[i].time >= 0 && lines[i].time <= t) idx = i;
    else if (lines[i].time > t) break;
  }
  return idx;
}

function findActiveWordIndex(words: Word[], t: number): number {
  let wIdx = -1;
  for (let i = 0; i < words.length; i++) {
    if (words[i].time <= t) wIdx = i;
    else break;
  }
  return wIdx;
}

const _synced = $derived(_result?.kind === "lrc");

const _activeIndex = $derived(
  _synced
    ? findActiveLineIndex(_lines, getCurrentTime())
    : -1,
);

const _activeWordIndex = $derived(
  _synced && _activeIndex >= 0 && _lines[_activeIndex]?.words
    ? findActiveWordIndex(_lines[_activeIndex]!.words!, getCurrentTime())
    : -1,
);

export function getLyricsLoading(): boolean {
  return _loading;
}
export function getLyricsResult(): LyricsResult | null {
  return _result;
}
export function getLyricsLines(): LyricLine[] {
  return _lines;
}
export function getLyricsSynced(): boolean {
  return _synced;
}
export function getLyricsActiveIndex(): number {
  return _activeIndex;
}
export function getLyricsActiveWordIndex(): number {
  return _activeWordIndex;
}

export function isInstrumental(text: string): boolean {
  const t = text.trim();
  if (!t) return true;
  if (/^[\s♪♫♩♬🎵🎶…~～·.\-—–]+$/.test(t)) return true;
  return /^[(\[]?(instrumental|interlude|outro|intro|solo|break|bridge|build.?up|fade[.\s]?out|fade[.\s]?in|prelude|postlude|reprise)[)\]]?$/i.test(
    t,
  );
}

export function setLyricsManual(
  result: LyricsResult,
  parsedLines: LyricLine[],
): void {
  _result = result;
  _lines = parsedLines;
  const t = getCurrentTrack();
  if (t) _cache.set(t.path, { result, lines: parsedLines });
}

export function parseLrcText(text: string): LyricLine[] {
  return parseLrc(text);
}

export function toLyricLines(res: LyricsResult): LyricLine[] {
  return toLines(res);
}

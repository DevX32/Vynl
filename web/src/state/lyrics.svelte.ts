import type { LyricsResult } from "@lib/types";
import { vynl } from "@lib/vynl";
import { runLyricsProviders } from "@lib/plugins/providers";
import { getCurrentTrack, getCurrentTime } from "@state/now-playing.svelte";

type Word = { time: number; text: string; end?: number; bg?: boolean };
type LyricLine = {
  time: number;
  text: string;
  words?: Word[];
  end?: number;
  bg?: boolean;
  agent?: string;
  roman?: string;
};

let _loading = $state(false);
let _result = $state<LyricsResult | null>(null);
let _lines = $state<LyricLine[]>([]);

const MAX_CACHE_SIZE = 500;
const _cache = new Map<string, { result: LyricsResult; lines: LyricLine[] }>();
let _activeId: string | null = null;
let _loadVersion = 0;

function parseEnhancedWords(body: string, offset = 0): Word[] | null {
  const wordTagRe = /<(\d{1,2}):(\d{2})(?:[.:](\d{1,3}))?>/g;
  const wordTimestamps: number[] = [];
  let wm: RegExpExecArray | null;
  wordTagRe.lastIndex = 0;
  while ((wm = wordTagRe.exec(body))) {
    const min = Number(wm[1]);
    const sec = Number(wm[2]);
    const frac = wm[3] ? Number(wm[3].padEnd(3, "0")) / 1000 : 0;
    wordTimestamps.push(min * 60 + sec + frac + offset);
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

const OFFSET_TAG_RE = /^\[offset:\s*([+-]?\d+)\s*\]/i;

function readOffsetSecs(text: string): number {
  for (const raw of text.split(/\r?\n/)) {
    const m = OFFSET_TAG_RE.exec(raw.trim());
    if (m) {
      const ms = Number(m[1]);
      if (Number.isFinite(ms)) return ms / 1000;
    }
  }
  return 0;
}

function parseLrc(text: string): LyricLine[] {
  const out: LyricLine[] = [];
  const re = /\[(\d{1,2}):(\d{1,2})(?:[.:](\d{1,3}))?\]/g;
  const offset = readOffsetSecs(text);
  for (const raw of text.split(/\r?\n/)) {
    if (METADATA_TAG_RE.test(raw.trim())) continue;

    const stamps: number[] = [];
    let m: RegExpExecArray | null;
    re.lastIndex = 0;
    while ((m = re.exec(raw))) {
      const min = Number(m[1]);
      const sec = Number(m[2]);
      const frac = m[3] ? Number(m[3].padEnd(3, "0")) / 1000 : 0;
      stamps.push(min * 60 + sec + frac + offset);
    }
    const body = raw.replace(re, "").trim();

    if (stamps.length === 0) {
      if (!body) continue;
      out.push({ time: -1, text: body });
      continue;
    }

    const words = parseEnhancedWords(body, offset);
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

function parseClock(value: string): number {
  const m = /^(\d+):(\d{1,2})(?:[.:](\d{1,3}))?$/.exec(value.trim());
  if (m) {
    const frac = m[3] ? Number(m[3].padEnd(3, "0")) / 1000 : 0;
    return Number(m[1]) * 60 + Number(m[2]) + frac;
  }
  const secs = Number(value.trim().replace(/s$/i, ""));
  return Number.isFinite(secs) ? secs : 0;
}

function parseOffsetAttr(attr: string): number {
  const clock = /offset\s*=\s*"([^"]+)"/i.exec(attr);
  if (clock) {
    const raw = clock[1].trim();
    const sign = raw.startsWith("-") ? -1 : 1;
    return sign * parseClock(raw.replace(/^[+-]/, ""));
  }
  const secs = /=\s*"?([\d.]+)"?\s*s"?$/i.exec(attr);
  if (secs) return Number(secs[1]);
  return 0;
}

function decodeEntities(s: string): string {
  return s
    .replace(/&lt;/g, "<")
    .replace(/&gt;/g, ">")
    .replace(/&quot;/g, '"')
    .replace(/&apos;/g, "'")
    .replace(/&#(\d+);/g, (_, d: string) => String.fromCharCode(Number(d)))
    .replace(/&#x([0-9a-f]+);/gi, (_, h: string) => String.fromCharCode(parseInt(h, 16)))
    .replace(/&amp;/g, "&");
}

function parseTtml(text: string): LyricLine[] {
  const doc = new DOMParser().parseFromString(text, "application/xml");
  if (doc.querySelector("parsererror")) return [];

  const body = doc.querySelector("body");
  if (!body) return [];

  let offset = 0;
  const ittp = doc.querySelector("head > ittp, head > metadata > ittp");
  if (ittp) {
    const frameRate = Number(ittp.getAttribute("ttp:frameRate"));
    const tickRate = Number(ittp.getAttribute("ttp:tickRate"));
    if (Number.isFinite(tickRate) && tickRate > 0 && Number.isFinite(frameRate) && frameRate > 0) {
      offset -= frameRate / tickRate;
    }
  }

  const timing = body.getAttribute("itunes:timing");
  if (timing) offset += parseOffsetAttr(`offset="${timing}"`);

  const out: LyricLine[] = [];
  const romans: Array<{ time: number; text: string }> = [];

  const attr = (el: Element, name: string): string | null =>
    el.getAttribute(name) ?? el.getAttributeNS("*", name.split(":")[1]);

  const roleOf = (el: Element): string => (attr(el, "ttm:role") ?? "").toLowerCase();

  const collect = (p: Element): void => {
    const beginRaw = attr(p, "begin");
    if (beginRaw === null) return;
    const start = parseClock(beginRaw) + offset;
    const endRaw = attr(p, "end");
    const endRawNum = endRaw !== null ? parseClock(endRaw) + offset : null;
    const end = endRawNum !== null && endRawNum > start ? endRawNum : null;

    const role = roleOf(p);
    const romanRole = /translation|romanization|romaji|translit/.test(role);
    const bgRole = /(^|[-_])bg$|background/.test(role);

    const spans = Array.from(p.querySelectorAll("span")).filter(
      (s) => attr(s, "begin") !== null,
    );
    const words: Word[] = [];
    let full = "";

    for (const span of spans) {
      const sb = attr(span, "begin")!;
      const se = attr(span, "end");
      const wStart = parseClock(sb) + offset;
      const wEndRaw = se !== null ? parseClock(se) + offset : null;
      const wEnd = wEndRaw !== null && wEndRaw > wStart ? wEndRaw : null;
      const wText = decodeEntities(span.textContent ?? "").replace(/\s+/g, " ").trim();
      if (!wText) continue;
      full += (full ? " " : "") + wText;
      words.push({
        time: wStart,
        text: wText,
        ...(wEnd !== null ? { end: wEnd } : {}),
        ...(/bg|background/.test(roleOf(span)) ? { bg: true } : {}),
      });
    }

    const lineText = full || decodeEntities(p.textContent ?? "").replace(/\s+/g, " ").trim();
    if (!lineText) return;

    if (romanRole) {
      romans.push({ time: start, text: lineText });
      return;
    }

    out.push({
      time: start,
      text: lineText,
      words: words.length > 0 ? words : undefined,
      ...(end !== null ? { end } : {}),
      ...(bgRole ? { bg: true } : {}),
      ...(attr(p, "ttm:agent") ? { agent: attr(p, "ttm:agent")! } : {}),
    });
  };

  body.querySelectorAll("p").forEach(collect);

  if (romans.length > 0 && out.length > 0) {
    for (const r of romans) {
      let best: LyricLine | null = null;
      let bestDist = Infinity;
      for (const line of out) {
        const d = Math.abs(line.time - r.time);
        if (d < bestDist) {
          bestDist = d;
          best = line;
        }
      }
      if (best && bestDist < 5) best.roman = r.text;
    }
  }

  return out.sort((a, b) => a.time - b.time);
}

function toLines(res: LyricsResult): LyricLine[] {
  if (res.kind === "ttml") return parseTtml(res.text);
  if (res.kind === "lrc") return parseLrc(res.text);
  return res.text.split(/\r?\n/).map((x) => ({ time: -1, text: x }));
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

const _embedded = new Set<string>();

function tryParseEmbeddedLyrics(res: LyricsResult, t: { path: string }): void {
  if (res.source !== "remote") return;
  if (res.kind === "ttml") return;
  if (_embedded.has(t.path)) return;
  const embedText =
    res.kind === "lrc"
      ? res.text.replace(/^\[\d{1,2}:\d{2}(?:[.:]\d{1,3})?\]\s*/gm, "").trim()
      : res.text;
  if (!embedText) return;
  _embedded.add(t.path);
  void vynl
    .lyricsEmbed({ file: t.path, text: embedText })
    .catch(() => _embedded.delete(t.path));
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

    let res: LyricsResult | null = null;

    res = await vynl.lyricsLocal(t.path);
    if (version !== _loadVersion) return;

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

const _synced = $derived(_result?.kind === "lrc" || _result?.kind === "ttml");

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

export function parseTtmlText(text: string): LyricLine[] {
  return parseTtml(text);
}

export function isTtmlText(text: string): boolean {
  return /<tt[\s>]/i.test(text) || /<ttml/i.test(text);
}

export function toLyricLines(res: LyricsResult): LyricLine[] {
  return toLines(res);
}

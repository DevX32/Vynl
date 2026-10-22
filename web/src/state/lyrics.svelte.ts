import type { LyricsResult } from "@lib/types";
import { vynl } from "@lib/vynl";
import { runLyricsProviders } from "@lib/plugins/providers";
import { getCurrentTrack } from "@state/now-playing.svelte";
import { getSmoothTime } from "@state/media-clock.svelte";

export type Word = {
  time: number;
  text: string;
  end?: number;
  bg?: boolean;
  /** Whether the source had whitespace after this word (karaoke spacing). */
  sep?: boolean;
};
export type LyricLine = {
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

const STAMP_RE =
  /\[[ \t]*(\d{1,3}):(\d{1,2})(?::(\d{1,4}))?(?:[.,](\d{1,4}))?[ \t]*\]/g;
const HAS_STAMP_RE =
  /\[[ \t]*\d{1,3}:\d{1,2}(?::\d{1,4})?(?:[.,]\d{1,4})?[ \t]*\]/;
const WORD_TAG_RE =
  /<[ \t]*(\d{1,3}):(\d{1,2})(?::(\d{1,4}))?(?:[.,](\d{1,4}))?[ \t]*>/g;
const WORD_TAG_STRIP_RE =
  /<[ \t]*\d{1,3}:\d{1,2}(?::\d{1,4})?(?:[.,]\d{1,4})?[ \t]*>/g;

function clockSeconds(m: RegExpExecArray): number {
  const a = Number(m[1]);
  const b = Number(m[2]);
  const third = m[3];
  const frac = m[4] ? Number(`0.${m[4]}`) : 0;
  if (!third) return a * 60 + b + frac;
  if (third.length <= 2) return a * 3600 + b * 60 + Number(third) + frac;
  return a * 60 + b + Number(`0.${third}`);
}

function parseEnhancedWords(body: string, offset = 0): Word[] | null {
  const wordTimestamps: number[] = [];
  let wm: RegExpExecArray | null;
  WORD_TAG_RE.lastIndex = 0;
  while ((wm = WORD_TAG_RE.exec(body))) {
    wordTimestamps.push(clockSeconds(wm) + offset);
  }
  if (wordTimestamps.length === 0) return null;
  const textSegments = body.split(WORD_TAG_STRIP_RE);
  const words: Word[] = [];
  for (let i = 0; i < wordTimestamps.length; i++) {
    const seg = textSegments[i + 1] ?? "";
    const text = seg.trim();
    if (!text) continue;
    const lead = seg.length - seg.trimStart().length;
    const trailing = seg.slice(lead + text.length);
    words.push({ time: wordTimestamps[i], text, sep: /\s/.test(trailing) });
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

const NOMINAL_LINE_SECS = 4;
const MIN_WORD_SECS = 0.12;
const LAST_LINE_GRACE_SECS = 10;

function distributeWords(text: string, start: number, end: number): Word[] {
  const parts = text.split(/\s+/).filter(Boolean);
  if (parts.length === 0) return [];
  const slice = Math.max(end - start, MIN_WORD_SECS * parts.length) / parts.length;
  return parts.map((w, i) => ({
    time: start + i * slice,
    text: w,
    end: start + (i + 1) * slice,
  }));
}

function applyEstimatedWords(lines: LyricLine[]): LyricLine[] {
  return lines.map((line, i) => {
    if (line.time < 0 || line.words?.length || isInstrumental(line.text)) return line;
    const next = lines[i + 1];
    const end =
      next && next.time > line.time ? next.time : line.time + NOMINAL_LINE_SECS;
    const words = distributeWords(line.text, line.time, end);
    return words.length > 0 ? { ...line, end, words } : line;
  });
}

function parseLrc(text: string): LyricLine[] {
  const out: LyricLine[] = [];
  const offset = readOffsetSecs(text);
  for (const raw of text.split(/\r?\n/)) {
    if (METADATA_TAG_RE.test(raw.trim())) continue;

    const stamps: number[] = [];
    let m: RegExpExecArray | null;
    STAMP_RE.lastIndex = 0;
    while ((m = STAMP_RE.exec(raw))) {
      stamps.push(clockSeconds(m) + offset);
    }
    const body = raw.replace(STAMP_RE, "").replace(/\s+/g, " ").trim();

    if (stamps.length === 0) {
      const plain = body
        .replace(WORD_TAG_STRIP_RE, "")
        .replace(/\s+/g, " ")
        .trim();
      if (!plain) continue;
      out.push({ time: -1, text: plain });
      continue;
    }

    const words = parseEnhancedWords(body, offset);
    const cleanBody = body
      .replace(WORD_TAG_STRIP_RE, "")
      .replace(/\s+/g, " ")
      .trim();
    for (const s of stamps)
      out.push({
        time: s,
        text: cleanBody,
        words: words ?? undefined,
      });
  }
  return applyEstimatedWords(out.sort((a, b) => a.time - b.time));
}

function parseClock(value: string): number {
  const v = value.trim();
  const m = /^(?:(\d+):)?(\d{1,2}):(\d{1,2})(?:[.:](\d{1,3}))?$/.exec(v);
  if (m) {
    const hours = m[1] ? Number(m[1]) : 0;
    const frac = m[4] ? Number(`0.${m[4]}`) : 0;
    return hours * 3600 + Number(m[2]) * 60 + Number(m[3]) + frac;
  }
  const secs = Number(v.replace(/s$/i, ""));
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

type RawSpan = {
  time: number;
  text: string;
  end: number | null;
  bg: boolean;
  sep: boolean;
};

function collectSpans(
  node: Node,
  out: RawSpan[],
  sep: boolean,
  bg: boolean,
  offset: number,
  attr: (el: Element, name: string) => string | null,
  bgRole: RegExp,
): void {
  let separated = sep;
  for (const child of Array.from(node.childNodes)) {
    if (child.nodeType === 3) {
      const text = child.textContent ?? "";
      if (text.length > 0) separated = /\s$/.test(text);
      continue;
    }
    const el = child as Element;
    if (el.nodeName === "br") {
      separated = true;
      continue;
    }

    const childBg = bg || bgRole.test(attr(el, "ttm:role") ?? attr(el, "role") ?? "");
    const beginRaw = attr(el, "begin");

    if (el.nodeName === "span" && beginRaw !== null) {
      const raw = decodeEntities(el.textContent ?? "").replace(/\s+/g, " ");
      const text = raw.trim();
      if (!text) {
        separated = false;
        continue;
      }
      const time = parseClock(beginRaw) + offset;
      const endRaw = attr(el, "end");
      const endNum = endRaw !== null ? parseClock(endRaw) + offset : null;
      out.push({
        time,
        text,
        end: endNum !== null && endNum > time ? endNum : null,
        bg: childBg,
        sep: separated,
      });
      separated = / $/.test(raw);
      continue;
    }

    collectSpans(el, out, separated, childBg, offset, attr, bgRole);
    separated = false;
  }
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
    const bgRole = /(^|[-_])bg$|background/;

    const spans: RawSpan[] = [];
    collectSpans(p, spans, false, false, offset, attr, bgRole);

    let full = "";
    for (let i = 0; i < spans.length; i++) {
      full += (i > 0 && spans[i].sep ? " " : "") + spans[i].text;
    }

    const words: Word[] = [];
    for (const span of spans) {
      if (span.bg) continue;
      const last = words[words.length - 1];
      if (last && !span.sep) {
        words[words.length - 1] = {
          ...last,
          text: last.text + span.text,
          ...(span.end !== null ? { end: span.end } : {}),
        };
        continue;
      }
      words.push({
        time: span.time,
        text: span.text,
        ...(span.end !== null ? { end: span.end } : {}),
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
      ...(bgRole.test(role) ? { bg: true } : {}),
      ...(attr(p, "ttm:agent") ? { agent: attr(p, "ttm:agent")! } : {}),
    });
  };

  body.querySelectorAll("p").forEach(collect);

  const agents: string[] = [];
  for (const line of out) {
    if (line.agent && !agents.includes(line.agent)) agents.push(line.agent);
  }
  if (agents.length < 2) {
    for (const line of out) delete line.agent;
  } else {
    const [first, second] = agents;
    for (const line of out) {
      if (line.agent === first) line.agent = "v1";
      else if (line.agent === second) line.agent = "v2";
      else delete line.agent;
    }
  }

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

function parsePlainText(text: string): LyricLine[] {
  const out: LyricLine[] = [];
  for (const raw of text.split(/\r?\n/)) {
    const line = raw
      .replace(STAMP_RE, "")
      .replace(WORD_TAG_STRIP_RE, "")
      .replace(/\s+/g, " ")
      .trim();
    if (line) out.push({ time: -1, text: line });
  }
  return out;
}

function toLines(res: LyricsResult): LyricLine[] {
  if (res.kind === "ttml") return parseTtml(res.text);
  if (res.kind === "lrc") return parseLrc(res.text);
  if (HAS_STAMP_RE.test(res.text)) {
    const lines = parseLrc(res.text);
    if (lines.some((l) => l.time >= 0)) return lines;
  }
  return parsePlainText(res.text);
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
    res.kind === "lrc" || HAS_STAMP_RE.test(res.text)
      ? res.text
          .split(/\r?\n/)
          .map((line) =>
            line
              .replace(STAMP_RE, "")
              .replace(WORD_TAG_STRIP_RE, "")
              .replace(/\s+/g, " ")
              .trim(),
          )
          .join("\n")
          .trim()
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
      ? HAS_STAMP_RE.test(embeddedLyrics)
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
  if (idx === lines.length - 1 && t - lines[idx].time > LAST_LINE_GRACE_SECS) {
    return -1;
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

const LAST_WORD_FALLBACK_SECS = 0.45;

export function wordEndAt(
  words: Word[],
  i: number,
  lineEnd?: number,
): number {
  const w = words[i];
  if (!w) return 0;
  if (w.end !== undefined && w.end > w.time) return w.end;
  const next = words[i + 1];
  if (next && next.time > w.time) return next.time;
  if (lineEnd !== undefined && lineEnd > w.time) return lineEnd;
  return w.time + LAST_WORD_FALLBACK_SECS;
}

export function wordFill(
  words: Word[],
  i: number,
  t: number,
  lineEnd?: number,
): number {
  const w = words[i];
  if (!w || t <= w.time) return 0;
  const end = wordEndAt(words, i, lineEnd);
  const span = end - w.time;
  if (span <= 0) return 1;
  const fill = (t - w.time) / span;
  return fill < 0 ? 0 : fill > 1 ? 1 : fill;
}

const _synced = $derived(_lines.some((l) => l.time >= 0));

const _activeIndex = $derived(
  _synced
    ? findActiveLineIndex(_lines, getSmoothTime())
    : -1,
);

const _activeWordIndex = $derived(
  _synced && _activeIndex >= 0 && _lines[_activeIndex]?.words
    ? findActiveWordIndex(_lines[_activeIndex]!.words!, getSmoothTime())
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

const DECORATION_ONLY_RE = /^[\s\p{P}\p{S}]+$/u;
const WRAP_OPEN_RE = /^[[({<【（\s]*/;
const WRAP_CLOSE_RE = /[\s:;,.·…~～\-–—_)\]}>）】]*$/;
const INSTRUMENTAL_WORD_RE = new RegExp(
  "^(?:" +
    "instrumental|inst\\.?|interlude|intermission|outro|intro|solo|break|bridge|" +
    "build[\\s._-]?up|fade[\\s._-]?(?:out|in)|prelude|postlude|reprise|" +
    "silence|soundcheck|music" +
    ")$",
  "i",
);
const INSTRUMENTAL_PHRASE_RE = new RegExp(
  "^(?:" +
    "(?:instrumental|inst\\.?|music)[\\s._-]+(?:break|solo|section|pass|part|interlude|intro|outro)|" +
    "(?:guitar|piano|string|sax|synth|drum|flute|violin|cello|trumpet|organ)[\\s._-]*solo|" +
    "no\\s+lyrics?|end\\s+of\\s+song" +
    ")$",
  "i",
);
const instrumentalCache = new Map<string, boolean>();
const INSTRUMENTAL_CACHE_MAX = 500;

export function isInstrumental(text: string): boolean {
  const hit = instrumentalCache.get(text);
  if (hit !== undefined) return hit;

  let result = computeInstrumental(text);
  if (instrumentalCache.size >= INSTRUMENTAL_CACHE_MAX) instrumentalCache.clear();
  instrumentalCache.set(text, result);
  return result;
}

function computeInstrumental(text: string): boolean {
  const raw = text.trim();
  if (!raw) return true;
  if (DECORATION_ONLY_RE.test(raw)) return true;

  const t = raw.replace(WRAP_OPEN_RE, "").replace(WRAP_CLOSE_RE, "").trim();
  if (!t) return true;
  if (DECORATION_ONLY_RE.test(t)) return true;

  return INSTRUMENTAL_WORD_RE.test(t) || INSTRUMENTAL_PHRASE_RE.test(t);
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

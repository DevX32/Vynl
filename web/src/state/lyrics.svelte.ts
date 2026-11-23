import type { LyricsResult } from "@lib/types";
import { vynl } from "@lib/vynl";
import { getCurrentTrack, getCurrentId } from "@state/now-playing.svelte";
import { getSmoothTime } from "@state/media-clock.svelte";

export type Word = {
  time: number;
  text: string;
  end?: number;
  bg?: boolean;
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

type CacheEntry = { result: LyricsResult; lines: LyricLine[] };

const MAX_CACHE = 500;
const LAST_LINE_GRACE_SECS = 10;
const NOMINAL_LINE_SECS = 4;
const MIN_WORD_SECS = 0.12;
const LAST_WORD_FALLBACK_SECS = 0.45;

const STAMP_RE =
  /\[[ \t]*(\d{1,3}):(\d{1,2})(?::(\d{1,4}))?(?:[.,](\d{1,4}))?[ \t]*\]/g;
const HAS_STAMP_RE =
  /\[[ \t]*\d{1,3}:\d{1,2}(?::\d{1,4})?(?:[.,]\d{1,4})?[ \t]*\]/;
const WORD_TAG_RE =
  /<[ \t]*(\d{1,3}):(\d{1,2})(?::(\d{1,4}))?(?:[.,](\d{1,4}))?[ \t]*>/g;
const WORD_TAG_STRIP_RE =
  /<[ \t]*\d{1,3}:\d{1,2}(?::\d{1,4})?(?:[.,]\d{1,4})?[ \t]*>/g;
const METADATA_TAG_RE =
  /^\[(ar|ti|al|au|by|offset|re|ve|length|created|tool|version|application):/i;
const OFFSET_TAG_RE = /^\[offset:\s*([+-]?\d+)\s*\]/i;

let loading = $state(false);
let result = $state<LyricsResult | null>(null);
let lines = $state<LyricLine[]>([]);
let loadError = $state<string | null>(null);

const cache = new Map<string, CacheEntry>();
const embedded = new Set<string>();
let activeId: string | null = null;
let loadingId: string | null = null;
let loadVersion = 0;

function clockSeconds(m: RegExpExecArray): number {
  const a = Number(m[1]);
  const b = Number(m[2]);
  const third = m[3];
  const frac = m[4] ? Number(`0.${m[4]}`) : 0;
  if (!third) return a * 60 + b + frac;
  if (m[4]) return a * 3600 + b * 60 + Number(third) + frac;
  return a * 60 + b + Number(`0.${third}`);
}

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

function cleanLineText(text: string): string {
  return text.replace(WORD_TAG_STRIP_RE, "").replace(/\s+/g, " ").trim();
}

function plainLine(line: string): string {
  return cleanLineText(line.replace(STAMP_RE, ""));
}

function parseEnhancedWords(body: string, offset = 0, lineStart = 0): Word[] | null {
  const stamps: number[] = [];
  WORD_TAG_RE.lastIndex = 0;
  let wm: RegExpExecArray | null;
  while ((wm = WORD_TAG_RE.exec(body))) stamps.push(clockSeconds(wm) + offset);
  if (stamps.length === 0) return null;

  const segments = body.split(WORD_TAG_STRIP_RE);
  const words: Word[] = [];
  const prefix = (segments[0] ?? "").trim();
  if (prefix) words.push({ time: lineStart, text: prefix, sep: false });

  for (let i = 0; i < stamps.length; i++) {
    const seg = segments[i + 1] ?? "";
    const text = seg.trim();
    if (!text) continue;
    const lead = seg.length - seg.trimStart().length;
    const trail = seg.length - seg.trimEnd().length;
    if (lead > 0 && words.length > 0) {
      words[words.length - 1] = { ...words[words.length - 1], sep: true };
    }
    words.push({ time: stamps[i], text, sep: trail > 0 });
  }
  return words.length > 0 ? words : null;
}

function distributeWords(text: string, start: number, end: number): Word[] {
  const parts = text.split(/\s+/).filter(Boolean);
  if (parts.length === 0) return [];
  const slice = Math.max(end - start, MIN_WORD_SECS * parts.length) / parts.length;
  return parts.map((text, i) => ({
    time: start + i * slice,
    text,
    end: start + (i + 1) * slice,
  }));
}

function isChronological(lyricLines: LyricLine[]): boolean {
  let last = -Infinity;
  for (const line of lyricLines) {
    if (line.time < 0) continue;
    if (line.time < last) return false;
    last = line.time;
  }
  return true;
}

function applyEstimatedWords(lyricLines: LyricLine[]): LyricLine[] {
  return lyricLines.map((line, i) => {
    if (line.time < 0 || line.words?.length || isInstrumental(line.text)) return line;
    const next = lyricLines.slice(i + 1).find((candidate) => candidate.time > line.time);
    const end = next ? next.time : line.time + NOMINAL_LINE_SECS;
    const words = distributeWords(line.text, line.time, end);
    return words.length > 0 ? { ...line, end, words } : line;
  });
}

function finalizeLrcLines(lyricLines: LyricLine[]): LyricLine[] {
  if (isChronological(lyricLines)) return applyEstimatedWords(lyricLines);
  const timed = lyricLines.filter((l) => l.time >= 0).sort((a, b) => a.time - b.time);
  const untimed = lyricLines.filter((l) => l.time < 0);
  return applyEstimatedWords([...timed, ...untimed]);
}

function parseLrc(text: string): LyricLine[] {
  const out: LyricLine[] = [];
  const offset = readOffsetSecs(text);

  for (const raw of text.split(/\r?\n/)) {
    if (METADATA_TAG_RE.test(raw.trim())) continue;

    const stamps: number[] = [];
    STAMP_RE.lastIndex = 0;
    let m: RegExpExecArray | null;
    while ((m = STAMP_RE.exec(raw))) stamps.push(clockSeconds(m) + offset);
    const body = raw.replace(STAMP_RE, "").replace(/\s+/g, " ").trim();

    if (stamps.length === 0) {
      const plain = cleanLineText(body);
      if (plain) out.push({ time: -1, text: plain });
      continue;
    }

    const words = parseEnhancedWords(body, offset, stamps[0]) ?? undefined;
    const cleanBody = cleanLineText(body);
    for (const time of stamps) out.push({ time, text: cleanBody, words });
  }

  return finalizeLrcLines(out);
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

function parseTtmlTime(value: string, offset: number, tickRate: number, frameRate: number): number {
  const v = value.trim();
  const tick = /^([+-]?\d+(?:\.\d+)?)t$/.exec(v);
  if (tick) {
    const rate = tickRate > 0 ? tickRate : frameRate > 0 ? frameRate : 1;
    return Number(tick[1]) / rate + offset;
  }
  const m = /^(?:(\d+):)?(\d{1,2}):(\d{1,2})(?:([.:])(\d{1,3}))?$/.exec(v);
  if (m) {
    let frac = 0;
    if (m[5]) {
      frac = m[4] === ":" && frameRate > 0 ? Number(m[5]) / frameRate : Number(`0.${m[5]}`);
    }
    return Number(m[1] ?? 0) * 3600 + Number(m[2]) * 60 + Number(m[3]) + frac + offset;
  }
  const secs = Number(v.replace(/s$/i, ""));
  return (Number.isFinite(secs) ? secs : 0) + offset;
}

function parseOffsetAttr(attr: string): number {
  const clock = /offset\s*=\s*"([^"]+)"/i.exec(attr);
  if (clock) {
    const raw = clock[1].trim();
    const sign = raw.startsWith("-") ? -1 : 1;
    return sign * parseClock(raw.replace(/^[+-]/, ""));
  }
  const secs = /=\s*"?([\d.]+)"?\s*s"?$/i.exec(attr);
  return secs ? Number(secs[1]) : 0;
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
  time: number | null;
  text: string;
  end: number | null;
  bg: boolean;
  sep: boolean;
  roman: boolean;
};

const ROMAN_SPAN_RE = /translat|roman|romaji|translit/i;
const BG_ROLE_RE = /(^|[-_])bg$|background/;

function collectSpans(
  node: Node,
  out: RawSpan[],
  sep: boolean,
  bg: boolean,
  parseTime: (value: string) => number,
  attr: (el: Element, name: string) => string | null,
): boolean {
  let separated = sep;
  for (const child of Array.from(node.childNodes)) {
    if (child.nodeType === 3) {
      const raw = child.textContent ?? "";
      if (!raw) continue;
      const text = raw.replace(/\s+/g, " ").trim();
      if (text) {
        out.push({
          time: null,
          text,
          end: null,
          bg,
          sep: separated || /^\s/.test(raw),
          roman: false,
        });
      }
      separated = /\s$/.test(raw);
      continue;
    }

    const el = child as Element;
    if (el.nodeName === "br") {
      separated = true;
      continue;
    }

    const role = (attr(el, "ttm:role") ?? attr(el, "role") ?? "").toLowerCase();
    const childBg = bg || BG_ROLE_RE.test(role);
    const beginRaw = attr(el, "begin");

    if (el.nodeName === "span" && beginRaw !== null) {
      const raw = decodeEntities(el.textContent ?? "").replace(/\s+/g, " ");
      const text = raw.trim();
      if (!text) {
        separated = false;
        continue;
      }
      const time = parseTime(beginRaw);
      const endRaw = attr(el, "end");
      const endNum = endRaw !== null ? parseTime(endRaw) : null;
      out.push({
        time,
        text,
        end: endNum !== null && endNum > time ? endNum : null,
        bg: childBg,
        sep: separated || /^\s/.test(raw),
        roman: ROMAN_SPAN_RE.test(role),
      });
      separated = /\s$/.test(raw);
      continue;
    }

    separated = collectSpans(el, out, separated, childBg, parseTime, attr);
  }
  return separated;
}

function parseTtml(text: string): LyricLine[] {
  const doc = new DOMParser().parseFromString(text, "application/xml");
  if (doc.querySelector("parsererror")) return [];

  const body = doc.querySelector("body");
  if (!body) return [];

  let offset = 0;
  let frameRate = 0;
  let tickRate = 0;
  const ittp = doc.querySelector("head > ittp, head > metadata > ittp");
  if (ittp) {
    const frame = Number(ittp.getAttribute("ttp:frameRate"));
    const tick = Number(ittp.getAttribute("ttp:tickRate"));
    if (Number.isFinite(frame) && frame > 0) frameRate = frame;
    if (Number.isFinite(tick) && tick > 0) tickRate = tick;
  }

  const timing = body.getAttribute("itunes:timing");
  if (timing) offset += parseOffsetAttr(`offset="${timing}"`);

  const parseTime = (raw: string): number => parseTtmlTime(raw, offset, tickRate, frameRate);
  const attr = (el: Element, name: string): string | null =>
    el.getAttribute(name) ?? el.getAttributeNS("*", name.split(":")[1]);
  const roleOf = (el: Element): string => (attr(el, "ttm:role") ?? "").toLowerCase();

  const out: LyricLine[] = [];
  const romans: Array<{ time: number; text: string }> = [];

  body.querySelectorAll("p").forEach((p) => {
    const beginRaw = attr(p, "begin");
    if (beginRaw === null) return;
    const start = parseTime(beginRaw);
    const endRaw = attr(p, "end");
    const endRawNum = endRaw !== null ? parseTime(endRaw) : null;
    const end = endRawNum !== null && endRawNum > start ? endRawNum : null;
    const role = roleOf(p);
    const romanRole = /translation|romanization|romaji|translit|roman/.test(role);

    const spans: RawSpan[] = [];
    collectSpans(p, spans, false, false, parseTime, attr);
    const usable = spans.filter((s) => !s.roman);
    const inlineRoman = spans
      .filter((s) => s.roman)
      .map((s) => s.text)
      .join(" ")
      .replace(/\s+/g, " ")
      .trim();

    const words: Word[] = [];
    for (const span of usable) {
      const last = words[words.length - 1];
      const time = span.time ?? (last ? (last.end ?? last.time) : start);
      if (span.bg) {
        words.push({
          time,
          text: span.text,
          ...(span.end !== null ? { end: span.end } : {}),
          bg: true,
        });
        continue;
      }
      if (last && !last.bg && !span.sep) {
        words[words.length - 1] = {
          ...last,
          text: last.text + span.text,
          ...(span.end !== null ? { end: span.end } : {}),
        };
        continue;
      }
      words.push({
        time,
        text: span.text,
        ...(span.end !== null ? { end: span.end } : {}),
      });
    }

    const lineText = words.map((w) => w.text).join(" ").replace(/\s+/g, " ").trim();
    if (!lineText) return;

    if (romanRole) {
      romans.push({ time: start, text: lineText });
      return;
    }

    const wordTimed = spans.some((s) => s.time !== null);
    out.push({
      time: start,
      text: lineText,
      ...(wordTimed && words.length > 0 ? { words } : {}),
      ...(end !== null ? { end } : {}),
      ...(BG_ROLE_RE.test(role) ? { bg: true } : {}),
      ...(attr(p, "ttm:agent") ? { agent: attr(p, "ttm:agent")! } : {}),
      ...(inlineRoman ? { roman: inlineRoman } : {}),
    });
  });

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

  if (romans.length > 0) {
    for (const r of romans) {
      let best: LyricLine | null = null;
      let bestDist = Infinity;
      for (const line of out) {
        if (line.roman) continue;
        const d = Math.abs(line.time - r.time);
        if (d < bestDist) {
          bestDist = d;
          best = line;
        }
      }
      if (best && bestDist < 5) best.roman = r.text;
    }
  }

  return finalizeLrcLines(out);
}

function parsePlainText(text: string): LyricLine[] {
  const out: LyricLine[] = [];
  for (const raw of text.split(/\r?\n/)) {
    const line = plainLine(raw);
    if (line) out.push({ time: -1, text: line });
  }
  return out;
}

function isTtmlText(text: string): boolean {
  return /<(?:ttml|tt)(?=[\s>/])/i.test(text);
}

export function lyricsKindOf(text: string): LyricsResult["kind"] {
  if (isTtmlText(text)) return "ttml";
  if (HAS_STAMP_RE.test(text)) return "lrc";
  return "txt";
}

export function toLyricLines(res: LyricsResult): LyricLine[] {
  const kind = res.kind === "txt" ? lyricsKindOf(res.text) : res.kind;
  if (kind === "ttml") return parseTtml(res.text);
  if (kind === "lrc") return parseLrc(res.text);
  return parsePlainText(res.text);
}

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
  if (cache.size <= MAX_CACHE) return;
  const first = cache.keys().next().value;
  if (first !== undefined) cache.delete(first);
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

const DECORATION_ONLY_RE = /^[\s\p{P}\p{S}]+$/u;
const WRAP_OPEN_RE = /^[[({<【（\s]*/;
const WRAP_CLOSE_RE = /[\s:;,.·…~～\-–—_)\]}>）】]*$/;
const INSTRUMENTAL_WORD_RE =
  /^(?:instrumental|inst\.?|interlude|intermission|outro|intro|solo|break|bridge|build[\s._-]?up|fade[\s._-]?(?:out|in)|prelude|postlude|reprise|silence|soundcheck|music)$/i;
const INSTRUMENTAL_PHRASE_RE =
  /^(?:(?:instrumental|inst\.?|music)[\s._-]+(?:break|solo|section|pass|part|interlude|intro|outro)|(?:guitar|piano|string|sax|synth|drum|flute|violin|cello|trumpet|organ)[\s._-]*solo|no\s+lyrics?|end\s+of\s+song)$/i;
const instrumentalCache = new Map<string, boolean>();

export function isInstrumental(text: string): boolean {
  const cached = instrumentalCache.get(text);
  if (cached !== undefined) return cached;
  const raw = text.trim();
  let value = false;
  if (!raw || DECORATION_ONLY_RE.test(raw)) value = true;
  else {
    const t = raw.replace(WRAP_OPEN_RE, "").replace(WRAP_CLOSE_RE, "").trim();
    value =
      !t ||
      DECORATION_ONLY_RE.test(t) ||
      INSTRUMENTAL_WORD_RE.test(t) ||
      INSTRUMENTAL_PHRASE_RE.test(t);
  }
  if (instrumentalCache.size >= 500) instrumentalCache.clear();
  instrumentalCache.set(text, value);
  return value;
}

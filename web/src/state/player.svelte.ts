import { resetShuffleForMode } from "./now-playing.svelte";
import { normalizeSpeed } from "@lib/playback";

export type LoopMode = "off" | "all" | "one";

const LOOP_CYCLE: readonly LoopMode[] = ["off", "all", "one"];

function lsGet(key: string): string | null {
  return typeof localStorage === "undefined" ? null : localStorage.getItem(key);
}

function loadLoop(): LoopMode {
  const raw = lsGet("vynl.loop");
  if (raw === "off" || raw === "all" || raw === "one") return raw;
  if (raw === "1") return "one";
  return "off";
}

function loadVolume(): number {
  const raw = lsGet("vynl.volume");
  if (raw === null) return 0.3;
  const v = Number(raw);
  return v >= 0 && v <= 1 ? v : 0.3;
}

export const CROSSFADE_STEPS: readonly number[] = [0, 3, 5, 8, 12];

const MAX_CROSSFADE_SECS = 12;

function loadCrossfade(): number {
  const raw = lsGet("vynl.crossfade");
  if (raw === null) return 0;
  const v = Number(raw);
  return Number.isFinite(v) && v >= 0 && v <= MAX_CROSSFADE_SECS ? v : 0;
}

function loadSpeed(): number {
  const raw = lsGet("vynl.speed");
  if (raw === null) return 1;
  return normalizeSpeed(Number(raw));
}

function loadReplayGain(): boolean {
  return lsGet("vynl.replayGain") === "1";
}

let _crossfade = $state(loadCrossfade());
let _speed = $state(loadSpeed());
let _replayGain = $state(loadReplayGain());
let _volume = $state(loadVolume());
let _muted = $state(lsGet("vynl.muted") === "1");
let _shuffle = $state(lsGet("vynl.shuffle") === "1");
let _loop = $state<LoopMode>(loadLoop());
let _showLyrics = $state(false);
let _playError = $state<string | null>(null);

export function getCrossfade(): number {
  return _crossfade;
}

export function setCrossfade(secs: number): void {
  const next = Number.isFinite(secs)
    ? Math.min(MAX_CROSSFADE_SECS, Math.max(0, Math.round(secs)))
    : 0;
  if (next === _crossfade) return;
  _crossfade = next;
  localStorage.setItem("vynl.crossfade", String(next));
}

export function getPlaybackSpeed(): number {
  return _speed;
}

let _onSpeedChange: (() => void)[] = [];

export function onSpeedChange(fn: () => void): () => void {
  _onSpeedChange.push(fn);
  return () => {
    _onSpeedChange = _onSpeedChange.filter((f) => f !== fn);
  };
}

export function setPlaybackSpeed(value: number): void {
  const next = normalizeSpeed(value);
  if (next === _speed) return;
  _speed = next;
  localStorage.setItem("vynl.speed", String(next));
  _onSpeedChange.forEach((fn) => fn());
}

export function getReplayGain(): boolean {
  return _replayGain;
}

let _onReplayGainChange: (() => void)[] = [];

export function onReplayGainChange(fn: () => void): () => void {
  _onReplayGainChange.push(fn);
  return () => {
    _onReplayGainChange = _onReplayGainChange.filter((f) => f !== fn);
  };
}

export function setReplayGain(enabled: boolean): void {
  if (enabled === _replayGain) return;
  _replayGain = enabled;
  localStorage.setItem("vynl.replayGain", enabled ? "1" : "0");
  _onReplayGainChange.forEach((fn) => fn());
}

export function getCurrentVolume(): number {
  return _volume;
}
export function getCurrentMuted(): boolean {
  return _muted;
}
export function getCurrentShuffle(): boolean {
  return _shuffle;
}
export function getCurrentLoop(): LoopMode {
  return _loop;
}
export function getLyricsOpen(): boolean {
  return _showLyrics;
}
export function getCurrentPlayError(): string | null {
  return _playError;
}

export function setVolume(v: number): void {
  _volume = v;
  localStorage.setItem("vynl.volume", String(v));
  if (v > 0 && _muted) {
    _muted = false;
    localStorage.setItem("vynl.muted", "0");
  }
}

let _onMuteChange: (() => void)[] = [];
export function onMuteChange(fn: () => void): () => void {
  _onMuteChange.push(fn);
  return () => {
    _onMuteChange = _onMuteChange.filter((f) => f !== fn);
  };
}

export function toggleMute(): void {
  _muted = !_muted;
  localStorage.setItem("vynl.muted", _muted ? "1" : "0");
  _onMuteChange.forEach((fn) => fn());
}

export function setShuffle(enabled: boolean): void {
  if (_shuffle === enabled) return;
  _shuffle = enabled;
  localStorage.setItem("vynl.shuffle", enabled ? "1" : "0");
  if (enabled) resetShuffleForMode();
}

export function toggleShuffle(): void {
  setShuffle(!_shuffle);
}

export function toggleLoop(): void {
  const i = LOOP_CYCLE.indexOf(_loop);
  _loop = LOOP_CYCLE[(i + 1) % LOOP_CYCLE.length];
  localStorage.setItem("vynl.loop", _loop);
}

export function toggleLyrics(): void {
  _showLyrics = !_showLyrics;
}

export function closeLyrics(): void {
  _showLyrics = false;
}

export function setPlayError(e: string | null): void {
  _playError = e;
}

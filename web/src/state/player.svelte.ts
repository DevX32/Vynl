import { resetShuffleForMode } from "./now-playing.svelte";

export type LoopMode = "off" | "all" | "one";

const LOOP_CYCLE: readonly LoopMode[] = ["off", "all", "one"];

function loadLoop(): LoopMode {
  const raw = localStorage.getItem("vynl.loop");
  if (raw === "off" || raw === "all" || raw === "one") return raw;
  if (raw === "1") return "one";
  return "off";
}

function loadVolume(): number {
  const raw = localStorage.getItem("vynl.volume");
  if (raw === null) return 0.3;
  const v = Number(raw);
  return v >= 0 && v <= 1 ? v : 0.3;
}

let _volume = $state(loadVolume());
let _muted = $state(localStorage.getItem("vynl.muted") === "1");
let _shuffle = $state(localStorage.getItem("vynl.shuffle") === "1");
let _loop = $state<LoopMode>(loadLoop());
let _showLyrics = $state(false);
let _playError = $state<string | null>(null);

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
export function onMuteChange(fn: () => void): (() => void) {
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

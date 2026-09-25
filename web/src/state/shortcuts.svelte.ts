import {
  getCurrentVolume,
  toggleMute,
  toggleShuffle,
  toggleLoop,
  toggleLyrics,
} from "@state/player.svelte";
import {
  toggle,
  next,
  prev,
  setVolumeAt,
} from "@lib/player.svelte";
import { closeEq, toggleEq, getEqOpen } from "@state/equalizer.svelte";

interface Shortcut {
  key: string;
  label: string;
  category: string;
}

export const SHORTCUTS: Shortcut[] = [
  { key: "Space", label: "Play / Pause", category: "Playback" },
  { key: "N", label: "Next track", category: "Playback" },
  { key: "P", label: "Previous track", category: "Playback" },
  { key: "M", label: "Mute / Unmute", category: "Volume" },
  { key: "ArrowUp", label: "Volume up", category: "Volume" },
  { key: "ArrowDown", label: "Volume down", category: "Volume" },
  { key: "S", label: "Toggle shuffle", category: "Playback" },
  { key: "R", label: "Toggle loop", category: "Playback" },
  { key: "L", label: "Toggle lyrics", category: "Playback" },
  { key: "F", label: "Fullscreen player", category: "Interface" },
  { key: "E", label: "Equalizer", category: "Interface" },
  { key: "?", label: "Show shortcuts", category: "Interface" },
];

let _showOverlay = $state(false);

export function getShowShortcutsOverlay(): boolean {
  return _showOverlay;
}

function toggleShortcutsOverlay(): void {
  _showOverlay = !_showOverlay;
}

export function closeShortcutsOverlay(): void {
  _showOverlay = false;
}

export function isInputField(element: EventTarget | null): boolean {
  const el = element as HTMLElement | null;
  if (!el) return false;
  const tag = el.tagName;
  return tag === "INPUT" || tag === "TEXTAREA" || tag === "SELECT" || el.isContentEditable;
}

export function handleGlobalKey(e: KeyboardEvent): void {
  if (isInputField(e.target)) return;

  if (e.repeat || e.defaultPrevented) return;

  if (getEqOpen()) {
    if (e.key === "Escape" || e.key.toLowerCase() === "e") {
      e.preventDefault();
      closeEq();
    }
    return;
  }

  if (_showOverlay) {
    if (e.key === "Escape") {
      e.preventDefault();
      closeShortcutsOverlay();
    }
    return;
  }

  const key = e.key.toLowerCase();

  if (key === "?") {
    e.preventDefault();
    toggleShortcutsOverlay();
    return;
  }

  const handler = KEY_HANDLERS[key];
  if (handler) {
    e.preventDefault();
    handler();
  }
}

const KEY_HANDLERS: Record<string, () => void> = {
  " ": () => toggle(),
  n: () => next(),
  p: () => prev(),
  m: () => toggleMute(),
  arrowup: () => setVolumeAt(getCurrentVolume() + 0.05),
  arrowdown: () => setVolumeAt(getCurrentVolume() - 0.05),
  s: () => toggleShuffle(),
  r: () => toggleLoop(),
  l: () => toggleLyrics(),
  f: () => document.dispatchEvent(new CustomEvent("vynl:toggle-fullscreen")),
  e: () => toggleEq(),
};

import { getCurrentVolume, toggleMute } from "@state/player.svelte";
import { toggle, setVolumeAt } from "@lib/player.svelte";
import { closeEq, getEqOpen } from "@state/equalizer.svelte";

interface Shortcut {
  key: string;
  labelKey: string;
  categoryKey: string;
}

export const SHORTCUTS: Shortcut[] = [
  {
    key: "Space",
    labelKey: "shortcuts.actions.playPause",
    categoryKey: "shortcuts.categories.playback",
  },
  {
    key: "M",
    labelKey: "shortcuts.actions.muteUnmute",
    categoryKey: "shortcuts.categories.volume",
  },
  {
    key: "ArrowUp",
    labelKey: "shortcuts.actions.volumeUp",
    categoryKey: "shortcuts.categories.volume",
  },
  {
    key: "ArrowDown",
    labelKey: "shortcuts.actions.volumeDown",
    categoryKey: "shortcuts.categories.volume",
  },
  {
    key: "F",
    labelKey: "shortcuts.actions.fullscreenPlayer",
    categoryKey: "shortcuts.categories.interface",
  },
];

let _showOverlay = $state(false);

export function getShowShortcutsOverlay(): boolean {
  return _showOverlay;
}

function openShortcutsOverlay(): void {
  _showOverlay = true;
}

export function closeShortcutsOverlay(): void {
  _showOverlay = false;
}

export function isInputField(element: EventTarget | null): boolean {
  const el = element as HTMLElement | null;
  if (!el) return false;
  const tag = el.tagName;
  return (
    tag === "INPUT" ||
    tag === "TEXTAREA" ||
    tag === "SELECT" ||
    el.isContentEditable
  );
}

export function handleGlobalKey(e: KeyboardEvent): void {
  if (isInputField(e.target)) return;

  if (e.repeat || e.defaultPrevented) return;

  if (getEqOpen()) {
    if (e.key === "Escape") {
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
    openShortcutsOverlay();
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
  m: () => toggleMute(),
  arrowup: () => setVolumeAt(getCurrentVolume() + 0.05),
  arrowdown: () => setVolumeAt(getCurrentVolume() - 0.05),
  f: () => document.dispatchEvent(new CustomEvent("vynl:toggle-fullscreen")),
};

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
    key: "?",
    labelKey: "shortcuts.actions.showShortcuts",
    categoryKey: "shortcuts.categories.interface",
  },
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

export function isInteractiveTarget(element: EventTarget | null): boolean {
  const el = element as HTMLElement | null;
  if (!el || typeof el.closest !== "function") return false;
  return (
    isInputField(el) ||
    el.closest(
      'button, [role="menuitem"], [role="menuitemcheckbox"], [role="option"], [role="slider"], [role="switch"], [role="tab"], a[href], summary',
    ) !== null
  );
}

export function handleGlobalKey(e: KeyboardEvent): void {
  if (e.repeat || e.defaultPrevented) return;
  if (e.ctrlKey || e.metaKey || e.altKey) return;
  if (isInteractiveTarget(e.target)) return;

  const key = e.key.toLowerCase();

  if (key === "?") {
    e.preventDefault();
    _showOverlay = !_showOverlay;
    return;
  }

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

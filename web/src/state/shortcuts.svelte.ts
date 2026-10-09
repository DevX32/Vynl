import { getCurrentVolume, toggleMute } from "@state/player.svelte";
import { toggle, setVolumeAt } from "@lib/player.svelte";
import { closeEq, getEqOpen } from "@state/equalizer.svelte";

export interface KeyBinding {
  key: string;
  ctrl?: boolean;
  meta?: boolean;
  alt?: boolean;
  shift?: boolean;
}

export interface KeybindAction {
  id: string;
  labelKey: string;
  categoryKey: string;
  binding: KeyBinding;
  run: () => void;
}

const VOLUME_STEP = 0.05;

const KEY_ALIASES: Readonly<Record<string, string>> = {
  " ": "space",
  spacebar: "space",
  esc: "escape",
  left: "arrowleft",
  right: "arrowright",
  up: "arrowup",
  down: "arrowdown",
};

const NON_COMMAND_KEYS = new Set([
  "control",
  "meta",
  "alt",
  "shift",
  "capslock",
  "numlock",
  "scrolllock",
  "altgraph",
  "os",
  "dead",
  "unidentified",
  "process",
]);

const SHIFT_IMPLIED_KEYS = new Set([
  "!", '"', "#", "$", "%", "&", "'", "(", ")", "*", "+", ":", ";", "<", "=",
  ">", "?", "@", "[", "\\", "]", "^", "_", "`", "{", "|", "}", "~",
]);

const KEY_LABELS: Readonly<Record<string, string>> = {
  space: "Space",
  arrowup: "↑",
  arrowdown: "↓",
  arrowleft: "←",
  arrowright: "→",
  escape: "Esc",
  enter: "Enter",
  tab: "Tab",
  backspace: "Backspace",
  delete: "Delete",
};

function normalizeKey(key: string): string {
  const lower = key.toLowerCase();
  return KEY_ALIASES[lower] ?? lower;
}

interface ComboModifiers {
  ctrl: boolean;
  meta: boolean;
  alt: boolean;
  shift: boolean;
}

function comboId(mods: ComboModifiers, key: string): string {
  const parts: string[] = [];
  if (mods.ctrl || mods.meta) parts.push("mod");
  if (mods.alt) parts.push("alt");
  if (mods.shift) parts.push("shift");
  parts.push(key);
  return parts.join("+");
}

function bindingModifiers(b: KeyBinding, shift: boolean): ComboModifiers {
  return { ctrl: b.ctrl === true, meta: b.meta === true, alt: b.alt === true, shift };
}

function bindingIds(binding: KeyBinding): string[] {
  const key = normalizeKey(binding.key);
  const shift = binding.shift === true;
  const ids = [comboId(bindingModifiers(binding, shift), key)];
  if (SHIFT_IMPLIED_KEYS.has(key)) {
    const flipped = !shift;
    ids.push(comboId(bindingModifiers(binding, flipped), key));
  }
  return ids;
}

function eventId(e: KeyboardEvent): string | null {
  const key = normalizeKey(e.key);
  if (!key || NON_COMMAND_KEYS.has(key)) return null;
  return comboId(
    {
      ctrl: e.ctrlKey,
      meta: e.metaKey,
      alt: e.altKey,
      shift: e.shiftKey && !SHIFT_IMPLIED_KEYS.has(key),
    },
    key,
  );
}

export const SHORTCUT_CATEGORIES = [
  "shortcuts.categories.playback",
  "shortcuts.categories.volume",
  "shortcuts.categories.interface",
] as const;

const ACTIONS: readonly KeybindAction[] = [
  {
    id: "playPause",
    labelKey: "shortcuts.actions.playPause",
    categoryKey: "shortcuts.categories.playback",
    binding: { key: "Space" },
    run: () => toggle(),
  },
  {
    id: "volumeUp",
    labelKey: "shortcuts.actions.volumeUp",
    categoryKey: "shortcuts.categories.volume",
    binding: { key: "ArrowUp" },
    run: () => setVolumeAt(getCurrentVolume() + VOLUME_STEP),
  },
  {
    id: "volumeDown",
    labelKey: "shortcuts.actions.volumeDown",
    categoryKey: "shortcuts.categories.volume",
    binding: { key: "ArrowDown" },
    run: () => setVolumeAt(getCurrentVolume() - VOLUME_STEP),
  },
  {
    id: "toggleMute",
    labelKey: "shortcuts.actions.muteUnmute",
    categoryKey: "shortcuts.categories.volume",
    binding: { key: "m" },
    run: () => toggleMute(),
  },
  {
    id: "showShortcuts",
    labelKey: "shortcuts.actions.showShortcuts",
    categoryKey: "shortcuts.categories.interface",
    binding: { key: "?" },
    run: () => toggleShortcutsOverlay(),
  },
  {
    id: "fullscreenPlayer",
    labelKey: "shortcuts.actions.fullscreenPlayer",
    categoryKey: "shortcuts.categories.interface",
    binding: { key: "f" },
    run: () => document.dispatchEvent(new CustomEvent("vynl:toggle-fullscreen")),
  },
];

function buildIndex(): ReadonlyMap<string, KeybindAction> {
  const index = new Map<string, KeybindAction>();
  const seen = new Map<string, string>();
  for (const action of ACTIONS) {
    for (const id of bindingIds(action.binding)) {
      const previous = seen.get(id);
      if (previous !== undefined) {
        console.warn(
          `[shortcuts] "${action.id}" binds ${id}, already bound by "${previous}"`,
        );
        continue;
      }
      seen.set(id, action.id);
      index.set(id, action);
    }
  }
  return index;
}

const ACTION_INDEX = buildIndex();

export function getShortcutActions(): readonly KeybindAction[] {
  return ACTIONS;
}

export function bindingParts(binding: KeyBinding): string[] {
  const key = normalizeKey(binding.key);
  const parts: string[] = [];
  if (binding.ctrl === true || binding.meta === true) parts.push("Ctrl");
  if (binding.alt === true) parts.push("Alt");
  if (binding.shift === true) parts.push("Shift");
  parts.push(KEY_LABELS[key] ?? (key.length === 1 ? key.toUpperCase() : key));
  return parts;
}

export function formatBinding(binding: KeyBinding): string {
  return bindingParts(binding).join(" + ");
}

let _showOverlay = $state(false);

export function getShowShortcutsOverlay(): boolean {
  return _showOverlay;
}

export function toggleShortcutsOverlay(): void {
  _showOverlay = !_showOverlay;
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

const SLIDER_OWNED_KEYS = new Set([
  "arrowup",
  "arrowdown",
  "arrowleft",
  "arrowright",
  "home",
  "end",
  "pageup",
  "pagedown",
]);

const ACTIVATION_KEYS = new Set(["space", "enter"]);

const NAVIGATION_ROLES =
  '[role="menuitem"], [role="menuitemcheckbox"], [role="menuitemradio"], [role="option"], [role="tab"], [role="switch"], [role="combobox"]';

export function shouldYieldToTarget(e: KeyboardEvent): boolean {
  if (isInputField(e.target)) return true;
  const el = e.target as HTMLElement | null;
  if (!el || typeof el.closest !== "function") return false;
  if (el.closest('[role="slider"]') !== null) {
    return SLIDER_OWNED_KEYS.has(normalizeKey(e.key));
  }
  if (el.closest(NAVIGATION_ROLES) !== null) return true;
  if (!isInteractiveTarget(el)) return false;
  return ACTIVATION_KEYS.has(normalizeKey(e.key));
}

export function handleGlobalKey(e: KeyboardEvent): void {
  if (e.repeat || e.defaultPrevented) return;

  if (e.key === "Escape") {
    if (getEqOpen()) {
      e.preventDefault();
      closeEq();
      return;
    }
    if (_showOverlay) {
      e.preventDefault();
      closeShortcutsOverlay();
    }
    return;
  }

  if (shouldYieldToTarget(e)) return;

  const id = eventId(e);
  if (id === null) return;
  const action = ACTION_INDEX.get(id);
  if (!action) return;

  if (getEqOpen() || _showOverlay) {
    if (action.id !== "showShortcuts") return;
  }

  e.preventDefault();
  action.run();
}

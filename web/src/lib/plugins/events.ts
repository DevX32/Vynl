
import type { PLUGIN_EVENTS } from "./types";

type Listener = (payload: unknown) => void;

type AppPluginEvent = (typeof PLUGIN_EVENTS)[number];

interface OwnedListener {
  pluginId: string;
  fn: Listener;
}

const listeners = new Map<string, Set<OwnedListener>>();

export function addPluginEventListener(
  pluginId: string,
  event: string,
  fn: Listener,
): () => void {
  let set = listeners.get(event);
  if (!set) {
    set = new Set();
    listeners.set(event, set);
  }
  const owned: OwnedListener = { pluginId, fn };
  set.add(owned);
  return () => {
    const current = listeners.get(event);
    if (!current) return;
    current.delete(owned);
    if (current.size === 0) listeners.delete(event);
  };
}

export function removeListenersForPlugin(pluginId: string): void {
  for (const [event, set] of listeners) {
    for (const owned of [...set]) {
      if (owned.pluginId === pluginId) set.delete(owned);
    }
    if (set.size === 0) listeners.delete(event);
  }
}

function dispatch(event: string, payload: unknown, onlyPlugin?: string): void {
  const set = listeners.get(event);
  if (!set) return;
  for (const owned of [...set]) {
    if (onlyPlugin !== undefined && owned.pluginId !== onlyPlugin) continue;
    try {
      owned.fn(payload);
    } catch (e) {
      console.warn(
        `[plugins] listener for "${event}" (plugin ${owned.pluginId}) failed:`,
        e,
      );
    }
  }
}

export function emitAppEvent(event: AppPluginEvent, payload?: unknown): void {
  dispatch(event, payload);
}

export function emitScopedEvent(
  pluginId: string,
  event: string,
  payload?: unknown,
): void {
  dispatch(event, payload, pluginId);
}

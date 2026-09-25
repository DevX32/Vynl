
import { convertFileSrc } from "@tauri-apps/api/core";
import {
  getCurrentTime,
  getCurrentTrack,
} from "@state/now-playing.svelte";
import { getLyricsResult } from "@state/lyrics.svelte";
import {
  isSharedPlayback,
  pausePlayback,
  playShared as hostPlayShared,
  resumeOrPlay,
  seekToTime,
  stopShared as hostStopShared,
} from "@lib/player.svelte";
import type {
  PluginPage,
  PluginPageMount,
  PluginPageRecord,
  PluginPlayerState,
  PluginSharedState,
} from "./types";
import { vynl } from "@lib/vynl";
import {
  addPluginEventListener,
  emitScopedEvent,
  removeListenersForPlugin,
} from "./events";
import {
  getProvider,
  listProviders,
  registerProvider,
  subscribeProviders,
  unregisterProvider,
} from "./providers";
import {
  PLUGIN_API_VERSION,
  PLUGIN_PERMISSIONS,
  type PluginHomeSection,
  type PluginHttpInit,
  type PluginManifest,
  type PluginProvider,
  type PluginProviderKind,
  type PluginSettingsSection,
  type VynlPlugin,
  type VynlPluginAPI,
} from "./types";

export interface PluginHost {
  id: string;
  name: string;
  emitChange: () => void;
  getConfig: () => Record<string, unknown>;
  setConfig: (next: Record<string, unknown>) => Promise<void>;
}

export interface PluginRuntime {
  api: VynlPluginAPI;
  plugin: VynlPlugin;
  dispose: () => void;
  settingsSections: PluginSettingsSection[];
  homeSections: PluginHomeSection[];
  getPage: () => PluginPageRecord | null;
}

const knownPermissions = new Set<string>(PLUGIN_PERMISSIONS);

function validateSection(
  section: PluginSettingsSection | PluginHomeSection,
): void {
  if (!section.id || typeof section.id !== "string") {
    throw new Error("Section id must be a non-empty string");
  }
}

export async function createRuntime(
  host: PluginHost,
  plugin: VynlPlugin,
  manifest: PluginManifest,
): Promise<PluginRuntime> {
  const permissions = (manifest.vynl?.permissions ?? []).filter((p) =>
    knownPermissions.has(p),
  );
  const requirePermission = (required: string): void => {
    if (!permissions.includes(required)) {
      throw new Error(
        `Plugin does not have the "${required}" permission declared in package.json`,
      );
    }
  };

  const disposers: Array<() => void> = [];
  const settingsSections: PluginSettingsSection[] = [];
  const homeSections: PluginHomeSection[] = [];
  let pluginPage: PluginPageRecord | null = null;

  function addSettingsSection(section: PluginSettingsSection): string {
    validateSection(section);
    const stored: PluginSettingsSection = {
      ...section,
      pluginId: host.id,
      pluginName: host.name,
    };
    const idx = settingsSections.findIndex((s) => s.id === section.id);
    if (idx >= 0) settingsSections[idx] = stored;
    else settingsSections.push(stored);
    host.emitChange();
    disposers.push(() => {
      const i = settingsSections.findIndex((s) => s.id === section.id);
      if (i >= 0) {
        settingsSections.splice(i, 1);
        host.emitChange();
      }
    });
    return section.id;
  }

  const api: VynlPluginAPI = {
    pluginId: host.id,
    permissions: Object.freeze([...permissions]),
    apiVersion: PLUGIN_API_VERSION,

    Settings: {
      register(section) {
        addSettingsSection(section);
      },
      async get(key) {
        return host.getConfig()[key];
      },
      async set(key, value) {
        const next = { ...host.getConfig(), [key]: value };
        await host.setConfig(next);
        emitScopedEvent(host.id, pluginScopeKey(host.id, key), value);
      },
      subscribe(key, cb) {
        const off = addPluginEventListener(
          host.id,
          pluginScopeKey(host.id, key),
          cb as (payload: unknown) => void,
        );
        disposers.push(off);
        return off;
      },
    },

    Providers: {
      register<T extends PluginProvider>(provider: T): string {
        if (!provider.id || typeof provider.id !== "string") {
          throw new Error("Provider id must be a non-empty string");
        }
        registerProvider(host.id, provider);
        disposers.push(() => unregisterProvider(host.id, provider.id));
        host.emitChange();
        return provider.id;
      },
      unregister(id: string): boolean {
        const ok = unregisterProvider(host.id, id);
        host.emitChange();
        return ok;
      },
      list(kind?: PluginProviderKind) {
        return listProviders(kind);
      },
      get<T extends PluginProvider>(id: string, kind: PluginProviderKind) {
        return getProvider<T>(id, kind);
      },
      subscribe(cb: () => void) {
        const off = subscribeProviders(cb);
        disposers.push(off);
        return off;
      },
    },

    UI: {
      registerSettingsSection(section) {
        return addSettingsSection(section);
      },
      registerHomeSection(section): string {
        validateSection(section);
        const stored: PluginHomeSection = {
          ...section,
          pluginId: host.id,
          pluginName: host.name,
        };
        const idx = homeSections.findIndex((s) => s.id === section.id);
        if (idx >= 0) homeSections[idx] = stored;
        else homeSections.push(stored);
        host.emitChange();
        disposers.push(() => {
          const i = homeSections.findIndex((s) => s.id === section.id);
          if (i >= 0) {
            homeSections.splice(i, 1);
            host.emitChange();
          }
        });
        return section.id;
      },
      unregisterSettingsSection(id: string): void {
        const i = settingsSections.findIndex((s) => s.id === id);
        if (i >= 0) {
          settingsSections.splice(i, 1);
          host.emitChange();
        }
      },
      unregisterHomeSection(id: string): void {
        const i = homeSections.findIndex((s) => s.id === id);
        if (i >= 0) {
          homeSections.splice(i, 1);
          host.emitChange();
        }
      },
      registerPage(page: PluginPage, mount: PluginPageMount): string {
        if (!page || typeof page.title !== "string" || page.title.trim() === "") {
          throw new Error("Page title must be a non-empty string");
        }
        if (typeof mount !== "function") {
          throw new Error("registerPage requires a mount function");
        }
        pluginPage = {
          ...page,
          pluginId: host.id,
          pluginName: host.name,
          mount,
        };
        host.emitChange();
        disposers.push(() => {
          if (pluginPage !== null) {
            pluginPage = null;
            host.emitChange();
          }
        });
        return `plugin:${host.id}`;
      },
      unregisterPage(): void {
        if (pluginPage !== null) {
          pluginPage = null;
          host.emitChange();
        }
      },
    },

    Player: {
      getState(): PluginPlayerState {
        const np = getCurrentTrack();
        let lyrics: PluginPlayerState["lyrics"] = null;
        const lr = getLyricsResult();
        if (lr?.text) {
          lyrics = { text: lr.text, kind: lr.kind };
        } else if (np?.lyrics?.trim()) {
          const text = np.lyrics;
          lyrics = { text, kind: /\[\d{1,2}:\d{2}/.test(text) ? "lrc" : "txt" };
        }
        return {
          playing: np?.playing ?? false,
          position: getCurrentTime(),
          shared: isSharedPlayback(),
          lyrics,
          track: np
            ? {
                id: np.id,
                title: np.title,
                artist: np.artist,
                album: np.album,
                duration: np.duration,
                path: np.path,
              }
            : null,
        };
      },
      isShared(): boolean {
        return isSharedPlayback();
      },
      play(): void {
        requirePermission("player");
        const np = getCurrentTrack();
        if (!isSharedPlayback() || !np?.path) return;
        void resumeOrPlay(np.path, getCurrentTime() || 0).catch((e) =>
          console.warn(`[plugin:${host.id}] play failed:`, e),
        );
      },
      pause(): void {
        requirePermission("player");
        if (!isSharedPlayback()) return;
        pausePlayback();
      },
      seek(seconds: number): void {
        requirePermission("player");
        if (!isSharedPlayback()) return;
        if (!Number.isFinite(seconds)) return;
        seekToTime(Math.max(0, seconds));
      },
      async playShared(state: PluginSharedState): Promise<void> {
        requirePermission("player");
        if (
          typeof state?.coverUrl === "string" &&
          !/^https?:/i.test(state.coverUrl)
        ) {
          throw new Error("coverUrl must be an http(s) URL");
        }
        await hostPlayShared(state);
      },
      stopShared(): void {
        requirePermission("player");
        hostStopShared();
      },
      async getLocalFileUrl(kind: "audio" | "cover"): Promise<string | null> {
        requirePermission("player");
        const np = getCurrentTrack();
        const path = kind === "audio" ? np?.path : np?.cover;
        if (!path || /^(https?:|blob:|data:|asset:)/i.test(path)) return null;
        try {
          const validated = await vynl.pluginValidateLocalMedia(path, kind);
          return convertFileSrc(validated);
        } catch {
          return null;
        }
      },
      async clearAudioCache(): Promise<void> {
        requirePermission("player");
        await vynl.clearAudioCache();
      },
    },

    Http: {
      async fetch(url: string, init?: PluginHttpInit) {
        requirePermission("network");
        return vynl.pluginHttpFetch(url, init ?? null);
      },
    },

    Shell: {
      async openExternal(url: string) {
        requirePermission("shell");
        if (!/^(https?:|mailto:)/i.test(url)) {
          throw new Error("Only http(s) and mailto URLs can be opened");
        }
        await vynl.openExternal(url);
      },
    },

    Events: {
      on(event: string, cb: (payload: unknown) => void) {
        const off = addPluginEventListener(host.id, event, cb);
        disposers.push(off);
        return off;
      },
    },

    Logger: {
      debug: (m: string) => console.debug(`[plugin:${host.id}]`, m),
      info: (m: string) => console.info(`[plugin:${host.id}]`, m),
      warn: (m: string) => console.warn(`[plugin:${host.id}]`, m),
      error: (m: string) => console.error(`[plugin:${host.id}]`, m),
    },
  };

  const dispose = () => {
    for (const d of disposers.splice(0)) {
      try {
        d();
      } catch (e) {
        console.warn(`[plugin:${host.id}] disposer failed:`, e);
      }
    }
    removeListenersForPlugin(host.id);
    settingsSections.length = 0;
    homeSections.length = 0;
    pluginPage = null;
    host.emitChange();
  };

  return {
    api,
    plugin,
    dispose,
    settingsSections,
    homeSections,
    getPage: () => pluginPage,
  };
}

export function pluginScopeKey(pluginId: string, key: string): string {
  return `settings:${pluginId}:${key}`;
}

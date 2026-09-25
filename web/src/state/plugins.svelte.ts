import { vynl } from "@lib/vynl";
import { t } from "@lib/i18n";
import { toasts } from "@lib/toast";
import fallbackCatalog from "@lib/plugins/fallback-catalog.json";
import { emitAppEvent, emitScopedEvent } from "@lib/plugins/events";
import {
  createRuntime,
  pluginScopeKey,
  type PluginHost,
  type PluginRuntime,
} from "@lib/plugins/api";
import { loadPluginSource, evaluatePlugin } from "@lib/plugins/loader";
import { clearCompilerCache } from "@lib/plugins/compiler";
import {
  PLUGIN_API_VERSION,
  type PluginEntry,
  type PluginHomeSection,
  type PluginManifest,
  type PluginPageRecord,
  type PluginSettingsSection,
  type StorePlugin,
} from "@lib/plugins/types";
import { getCurrentSettings } from "./settings.svelte";
import { getCurrentTrack, getIsTrackPlaying } from "./now-playing.svelte";

const DEFAULT_STORE_SOURCES = [
  "https://raw.githubusercontent.com/DevX32/vynl-store/main/plugins.json",
];

const STORE_TTL_MS = 5 * 60 * 1000;


let _installed = $state<PluginEntry[]>([]);
let _store = $state<StorePlugin[]>([]);
let _storeLoading = $state(false);
let _storeError = $state<string | null>(null);
let _errors = $state<Record<string, string>>({});
let _busy = $state<Record<string, boolean>>({});
let _settingsSections = $state<PluginSettingsSection[]>([]);
let _homeSections = $state<PluginHomeSection[]>([]);
let _pluginPages = $state<PluginPageRecord[]>([]);
let _configs = $state<Record<string, Record<string, unknown>>>({});

interface LoadedPlugin {
  runtime: PluginRuntime;
  entry: PluginEntry;
}

const loaded = new Map<string, LoadedPlugin>();
const loading = new Set<string>();


export function getInstalledPlugins(): PluginEntry[] {
  return _installed;
}

export function getStorePlugins(): StorePlugin[] {
  return _store;
}

export function getStoreLoading(): boolean {
  return _storeLoading;
}

export function getStoreError(): string | null {
  return _storeError;
}

export function getPluginError(id: string): string | null {
  return _errors[id] ?? null;
}

export function isPluginBusy(id: string): boolean {
  return _busy[id] ?? false;
}

export function getPluginSettingsSections(): PluginSettingsSection[] {
  return _settingsSections;
}

export function getPluginHomeSections(): PluginHomeSection[] {
  return _homeSections;
}

export function getPluginPages(): PluginPageRecord[] {
  return _pluginPages;
}

export function getPluginConfig(id: string): Record<string, unknown> {
  return _configs[id] ?? {};
}


function errText(e: unknown): string {
  return e instanceof Error ? e.message : String(e);
}

function upsert(list: PluginEntry[], entry: PluginEntry): PluginEntry[] {
  const idx = list.findIndex((p) => p.id === entry.id);
  if (idx < 0) return [...list, entry];
  const next = [...list];
  next[idx] = entry;
  return next;
}

function syncSections(): void {
  const settings: PluginSettingsSection[] = [];
  const home: PluginHomeSection[] = [];
  const pages: PluginPageRecord[] = [];
  for (const rec of loaded.values()) {
    settings.push(...rec.runtime.settingsSections);
    home.push(...rec.runtime.homeSections);
    const page = rec.runtime.getPage();
    if (page !== null) pages.push(page);
  }
  _settingsSections = settings;
  _homeSections = home;
  _pluginPages = pages;
}

function makeDeps(entry: PluginEntry, manifest: PluginManifest): PluginHost {
  return {
    id: entry.id,
    name: manifest.vynl?.displayName ?? entry.displayName ?? manifest.name,
    emitChange: () => syncSections(),
    getConfig: () => _configs[entry.id] ?? {},
    setConfig: async (next: Record<string, unknown>) => {
      _configs[entry.id] = next;
      await vynl.pluginSetConfig(entry.id, next);
    },
  };
}

async function runHook(
  id: string,
  label: string,
  fn: () => void | Promise<void>,
): Promise<void> {
  try {
    await fn();
  } catch (e) {
    console.warn(`[plugins] "${id}" ${label} hook failed:`, e);
  }
}


async function loadOne(entry: PluginEntry): Promise<void> {
  if (loaded.has(entry.id) || loading.has(entry.id)) return;
  loading.add(entry.id);
  try {
    let config: Record<string, unknown> = {};
    try {
      const raw = (await vynl.pluginGetConfig(entry.id)) as unknown;
      if (raw && typeof raw === "object" && !Array.isArray(raw)) {
        config = raw as Record<string, unknown>;
      }
    } catch {}
    _configs[entry.id] = config;

    const { manifest, code } = await loadPluginSource(entry);
    const plugin = await evaluatePlugin(code);
    const runtime = await createRuntime(makeDeps(entry, manifest), plugin, manifest);
    loaded.set(entry.id, { runtime, entry });
    await runHook(entry.id, "onLoad", () => runtime.plugin.onLoad?.(runtime.api));
    if (entry.enabled) {
      await runHook(entry.id, "onEnable", () =>
        runtime.plugin.onEnable?.(runtime.api),
      );
    }
    syncSections();
    if (_errors[entry.id]) delete _errors[entry.id];
  } catch (e) {
    _errors[entry.id] = errText(e);
    console.warn(`[plugins] failed to load "${entry.id}":`, e);
    const partial = loaded.get(entry.id);
    if (partial) {
      partial.runtime.dispose();
      loaded.delete(entry.id);
    }
    syncSections();
  } finally {
    loading.delete(entry.id);
  }
}

interface UnloadOpts {
  unload: boolean;
  disable: boolean;
}

async function unloadOne(id: string, opts: UnloadOpts): Promise<void> {
  const rec = loaded.get(id);
  if (!rec) return;
  loaded.delete(id);
  const { runtime } = rec;
  if (opts.unload) {
    await runHook(id, "onUnload", () => runtime.plugin.onUnload?.(runtime.api));
  }
  if (opts.disable) {
    await runHook(id, "onDisable", () => runtime.plugin.onDisable?.(runtime.api));
  }
  runtime.dispose();
  syncSections();
}


let _initialized = false;
let _stopEvents: (() => void) | null = null;

export async function initPlugins(): Promise<void> {
  if (_initialized) return;
  _initialized = true;

  globalThis.__VYNL_PLUGIN_SDK__ = { PLUGIN_API_VERSION };

  try {
    _installed = await vynl.pluginsList();
  } catch (e) {
    console.warn("[plugins] failed to read plugin registry:", e);
    _installed = [];
  }

  for (const entry of _installed) {
    if (entry.enabled) await loadOne(entry);
  }

  startEventBridge();
  void checkPluginUpdates(true).catch((e) =>
    console.warn("[plugins] auto-update check failed:", e),
  );
}

function startEventBridge(): void {
  if (_stopEvents) return;
  _stopEvents = $effect.root(() => {
    let lastTrackId: string | null = null;
    let lastPlaying = false;
    $effect(() => {
      const track = getCurrentTrack();
      const playing = getIsTrackPlaying();
      const id = track?.id ?? null;
      if (id !== lastTrackId) {
        lastTrackId = id;
        emitAppEvent(
          "track-changed",
          track
            ? {
                id: track.id,
                title: track.title,
                artist: track.artist,
                album: track.album,
                duration: track.duration,
                path: track.path,
              }
            : null,
        );
      }
      if (playing !== lastPlaying) {
        lastPlaying = playing;
        emitAppEvent("playback-changed", { playing, id });
      }
    });
  });
}


export async function setPluginEnabled(id: string, enabled: boolean): Promise<void> {
  const entry = _installed.find((p) => p.id === id);
  if (!entry) return;
  try {
    const updated = await vynl.pluginsSetEnabled(id, enabled);
    _installed = upsert(_installed, updated);
    if (enabled) {
      const rec = loaded.get(id);
      if (rec) {
        await runHook(id, "onEnable", () =>
          rec.runtime.plugin.onEnable?.(rec.runtime.api),
        );
        syncSections();
      } else {
        await loadOne(updated);
      }
    } else {
      await unloadOne(id, { unload: false, disable: true });
    }
  } catch (e) {
    toasts.error(
      t(enabled ? "plugins.enableFailed" : "plugins.disableFailed", {
        name: entry.displayName ?? id,
        error: errText(e),
      }),
    );
  }
}

export async function removePlugin(id: string): Promise<void> {
  const entry = _installed.find((p) => p.id === id);
  if (!entry) return;
  const name = entry.displayName ?? entry.id;
  _busy[id] = true;
  try {
    await unloadOne(id, { unload: true, disable: true });
    await vynl.pluginsRemove(id);
    _installed = _installed.filter((p) => p.id !== id);
    delete _configs[id];
    delete _errors[id];
    toasts.success(t("plugins.removed", { name }));
  } catch (e) {
    toasts.error(t("plugins.removeFailed", { name, error: errText(e) }));
    if (entry.enabled) await loadOne(entry);
  } finally {
    delete _busy[id];
  }
}

export async function reloadDevPlugin(id: string): Promise<void> {
  const entry = _installed.find((p) => p.id === id);
  if (!entry) return;
  const name = entry.displayName ?? entry.id;
  _busy[id] = true;
  try {
    await unloadOne(id, { unload: true, disable: true });
    clearCompilerCache();
    const updated = await vynl.pluginsReloadDev(id);
    _installed = upsert(_installed, updated);
    delete _errors[id];
    if (updated.enabled) await loadOne(updated);
    toasts.success(t("plugins.reloaded", { name }));
  } catch (e) {
    toasts.error(t("plugins.reloadFailed", { name, error: errText(e) }));
    if (entry.enabled) await loadOne(entry);
  } finally {
    delete _busy[id];
  }
}


async function finishInstall(entry: PluginEntry): Promise<void> {
  if (loaded.has(entry.id)) {
    await unloadOne(entry.id, { unload: true, disable: true });
  }
  _installed = upsert(_installed, entry);
  delete _errors[entry.id];
  if (entry.enabled) await loadOne(entry);
}

export async function installFromStore(sp: StorePlugin): Promise<void> {
  if (!sp.downloadUrl) {
    toasts.error(
      t("plugins.installFailed", { name: sp.name, error: "no download URL" }),
    );
    return;
  }
  const wasInstalled = _installed.some((p) => p.id === sp.id);
  _busy[sp.id] = true;
  try {
    const entry = await vynl.pluginsInstallFromUrl(sp.downloadUrl, sp.repo);
    await finishInstall(entry);
    toasts.success(
      t(wasInstalled ? "plugins.updated" : "plugins.installed", {
        name: entry.displayName ?? sp.name,
      }),
    );
  } catch (e) {
    toasts.error(t("plugins.installFailed", { name: sp.name, error: errText(e) }));
  } finally {
    delete _busy[sp.id];
  }
}

export async function installDevFolder(): Promise<void> {
  let path: string | null;
  try {
    path = await vynl.pluginsPickFolder();
  } catch (e) {
    toasts.error(t("plugins.pickFailed", { error: errText(e) }));
    return;
  }
  if (!path) return; 
  try {
    const entry = await vynl.pluginsInstallFromFolder(path);
    const wasInstalled = _installed.some((p) => p.id === entry.id);
    await finishInstall(entry);
    toasts.success(
      t(wasInstalled ? "plugins.updated" : "plugins.installed", {
        name: entry.displayName ?? entry.id,
      }),
    );
  } catch (e) {
    toasts.error(t("plugins.installFailed", { name: path, error: errText(e) }));
  }
}

export async function installZip(): Promise<void> {
  let path: string | null;
  try {
    path = await vynl.pluginsPickZip();
  } catch (e) {
    toasts.error(t("plugins.pickFailed", { error: errText(e) }));
    return;
  }
  if (!path) return; 
  try {
    const entry = await vynl.pluginsInstallFromZip(path);
    const wasInstalled = _installed.some((p) => p.id === entry.id);
    await finishInstall(entry);
    toasts.success(
      t(wasInstalled ? "plugins.updated" : "plugins.installed", {
        name: entry.displayName ?? entry.id,
      }),
    );
  } catch (e) {
    toasts.error(t("plugins.installFailed", { name: path, error: errText(e) }));
  }
}


let _storeInflight: Promise<void> | null = null;
let _storeFetchedAt = 0;

function cacheBustedSources(): string[] {
  const stamp = Date.now().toString(36);
  return DEFAULT_STORE_SOURCES.map((url) =>
    `${url}${url.includes("?") ? "&" : "?"}cachebust=${stamp}`,
  );
}

export function fetchStore(force = false): Promise<void> {
  if (_storeInflight) return _storeInflight;
  if (
    !force &&
    _storeFetchedAt > 0 &&
    Date.now() - _storeFetchedAt < STORE_TTL_MS
  ) {
    return Promise.resolve();
  }
  _storeLoading = true;
  _storeError = null;
  _storeInflight = (async () => {
    try {
      const res = await vynl.pluginStoreFetch(
        force ? cacheBustedSources() : DEFAULT_STORE_SOURCES,
      );
      if (res.entries.length > 0) {
        _store = res.entries;
        _storeError = res.sourceError;
      } else if (res.sourceError) {
        _store = fallbackCatalog.plugins;
        _storeError = res.sourceError;
      } else {
        _store = [];
      }
    } catch (e) {
      _store = fallbackCatalog.plugins;
      _storeError = errText(e);
    } finally {
      _storeLoading = false;
      _storeInflight = null;
      _storeFetchedAt = Date.now();
    }
  })();
  return _storeInflight;
}

interface PluginUpdateInfo {
  entry: PluginEntry;
  store: StorePlugin;
}

function parseSemver(v: string): number[] | null {
  const m = /^v?(\d+)(?:\.(\d+))?(?:\.(\d+))?/.exec(v.trim());
  if (!m) return null;
  return [Number(m[1]), Number(m[2] ?? 0), Number(m[3] ?? 0)];
}

function compareSemver(a: string, b: string): number {
  const pa = parseSemver(a);
  const pb = parseSemver(b);
  if (!pa || !pb) return 0;
  for (let i = 0; i < 3; i++) {
    const d = (pa[i] ?? 0) - (pb[i] ?? 0);
    if (d !== 0) return d;
  }
  return 0;
}

export function getPluginUpdates(): PluginUpdateInfo[] {
  const out: PluginUpdateInfo[] = [];
  for (const entry of _installed) {
    if (entry.installationMethod !== "store") continue;
    const sp = _store.find((s) => s.id === entry.id);
    if (!sp?.version) continue;
    if (compareSemver(sp.version, entry.version) > 0) {
      out.push({ entry, store: sp });
    }
  }
  return out;
}

export async function checkPluginUpdates(auto: boolean): Promise<void> {
  if (auto && !getCurrentSettings().pluginsAutoUpdate) return;
  await fetchStore(true);
  const updates = getPluginUpdates();
  if (auto) {
    for (const { store } of updates) {
      await installFromStore(store);
    }
    return;
  }
  if (updates.length === 0) {
    toasts.info(t("plugins.upToDate"));
  } else {
    toasts.info(t("plugins.updatesAvailable", { n: updates.length }));
  }
}


export async function setPluginConfigValue(
  id: string,
  key: string,
  value: unknown,
): Promise<void> {
  const next = { ...(_configs[id] ?? {}), [key]: value };
  _configs[id] = next;
  try {
    await vynl.pluginSetConfig(id, next);
  } catch (e) {
    toasts.error(t("plugins.configSaveFailed", { error: errText(e) }));
    return;
  }
  emitScopedEvent(id, pluginScopeKey(id, key), value);
}


let _uiOpen = $state(false);

export function getPluginsUiOpen(): boolean {
  return _uiOpen;
}

export function openPluginsUi(): void {
  _uiOpen = true;
}

export function closePluginsUi(): void {
  _uiOpen = false;
}

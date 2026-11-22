import type { Settings, ToolName, ToolStatus, AudioFormat } from "@lib/types";
import { BITRATES } from "@lib/constants";
import { eqFlat, loadEqDesign } from "@lib/eq.svelte";
import { vynl } from "@lib/vynl";
import { toasts } from "@lib/toast";
import { t } from "@lib/i18n";
import { DEFAULT_ACCENT_COLOR, applyAccentColor } from "@lib/color";
import { DEFAULT_THEME, applyTheme, isThemeId } from "@lib/theme";
import { extractVibrantColor } from "@lib/art-color";
import { getCurrentTrack } from "./now-playing.svelte";

function defaultSettings(): Settings {
  return {
    outputDir: "",
    format: "mp3",
    bitrate: null,
    filenamePattern: "{track} - {title}",
    overwrite: false,
    discordRpc: true,
    confirmMatches: false,
    minimizeToTray: false,
    theme: DEFAULT_THEME,
    accentColor: DEFAULT_ACCENT_COLOR,
    dynamicAccent: false,
    hardwareAcceleration: true,
    launchAtStartup: false,
    displayName: "",
    eqEnabled: false,
    eqBands: eqFlat(),
    globalMediaKeys: true,
  };
}

let _settings = $state<Settings | null>(null);
let _tools = $state<ToolStatus[]>([]);
let _toolsUnlisten: (() => void) | null = null;

const currentSettings = $derived(_settings ?? defaultSettings());

function normalizeSettings(s: Settings): Settings {
  const format: AudioFormat = "mp3";
  const theme = isThemeId(s.theme) ? s.theme : DEFAULT_THEME;
  const br = BITRATES.mp3;
  if (br && s.bitrate != null && !br.includes(s.bitrate)) {
    console.warn(
      `[Settings] Bitrate ${s.bitrate} is not valid. ` +
        `Valid bitrates: ${br.join(", ")}. Using ${br[br.length - 1]} as fallback.`
    );
    return { ...s, format, theme, bitrate: br[br.length - 1] };
  }
  return { ...s, format, theme, globalMediaKeys: s.globalMediaKeys ?? true };
}

export function getCurrentSettings(): Settings {
  return currentSettings;
}

export async function initSettings(): Promise<void> {
  await loadEqDesign();
  const loaded = await vynl.getSettings().catch((e) => {
    console.warn("settings load failed, using defaults:", e);
    return defaultSettings();
  });
  _settings = normalizeSettings(loaded);
  applyTheme(_settings.theme);
  applyAccentColor(_settings.accentColor, false);
  startDynamicAccent();
  const tools = await vynl.getTools().catch((e) => {
    console.warn("tools load failed:", e);
    return null;
  });
  _tools = tools ?? [];
  if (_toolsUnlisten) _toolsUnlisten();
  _toolsUnlisten = vynl.onToolsUpdate((t) => {
    const update = t as Partial<ToolStatus> & { name: string };
    _tools = _tools.map((s) =>
      s.name === update.name ? { ...s, ...update } : s,
    );
  });
  if (tools !== null) void ensureTools();
}

let _ensureStarted = false;

async function ensureTools(): Promise<void> {
  if (_ensureStarted) return;
  _ensureStarted = true;
  const missing = _tools.filter((tool) => !tool.installed);
  for (const tool of missing) {
    try {
      await vynl.installTool(tool.name);
    } catch (e) {
      console.warn(`auto-install failed for ${tool.name}:`, e);
      markToolError(tool.name, e);
    }
  }
  if (missing.length > 0) return;
  try {
    const checked = await vynl.checkToolUpdates();
    for (const tool of checked) {
      if (!tool.updateAvailable) continue;
      console.info(`tools: updating ${tool.name} in background`);
      try {
        await vynl.installTool(tool.name);
      } catch (e) {
        console.warn(`auto-update failed for ${tool.name}:`, e);
        markToolError(tool.name, e);
      }
    }
  } catch (e) {
    console.warn("tool update check failed:", e);
  }
}

let _settingsQueue: Promise<void> = Promise.resolve();

export async function patchSettings(patch: Partial<Settings>): Promise<void> {
  const job = _settingsQueue.then(async () => {
    if (!_settings) {
      console.warn("patchSettings called before settings finished loading — patch dropped");
      return;
    }
    const next = normalizeSettings({ ..._settings, ...patch });
    const updated = await vynl.setSettings(next);
    _settings = updated;
    if (patch.theme !== undefined) applyTheme(updated.theme);
    if (patch.accentColor !== undefined) applyAccentColor(updated.accentColor);
  });
  _settingsQueue = job.catch(() => {});
  job.catch(() => {
    toasts.error(t("error.settingsSaveFailed"));
  });
  return job;
}

let _dynamicStop: (() => void) | null = null;
let _extractGen = 0;
const _vibrantCache = new Map<string, string>();
const MAX_VIBRANT_CACHE = 500;

function cacheVibrant(cover: string, hex: string): void {
  _vibrantCache.set(cover, hex);
  while (_vibrantCache.size > MAX_VIBRANT_CACHE) {
    const oldest = _vibrantCache.keys().next().value;
    if (oldest === undefined) break;
    _vibrantCache.delete(oldest);
  }
}

function startDynamicAccent(): void {
  if (_dynamicStop) return;
  _dynamicStop = $effect.root(() => {
    $effect(() => {
      const cover = getCurrentTrack()?.cover ?? null;
      const dynamic = currentSettings.dynamicAccent;
      const fallback = currentSettings.accentColor;
      if (!dynamic || !cover) {
        applyAccentColor(fallback);
        return;
      }
      void applyExtractedAccent(cover, fallback);
    });
  });
}

async function applyExtractedAccent(
  cover: string,
  fallback: string,
): Promise<void> {
  const gen = ++_extractGen;
  let hex = _vibrantCache.get(cover);
  if (!hex) {
    const extracted = await extractVibrantColor(cover);
    if (!extracted) {
      if (gen === _extractGen) applyAccentColor(fallback);
      return;
    }
    cacheVibrant(cover, extracted);
    hex = extracted;
  }
  if (gen !== _extractGen) return;
  if (currentSettings.dynamicAccent) applyAccentColor(hex);
}

function markToolError(name: ToolName, err: unknown): void {
  const message = err instanceof Error ? err.message : String(err);
  _tools = _tools.map((s) =>
    s.name === name
      ? { ...s, state: "error", progress: undefined, error: message }
      : s,
  );
}

export async function installTool(name: ToolName): Promise<void> {
  try {
    await vynl.installTool(name);
  } catch (e) {
    markToolError(name, e);
  }
}
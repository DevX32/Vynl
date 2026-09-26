import type { Settings, ToolName, ToolStatus } from "@lib/types";
import { BITRATES, EQ_FLAT } from "@lib/constants";
import { vynl } from "@lib/vynl";
import { DEFAULT_ACCENT_COLOR, applyAccentColor } from "@lib/color";
import { extractVibrantColor } from "@lib/art-color";
import { getCurrentTrack } from "./now-playing.svelte";

const DEFAULT_SETTINGS: Settings = {
  outputDir: "",
  format: "mp3",
  bitrate: null,
  filenamePattern: "{track} - {title}",
  overwrite: false,
  discordRpc: true,
  confirmMatches: false,
  minimizeToTray: false,
  accentColor: DEFAULT_ACCENT_COLOR,
  dynamicAccent: false,
  hardwareAcceleration: true,
  launchAtStartup: false,
  displayName: "",
  eqEnabled: false,
  eqBands: EQ_FLAT,
  pluginsAutoUpdate: true,
};

let _settings = $state<Settings | null>(null);
let _tools = $state<ToolStatus[]>([]);
let _toolsUnlisten: (() => void) | null = null;

const currentSettings = $derived(_settings ?? DEFAULT_SETTINGS);
const currentTools = $derived(_tools);

function normalizeBitrate(s: Settings): Settings {
  const br = BITRATES[s.format];
  if (br && s.bitrate != null && !br.includes(s.bitrate)) {
    console.warn(
      `[Settings] Bitrate ${s.bitrate} is not valid. ` +
        `Valid bitrates: ${br.join(", ")}. Using ${br[br.length - 1]} as fallback.`
    );
    return { ...s, bitrate: br[br.length - 1] };
  }
  return s;
}

export function getCurrentSettings(): Settings {
  return currentSettings;
}
export function getCurrentTools(): ToolStatus[] {
  return currentTools;
}

export async function initSettings(): Promise<void> {
  const loaded = await vynl.getSettings();
  _settings = normalizeBitrate(loaded);
  applyAccentColor(_settings.accentColor, false);
  startDynamicAccent();
  _tools = await vynl.getTools();
  if (_toolsUnlisten) _toolsUnlisten();
  _toolsUnlisten = vynl.onToolsUpdate((t) => {
    const update = t as Partial<ToolStatus> & { name: string };
    _tools = _tools.map((s) =>
      s.name === update.name ? { ...s, ...update } : s,
    );
  });
  void autoInstallMissingTools();
}

let _autoInstallStarted = false;

async function autoInstallMissingTools(): Promise<void> {
  if (_autoInstallStarted) return;
  _autoInstallStarted = true;
  const missing = _tools.filter((tool) => !tool.installed);
  for (const tool of missing) {
    try {
      await vynl.installTool(tool.name);
    } catch (e) {
      console.warn(`auto-install failed for ${tool.name}:`, e);
      markToolError(tool.name, e);
    }
  }
}

export async function patchSettings(patch: Partial<Settings>): Promise<void> {
  if (!_settings) return;
  const next = normalizeBitrate({ ..._settings, ...patch });
  const updated = await vynl.setSettings(next);
  _settings = updated;
  if (patch.accentColor !== undefined) applyAccentColor(updated.accentColor);
}

let _dynamicStop: (() => void) | null = null;
let _extractGen = 0;
const _vibrantCache = new Map<string, string>();

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
    _vibrantCache.set(cover, extracted);
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

export async function updateTool(name: ToolName): Promise<void> {
  try {
    await vynl.updateTool(name);
  } catch (e) {
    markToolError(name, e);
  }
}
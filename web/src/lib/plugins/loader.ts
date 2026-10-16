
import { compilePlugin } from "./compiler";
import type { PluginEntry, PluginManifest, VynlPlugin } from "./types";
import { vynl } from "@lib/vynl";

async function readManifest(pluginId: string): Promise<PluginManifest> {
  const raw = await vynl.pluginsReadFile(pluginId, "package.json");
  const manifest = JSON.parse(raw) as PluginManifest;
  if (!manifest.name || !manifest.version) {
    throw new Error("package.json must declare name and version");
  }
  return manifest;
}

async function resolveEntryFile(
  pluginId: string,
  manifest: PluginManifest,
): Promise<string> {
  const candidates = [
    manifest.main?.replace(/^\.\//, ""),
    "index.ts",
    "index.js",
    "index.mjs",
    "dist/index.js",
    "dist/index.ts",
  ].filter((c): c is string => !!c);
  for (const candidate of candidates) {
    try {
      await vynl.pluginsReadFile(pluginId, candidate);
      return candidate;
    } catch {
    }
  }
  throw new Error("Could not resolve plugin entry (checked main, index.*, dist/index.*)");
}

export async function loadPluginSource(
  entry: PluginEntry,
): Promise<{ manifest: PluginManifest; code: string; entryFile: string }> {
  const manifest = await readManifest(entry.id);
  const entryFile = await resolveEntryFile(entry.id, manifest);
  const code = await compilePlugin(entry.id, entryFile);
  return { manifest, code, entryFile };
}

export async function evaluatePlugin(code: string): Promise<VynlPlugin> {
  const blob = new Blob([code], { type: "text/javascript" });
  const url = URL.createObjectURL(blob);
  try {
    const mod = (await import(/* @vite-ignore */ url)) as {
      default?: VynlPlugin;
    };
    const plugin = mod.default;
    if (!plugin || typeof plugin !== "object") {
      throw new Error("Plugin must have a default export object");
    }
    return plugin;
  } finally {
    URL.revokeObjectURL(url);
  }
}

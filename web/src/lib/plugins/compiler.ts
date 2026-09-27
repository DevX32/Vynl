
import { vynl } from "@lib/vynl";

interface CacheEntry {
  code: string;
  deps: Record<string, string>;
}

interface CompileResult {
  code: string;
  deps: string[];
}

const cache = new Map<string, CacheEntry>();

function hash(str: string): string {
  let h = 5381;
  for (let i = 0; i < str.length; i++) {
    h = ((h << 5) + h + str.charCodeAt(i)) | 0;
  }
  return (h >>> 0).toString(36);
}

async function compile(pluginId: string, entryFile: string): Promise<CacheEntry> {
  const result: CompileResult = await vynl.pluginCompile(pluginId, entryFile);

  const deps: Record<string, string> = {};
  for (const file of result.deps) {
    try {
      deps[file] = hash(await vynl.pluginsReadFile(pluginId, file));
    } catch {
      deps[file] = "missing";
    }
  }
  return { code: result.code, deps };
}

async function depsUnchanged(pluginId: string, entry: CacheEntry): Promise<boolean> {
  const files = Object.entries(entry.deps);
  if (files.length === 0) return false;
  for (const [file, expected] of files) {
    try {
      const contents = await vynl.pluginsReadFile(pluginId, file);
      if (hash(contents) !== expected) return false;
    } catch {
      return false;
    }
  }
  return true;
}

export async function compilePlugin(pluginId: string, entryFile: string): Promise<string> {
  const key = `${pluginId}:${entryFile}`;
  const cached = cache.get(key);
  if (cached && (await depsUnchanged(pluginId, cached))) {
    return cached.code;
  }
  const compiled = await compile(pluginId, entryFile);
  cache.set(key, compiled);
  return compiled.code;
}

export function clearCompilerCache(): void {
  cache.clear();
}

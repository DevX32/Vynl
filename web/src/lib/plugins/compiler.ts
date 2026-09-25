
import type { BuildOptions, Plugin } from "esbuild-wasm";
import esbuildWasmUrl from "esbuild-wasm/esbuild.wasm?url";
import { vynl } from "@lib/vynl";

type EsbuildModule = typeof import("esbuild-wasm");

const SDK_MODULE = "@vynl/plugin-sdk";
const FS_NS = "vynl-fs";
const SDK_NS = "vynl-sdk";

interface EsbuildState {
  mod: EsbuildModule | null;
  init: Promise<void> | null;
}

function getEsbuildState(): EsbuildState {
  const g = globalThis as typeof globalThis & { __VYNL_ESBUILD__?: EsbuildState };
  g.__VYNL_ESBUILD__ ??= { mod: null, init: null };
  return g.__VYNL_ESBUILD__;
}

async function ensureEsbuild(): Promise<EsbuildModule> {
  const state = getEsbuildState();
  if (state.mod) return state.mod;
  if (!state.init) {
    state.init = (async () => {
      const es = await import("esbuild-wasm");
      await es.initialize({ wasmURL: esbuildWasmUrl, worker: false });
      state.mod = es;
    })();
  }
  await state.init;
  if (!state.mod) throw new Error("esbuild-wasm failed to initialize");
  return state.mod;
}

interface CacheEntry {
  code: string;
  deps: Record<string, string>;
}

const cache = new Map<string, CacheEntry>();

function hash(str: string): string {
  let h = 5381;
  for (let i = 0; i < str.length; i++) {
    h = ((h << 5) + h + str.charCodeAt(i)) | 0;
  }
  return (h >>> 0).toString(36);
}

function resolveRelative(importer: string, spec: string): string {
  const base =
    !importer || importer === "<stdin>" ? [] : importer.split("/").slice(0, -1);
  const segments = base.filter((s) => s !== "");
  for (const seg of spec.split("/")) {
    if (seg === "" || seg === ".") continue;
    if (seg === "..") {
      if (segments.length === 0) {
        throw new Error(`Import escapes the plugin directory: ${spec}`);
      }
      segments.pop();
    } else {
      segments.push(seg);
    }
  }
  const out = segments.join("/");
  if (!out) throw new Error(`Invalid import path: ${spec}`);
  return out;
}

function pickLoader(file: string): "ts" | "tsx" | "js" {
  if (file.endsWith(".tsx")) return "tsx";
  if (file.endsWith(".ts")) return "ts";
  return "js";
}

async function compile(pluginId: string, entryFile: string): Promise<CacheEntry> {
  const es = await ensureEsbuild();
  const deps: Record<string, string> = {};

  const readPluginFile = async (path: string): Promise<string> => {
    const candidates = [
      path,
      path + ".ts",
      path + ".tsx",
      path + ".js",
      path + ".mjs",
      path + "/index.ts",
      path + "/index.js",
    ];
    for (const candidate of candidates) {
      try {
        const contents = await vynl.pluginsReadFile(pluginId, candidate);
        deps[candidate] = hash(contents);
        return contents;
      } catch {}
    }
    throw new Error(`File not found in plugin: ${path}`);
  };

  const entrySource = await readPluginFile(entryFile);

  const vfsPlugin: Plugin = {
    name: "vynl-vfs",
    setup(build) {
      build.onResolve({ filter: /^@vynl\/plugin-sdk$/ }, () => ({
        path: SDK_MODULE,
        namespace: SDK_NS,
      }));

      build.onLoad({ filter: /.*/, namespace: SDK_NS }, () => ({
        contents:
          "const sdk = globalThis.__VYNL_PLUGIN_SDK__ || {};\n" +
          "export default sdk;\n" +
          "export const PLUGIN_API_VERSION = sdk.PLUGIN_API_VERSION || 0;\n",
        loader: "js",
      }));

      build.onResolve({ filter: /^\.{1,2}\// }, (args) => {
        try {
          const importer =
            !args.importer || args.importer === "<stdin>"
              ? entryFile
              : args.importer;
          return { path: resolveRelative(importer, args.path), namespace: FS_NS };
        } catch (e) {
          return { errors: [{ text: e instanceof Error ? e.message : String(e) }] };
        }
      });

      build.onLoad({ filter: /.*/, namespace: FS_NS }, async (args) => {
        try {
          const contents = await readPluginFile(args.path);
          return { contents, loader: pickLoader(args.path) };
        } catch (e) {
          return { errors: [{ text: e instanceof Error ? e.message : String(e) }] };
        }
      });
    },
  };

  const options: BuildOptions = {
    stdin: {
      contents: entrySource,
      resolveDir: "/",
      sourcefile: entryFile,
      loader: pickLoader(entryFile),
    },
    bundle: true,
    write: false,
    format: "esm",
    platform: "browser",
    target: "es2020",
    plugins: [vfsPlugin],
    logLevel: "silent",
  };

  const result = await es.build(options);
  const out = result.outputFiles?.[0];
  if (!out) throw new Error("Compilation produced no output");
  return { code: out.text, deps };
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

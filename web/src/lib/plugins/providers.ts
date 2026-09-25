
import type { ArtistInfo, Collection } from "@lib/types";
import { vynl } from "@lib/vynl";
import type {
  PluginLyricsLookup,
  PluginLyricsProvider,
  PluginLyricsResult,
  PluginMetadataProvider,
  PluginProvider,
  PluginProviderKind,
  PluginResolverProvider,
} from "./types";

interface Owned {
  pluginId: string;
  provider: PluginProvider;
}

let providers: Owned[] = [];
const listeners = new Set<() => void>();

function notify(): void {
  for (const l of [...listeners]) {
    try {
      l();
    } catch (e) {
      console.warn("[plugins] provider subscriber failed:", e);
    }
  }
}

export function registerProvider(
  pluginId: string,
  provider: PluginProvider,
): void {
  providers = providers.filter(
    (o) => !(o.pluginId === pluginId && o.provider.id === provider.id),
  );
  providers = [...providers, { pluginId, provider }];
  notify();
}

export function unregisterProvider(pluginId: string, id: string): boolean {
  const before = providers.length;
  providers = providers.filter(
    (o) => !(o.pluginId === pluginId && o.provider.id === id),
  );
  if (providers.length !== before) {
    notify();
    return true;
  }
  return false;
}

export function listProviders(kind?: PluginProviderKind): PluginProvider[] {
  return providers
    .filter((o) => !kind || o.provider.kind === kind)
    .map((o) => o.provider);
}

export function getProvider<T extends PluginProvider>(
  id: string,
  kind: PluginProviderKind,
): T | undefined {
  return providers.find(
    (o) => o.provider.id === id && o.provider.kind === kind,
  )?.provider as T | undefined;
}

export function subscribeProviders(cb: () => void): () => void {
  listeners.add(cb);
  return () => {
    listeners.delete(cb);
  };
}

export async function runLyricsProviders(
  lookup: PluginLyricsLookup,
): Promise<PluginLyricsResult | null> {
  for (const { provider } of providers) {
    if (provider.kind !== "lyrics") continue;
    const lyricsProvider = provider as PluginLyricsProvider;
    try {
      const res = await lyricsProvider.fetch(lookup);
      if (
        res &&
        typeof res.text === "string" &&
        res.text.trim().length > 0 &&
        (res.kind === "lrc" || res.kind === "txt")
      ) {
        return { kind: res.kind, text: res.text };
      }
    } catch (e) {
      console.warn(`[plugins] lyrics provider "${provider.id}" failed:`, e);
    }
  }
  return null;
}

export async function runArtistInfoProviders(
  artist: string,
): Promise<ArtistInfo | null> {
  for (const { provider } of providers) {
    if (provider.kind !== "metadata") continue;
    const metaProvider = provider as PluginMetadataProvider;
    if (!metaProvider.artistInfo) continue;
    try {
      const res = await metaProvider.artistInfo(artist);
      if (res && typeof res.name === "string") return res;
    } catch (e) {
      console.warn(`[plugins] metadata provider "${provider.id}" failed:`, e);
    }
  }
  return null;
}

export async function resolveCollectionWithPlugins(
  url: string,
): Promise<Collection> {
  const resolvers: PluginResolverProvider[] = [];
  for (const { provider } of providers) {
    if (provider.kind !== "resolver") continue;
    const resolver = provider as PluginResolverProvider;
    try {
      if (await resolver.canHandle(url)) resolvers.push(resolver);
    } catch (e) {
      console.warn(`[plugins] resolver "${provider.id}" failed:`, e);
    }
  }

  for (const resolver of resolvers) {
    try {
      const collection = await resolver.resolve(url);
      if (collection && Array.isArray(collection.tracks)) return collection;
    } catch (e) {
      console.warn(`[plugins] resolver "${resolver.id}" failed:`, e);
    }
  }

  return vynl.resolve(url);
}

import type { ArtistInfo } from "@lib/types";
import { vynl } from "@lib/vynl";
import { runArtistInfoProviders } from "@lib/plugins/providers";

const MAX = 64;
const cache = new Map<string, ArtistInfo>();
const inflight = new Map<string, Promise<ArtistInfo>>();

function keyOf(name: string): string {
  return name.trim().toLowerCase();
}

function remember(key: string, info: ArtistInfo): void {
  if (cache.size >= MAX) {
    const first = cache.keys().next().value;
    if (first !== undefined) cache.delete(first);
  }
  cache.set(key, info);
}

export function peekArtistInfo(artist: string): ArtistInfo | null {
  return cache.get(keyOf(artist)) ?? null;
}

export function loadArtistInfo(artist: string): Promise<ArtistInfo> {
  const key = keyOf(artist);
  const hit = cache.get(key);
  if (hit) return Promise.resolve(hit);

  const pending = inflight.get(key);
  if (pending) return pending;

  const req = (async () => {
    const info = (await runArtistInfoProviders(artist)) ??
      (await vynl.fetchArtistInfo(artist));
    remember(key, info);
    const canon = keyOf(info.name);
    if (canon !== key) remember(canon, info);
    return info;
  })().finally(() => {
    inflight.delete(key);
  });

  inflight.set(key, req);
  return req;
}

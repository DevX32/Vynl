import type { ArtistInfo } from "@lib/types";
import { vynl } from "@lib/vynl";
import { runArtistInfoProviders } from "@lib/plugins/providers";

const MAX = 64;
const cache = new Map<string, ArtistInfo>();
const inflight = new Map<string, Promise<ArtistInfo>>();
let epoch = 0;

function keyOf(name: string): string {
  return name.trim().toLowerCase();
}

function remember(key: string, info: ArtistInfo): void {
  if (!key || !keyOf(info.name)) return;
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
  if (!key) return Promise.reject(new Error("Artist name cannot be empty"));

  const hit = cache.get(key);
  if (hit) return Promise.resolve(hit);

  const pending = inflight.get(key);
  if (pending) return pending;

  const myEpoch = epoch;
  const req = (async () => {
    const info = (await runArtistInfoProviders(artist)) ??
      (await vynl.fetchArtistInfo(artist));
    if (myEpoch === epoch) {
      remember(key, info);
      const canon = keyOf(info.name);
      if (canon !== key) remember(canon, info);
    }
    return info;
  })().finally(() => {
    inflight.delete(key);
  });

  inflight.set(key, req);
  return req;
}

export function invalidateArtistCache(artists?: Iterable<string>): void {
  epoch += 1;
  cache.delete("");
  if (artists === undefined) {
    cache.clear();
    inflight.clear();
    return;
  }
  for (const name of artists) {
    const key = keyOf(name);
    if (!key) continue;
    cache.delete(key);
    inflight.delete(key);
  }
}

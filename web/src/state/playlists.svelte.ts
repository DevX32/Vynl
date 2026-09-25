import type { LibraryTrack, Playlist, PlaylistMeta } from "@lib/types";
import { getLibrary } from "./library.svelte";
import { vynl } from "@lib/vynl";

let _meta = $state<PlaylistMeta[]>([]);
let _all = $state<Record<string, Playlist>>({});
let _current = $state<Playlist | null>(null);
let _selected = $state<string | null>(null);
let _fetchAllGeneration = 0;
let _refreshPromise: Promise<void> | null = null;

const currentPlaylists = $derived(_meta);
const currentPlaylist = $derived(_current);
const selectedId = $derived(_selected);

export function getCurrentPlaylists(): PlaylistMeta[] {
  return currentPlaylists;
}
export function getCurrentPlaylist(): Playlist | null {
  return currentPlaylist;
}
export function getSelectedId(): string | null {
  return selectedId;
}

async function fetchAll(): Promise<void> {
  const gen = ++_fetchAllGeneration;
  const map: Record<string, Playlist> = {};
  await Promise.all(
    _meta.map(async (pl) => {
      const p = await vynl.getPlaylist(pl.id);
      if (p) map[p.id] = p;
    }),
  );
  if (gen === _fetchAllGeneration) {
    _all = map;
  }
}

export function refreshPlaylists(): Promise<void> {
  if (_refreshPromise) return _refreshPromise;

  const promise = (async () => {
    try {
      _meta = await vynl.listPlaylists();
      await fetchAll();
    } catch (e) {
      console.warn("refreshPlaylists failed:", e);
    } finally {
      _refreshPromise = null;
    }
  })();
  _refreshPromise = promise;
  return promise;
}

export async function selectPlaylist(id: string | null): Promise<void> {
  _selected = id;
  try {
    _current = id ? await vynl.getPlaylist(id) : null;
  } catch (e) {
    console.warn("selectPlaylist failed:", e);
    _current = null;
  }
}

export async function createPlaylist(name: string): Promise<Playlist> {
  const pl = await vynl.createPlaylist(name);
  _selected = pl.id;
  _current = pl;
  await refreshPlaylists();
  return pl;
}

function syncAfterMutation(pl: Playlist): void {
  _all = { ..._all, [pl.id]: pl };
  if (_current?.id === pl.id) _current = pl;
}

export async function renamePlaylist(id: string, name: string): Promise<void> {
  const pl = await vynl.renamePlaylist(id, name);
  syncAfterMutation(pl);
  _meta = await vynl.listPlaylists();
}

export async function deletePlaylist(id: string): Promise<void> {
  await vynl.deletePlaylist(id);
  if (_selected === id) {
    _selected = null;
    _current = null;
  }
  _meta = await vynl.listPlaylists();
}

export async function addToPlaylist(
  id: string,
  paths: string[],
): Promise<void> {
  const pl = await vynl.addToPlaylist(id, paths);
  syncAfterMutation(pl);
  _meta = await vynl.listPlaylists();
}

export async function removeFromPlaylist(
  id: string,
  trackPath: string,
): Promise<void> {
  const pl = await vynl.removeFromPlaylist(id, trackPath);
  syncAfterMutation(pl);
  _meta = await vynl.listPlaylists();
}

export async function moveInPlaylist(
  id: string,
  from: number,
  to: number,
): Promise<void> {
  const pl = await vynl.moveInPlaylist(id, from, to);
  syncAfterMutation(pl);
  _meta = await vynl.listPlaylists();
}

export async function setPlaylistCover(
  id: string,
  cover: string | null,
): Promise<void> {
  const pl = await vynl.setPlaylistCover(id, cover);
  syncAfterMutation(pl);
  _meta = await vynl.listPlaylists();
}

function tracksOf(pl: Playlist): LibraryTrack[] {
  const library = getLibrary();
  return pl.paths
    .map((p) => library.find((t) => t.path === p))
    .filter((t): t is LibraryTrack => !!t);
}

export function getPlaylistCovers(id: string): string[] {
  const pl = _all[id];
  if (!pl) return [];
  if (pl.cover) return [pl.cover];
  const covers = tracksOf(pl)
    .map((t) => t.cover)
    .filter((c): c is string => !!c);
  return [...new Set(covers)].slice(0, 4);
}

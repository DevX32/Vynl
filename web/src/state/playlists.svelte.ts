import type {
  CatalogueTrack,
  DeleteTracksResult,
  LibraryTrack,
  Playlist,
  PlaylistMeta,
} from "@lib/types";
import { getLibrary, getLibraryRevision, onLibraryChanged, refreshLibrary } from "./library.svelte";
import { moveItem } from "@lib/reorder";
import { songKey } from "@lib/format";
import { vynl } from "@lib/vynl";

let _meta = $state<PlaylistMeta[]>([]);
let _all = $state<Record<string, Playlist>>({});
let _current = $state<Playlist | null>(null);
let _selected = $state<string | null>(null);
let _selecting = $state(false);
let _selectGeneration = 0;
let _metaGeneration = 0;
let _refreshPromise: Promise<void> | null = null;
let _refreshAgain = false;

const currentPlaylists = $derived(_meta);
const currentPlaylist = $derived(_current);
const selectedId = $derived(_selected);
const isSelecting = $derived(_selecting);

export function getCurrentPlaylists(): PlaylistMeta[] {
  return currentPlaylists;
}
export function getCurrentPlaylist(): Playlist | null {
  return currentPlaylist;
}
export function getSelectedId(): string | null {
  return selectedId;
}
export function isPlaylistLoading(): boolean {
  return isSelecting;
}

async function fetchAll(generation: number): Promise<void> {
  const map: Record<string, Playlist> = {};
  await Promise.all(
    _meta.map(async (pl) => {
      if (generation !== _metaGeneration) return;
      try {
        const p = await vynl.getPlaylist(pl.id);
        if (p) map[p.id] = p;
      } catch (error) {
        console.warn("failed to load playlist", pl.id, error);
      }
    }),
  );
  if (generation === _metaGeneration) {
    _all = map;
  }
}

export function refreshPlaylistMeta(): Promise<void> {
  const generation = ++_metaGeneration;
  return (async () => {
    try {
      const meta = await vynl.listPlaylists();
      if (generation === _metaGeneration) _meta = meta;
    } catch (e) {
      console.warn("refreshPlaylistMeta failed:", e);
    }
  })();
}

export function refreshPlaylists(): Promise<void> {
  if (_refreshPromise) {
    _refreshAgain = true;
    return _refreshPromise;
  }

  const generation = ++_metaGeneration;
  const promise = (async () => {
    try {
      const meta = await vynl.listPlaylists();
      if (generation === _metaGeneration) _meta = meta;
      await fetchAll(generation);
      if (generation === _metaGeneration && _selected) {
        const fresh = _all[_selected] ?? null;
        if (fresh) {
          _current = fresh;
        } else if (!_selecting && !_meta.some((m) => m.id === _selected)) {
          _selected = null;
          _current = null;
        }
      }
    } catch (e) {
      console.warn("refreshPlaylists failed:", e);
    } finally {
      _refreshPromise = null;
      if (_refreshAgain) {
        _refreshAgain = false;
        void refreshPlaylists();
      }
    }
  })();
  _refreshPromise = promise;
  return promise;
}

export async function selectPlaylist(id: string | null): Promise<void> {
  const gen = ++_selectGeneration;
  _selected = id;
  if (!id) {
    _current = null;
    _selecting = false;
    return;
  }
  _selecting = true;
  try {
    const p = await vynl.getPlaylist(id);
    if (gen === _selectGeneration) _current = p;
  } catch (e) {
    console.warn("selectPlaylist failed:", e);
    if (gen === _selectGeneration) _current = null;
  } finally {
    _selecting = false;
  }
}

export async function createPlaylist(name: string): Promise<Playlist> {
  const pl = await vynl.createPlaylist(name);
  setCurrentPlaylist(pl);
  await refreshPlaylists();
  return pl;
}

export async function reorderPlaylists(
  fromId: string,
  toId: string,
): Promise<void> {
  const from = _meta.findIndex((p) => p.id === fromId);
  const to = _meta.findIndex((p) => p.id === toId);
  if (from < 0 || to < 0 || from === to) return;

  const next = moveItem(_meta, from, to);
  _metaGeneration += 1;
  _meta = next;
  try {
    await vynl.reorderPlaylists(next.map((p) => p.id));
  } catch (e) {
    console.warn("reorderPlaylists failed:", e);
    try {
      await refreshMeta();
    } catch {}
  }
}

function metaOf(pl: Playlist): PlaylistMeta {
  return {
    id: pl.id,
    name: pl.name,
    trackCount: pl.paths.length,
    createdAt: pl.createdAt,
    updatedAt: pl.updatedAt,
  };
}

export function setCurrentPlaylist(pl: Playlist): void {
  _selectGeneration += 1;
  _metaGeneration += 1;
  _selected = pl.id;
  _current = pl;
  _selecting = false;
  const meta = metaOf(pl);
  const exists = _meta.some((m) => m.id === pl.id);
  _meta = exists
    ? _meta.map((m) => (m.id === pl.id ? meta : m))
    : [..._meta, meta];
  _all = { ..._all, [pl.id]: pl };
}

function syncAfterMutation(pl: Playlist): void {
  _metaGeneration += 1;
  _all = { ..._all, [pl.id]: pl };
  if (_current?.id === pl.id) _current = pl;
}

export async function renamePlaylist(id: string, name: string): Promise<void> {
  const pl = await vynl.renamePlaylist(id, name);
  syncAfterMutation(pl);
  await refreshMeta();
}

async function refreshMeta(): Promise<void> {
  const generation = ++_metaGeneration;
  try {
    const meta = await vynl.listPlaylists();
    if (generation === _metaGeneration) _meta = meta;
    await fetchAll(generation);
  } catch (e) {
    console.warn("refreshMeta failed:", e);
  }
}

export async function deletePlaylist(id: string): Promise<DeleteTracksResult> {
  const result = await vynl.deletePlaylist(id);
  _selectGeneration += 1;
  _metaGeneration += 1;
  _meta = _meta.filter((p) => p.id !== id);
  await refreshMeta();
  const next = { ..._all };
  delete next[id];
  _all = next;
  if (_selected === id) {
    _selected = null;
    _current = null;
  }
  return result;
}

export async function addToPlaylist(
  id: string,
  paths: string[],
): Promise<void> {
  const pl = await vynl.addToPlaylist(id, paths);
  syncAfterMutation(pl);
  await refreshMeta();
}

export async function removeFromPlaylist(
  id: string,
  trackPath: string,
): Promise<void> {
  const pl = await vynl.removeFromPlaylist(id, trackPath);
  syncAfterMutation(pl);
  if (!trackPath.startsWith("ytm:")) {
    await refreshLibrary();
  }
  await refreshMeta();
}

export async function moveInPlaylist(
  id: string,
  from: number,
  to: number,
): Promise<void> {
  const pl = await vynl.moveInPlaylist(id, from, to);
  syncAfterMutation(pl);
  await refreshMeta();
}

export async function setPlaylistCover(
  id: string,
  cover: string | null,
): Promise<void> {
  const pl = await vynl.setPlaylistCover(id, cover);
  syncAfterMutation(pl);
  await refreshMeta();
}

export async function saveStreamToPlaylist(
  track: CatalogueTrack,
  playlistId?: string | null,
  playlistName?: string | null,
  refresh = true,
): Promise<Playlist> {
  const pl = await vynl.catalogueSaveToPlaylist(
    track,
    playlistId ?? null,
    playlistName ?? null,
  );
  syncAfterMutation(pl);
  if (refresh) await refreshMeta();
  return pl;
}

let _pathSource: LibraryTrack[] | null = null;
let _pathMap = new Map<string, LibraryTrack>();

let _failedCovers = $state<Set<string>>(new Set());

onLibraryChanged(() => {
  _failedCovers = new Set();
});

export function isCataloguePath(path: string): boolean {
  return path.startsWith("ytm:");
}

export function catalogueIdOf(path: string): string | null {
  if (!path.startsWith("ytm:")) return null;
  const id = path.slice(4);
  return id.length > 0 ? id : null;
}

function streamTrackOf(track: CatalogueTrack): LibraryTrack {
  return {
    id: track.id,
    path: `ytm:${track.id}`,
    title: track.title,
    artist: track.artist,
    album: track.album?.name ?? "",
    year: null,
    duration:
      Number.isFinite(track.duration) && (track.duration ?? 0) >= 0
        ? (track.duration ?? 0)
        : 0,
    cover: track.cover,
    trackNumber: null,
    lyrics: null,
    ext: "webm",
    mtime: null,
    addedAt: null,
  };
}

export function getPlaylistStreamTracks(): LibraryTrack[] {
  const seen = new Map<string, LibraryTrack>();
  for (const pl of Object.values(_all)) {
    if (pl.id === "liked") continue;
    const at = pl.updatedAt || pl.createdAt;
    for (const track of pl.streams) {
      const known = seen.get(track.id);
      if (known && (known.addedAt ?? 0) >= at) continue;
      seen.set(track.id, { ...streamTrackOf(track), addedAt: at });
    }
  }
  return [...seen.values()];
}

const _tracksCache = new Map<
  Playlist,
  { library: LibraryTrack[] | null; result: LibraryTrack[] }
>();

export function tracksOf(pl: Playlist): LibraryTrack[] {
  const library = getLibrary();
  const cached = _tracksCache.get(pl);
  if (cached && cached.library === library) return cached.result;

  if (_pathSource !== library) {
    const next = new Map<string, LibraryTrack>();
    for (const track of library) next.set(track.path, track);
    _pathSource = library;
    _pathMap = next;
  }
  const streams = new Map(pl.streams.map((track) => [track.id, track]));
  const localTracks: LibraryTrack[] = [];
  for (const path of pl.paths) {
    if (isCataloguePath(path)) continue;
    const track = _pathMap.get(path);
    if (track) localTracks.push(track);
  }
  const localSongDurations = new Map<string, number[]>();
  for (const track of localTracks) {
    if (!(track.duration > 0)) continue;
    const key = songKey(track.title, track.artist);
    const list = localSongDurations.get(key);
    if (list) {
      list.push(track.duration);
    } else {
      localSongDurations.set(key, [track.duration]);
    }
  }
  const hasLocalMatch = (stream: LibraryTrack): boolean => {
    if (!(stream.duration > 0)) return false;
    const durations = localSongDurations.get(
      songKey(stream.title, stream.artist),
    );
    return (
      durations !== undefined &&
      durations.some((d) => Math.abs(d - stream.duration) <= 3)
    );
  };
  const result: LibraryTrack[] = [];
  for (const path of pl.paths) {
    if (isCataloguePath(path)) {
      const id = catalogueIdOf(path);
      const track = id ? streams.get(id) : undefined;
      if (!track) {
        result.push({
          id: id ?? path,
          path,
          title: "Catalogue track (details unavailable)",
          artist: "",
          album: "",
          year: null,
          duration: 0,
          cover: null,
          trackNumber: null,
          lyrics: null,
          ext: "",
        });
        continue;
      }
      const stream = streamTrackOf(track);
      if (hasLocalMatch(stream)) continue;
      result.push(stream);
      continue;
    }
    const track = _pathMap.get(path);
    if (track) result.push(track);
  }

  _tracksCache.set(pl, { library, result });
  if (_tracksCache.size > 64) {
    const oldest = _tracksCache.keys().next().value;
    if (oldest !== undefined) _tracksCache.delete(oldest);
  }
  return result;
}

export function markCoverFailed(path: string | null | undefined): void {
  if (!path || _failedCovers.has(path)) return;
  _failedCovers = new Set(_failedCovers).add(path);
}

export function resolvePlaylistCovers(
  custom: string | null | undefined,
  tracks: readonly { cover: string | null }[],
): string[] {
  if (custom && !_failedCovers.has(custom)) return [custom];
  const covers = tracks
    .map((t) => t.cover)
    .filter((c): c is string => !!c && !_failedCovers.has(c));
  return [...new Set(covers)].slice(0, 4);
}

export function getPlaylistCustomCover(id: string): string | null {
  return _all[id]?.cover ?? null;
}

const _coversCache = new Map<
  string,
  { failed: Set<string>; revision: number; meta: number; result: string[] }
>();

export function getPlaylistCovers(id: string): string[] {
  const pl = _all[id];
  if (!pl) return [];
  const cached = _coversCache.get(id);
  const revision = getLibraryRevision();
  const meta = _metaGeneration;
  if (cached && cached.failed === _failedCovers && cached.revision === revision && cached.meta === meta) {
    return cached.result;
  }
  const result = resolvePlaylistCovers(pl.cover, tracksOf(pl));
  _coversCache.set(id, { failed: _failedCovers, revision, meta, result });
  if (_coversCache.size > 128) {
    const oldest = _coversCache.keys().next().value;
    if (oldest !== undefined) _coversCache.delete(oldest);
  }
  return result;
}

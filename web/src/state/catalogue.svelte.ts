import type {
  CatalogueAddResult,
  CatalogueAlbumDetail,
  CatalogueArtist,
  CataloguePlaylistDetail,
  CataloguePlaylistPage,
  CatalogueSearchResults,
  CatalogueTrack,
  LibraryTrack,
} from "@lib/types";
import { vynl } from "@lib/vynl";
import { evictOldest } from "@lib/cache";
import { readStringSet } from "@lib/storage";
import { shuffle } from "@lib/shuffle";
import {
  CATALOGUE_PREFIX,
  cataloguePath,
  catalogueIdFromPath,
} from "@lib/catalogue";
import { refreshLibrary } from "./library.svelte";
import { refreshPlaylists } from "./playlists.svelte";

export { cataloguePath, catalogueIdFromPath };

const REGISTRY_KEY = "vynl.catalogue.registry.v1";
const REGISTRY_LIMIT = 800;

interface StoredTrack {
  id: string;
  title: string;
  artist: string;
  album: string;
  duration: number;
  cover: string | null;
}

function isStoredTrack(value: unknown): value is StoredTrack {
  if (!value || typeof value !== "object") return false;
  const t = value as Record<string, unknown>;
  return (
    typeof t.id === "string" &&
    typeof t.title === "string" &&
    typeof t.artist === "string" &&
    typeof t.album === "string" &&
    typeof t.duration === "number" &&
    (t.cover === null || typeof t.cover === "string")
  );
}

function loadRegistry(): LibraryTrack[] {
  if (typeof localStorage === "undefined") return [];
  try {
    const raw = localStorage.getItem(REGISTRY_KEY);
    if (!raw) return [];
    const parsed: unknown = JSON.parse(raw);
    if (!Array.isArray(parsed)) return [];
    return parsed.filter(isStoredTrack).map((track) => ({
      id: track.id,
      path: `${CATALOGUE_PREFIX}${track.id}`,
      title: track.title,
      artist: track.artist,
      album: track.album,
      year: null,
      duration:
        Number.isFinite(track.duration) && track.duration >= 0
          ? track.duration
          : 0,
      cover: track.cover,
      trackNumber: null,
      lyrics: null,
      ext: "webm",
      mtime: null,
      addedAt: null,
    }));
  } catch (error) {
    console.warn("catalogue registry load failed:", error);
    return [];
  }
}

let _tracks = $state<LibraryTrack[]>(loadRegistry());
let _revision = $state(0);
let _index: Map<string, LibraryTrack> = new Map();
let _indexRevision = -1;
let persistTimer: ReturnType<typeof setTimeout> | null = null;

function registryIndex(): Map<string, LibraryTrack> {
  if (_indexRevision !== _revision) {
    _index = new Map(_tracks.map((track) => [track.id, track]));
    _indexRevision = _revision;
  }
  return _index;
}

function writeRegistry(): void {
  if (typeof localStorage === "undefined") return;
  try {
    const stored: StoredTrack[] = _tracks
      .slice(-REGISTRY_LIMIT)
      .map((track) => ({
        id: track.id,
        title: track.title,
        artist: track.artist,
        album: track.album,
        duration: track.duration,
        cover: track.cover,
      }));
    localStorage.setItem(REGISTRY_KEY, JSON.stringify(stored));
  } catch (error) {
    console.warn("catalogue registry save failed:", error);
  }
}

function flushRegistry(): void {
  if (persistTimer !== null) {
    clearTimeout(persistTimer);
    persistTimer = null;
  }
  writeRegistry();
}

function persistRegistry(): void {
  if (typeof localStorage === "undefined") return;
  if (persistTimer !== null) clearTimeout(persistTimer);
  persistTimer = setTimeout(() => {
    persistTimer = null;
    writeRegistry();
  }, 750);
}

if (typeof window !== "undefined") {
  const onVisibilityChange = () => {
    if (document.visibilityState === "hidden") flushRegistry();
  };
  window.addEventListener("pagehide", flushRegistry);
  document.addEventListener("visibilitychange", onVisibilityChange);
  import.meta.hot?.dispose(() => {
    window.removeEventListener("pagehide", flushRegistry);
    document.removeEventListener("visibilitychange", onVisibilityChange);
  });
}

function libraryTrackOf(track: CatalogueTrack): LibraryTrack {
  return {
    id: track.id,
    path: `${CATALOGUE_PREFIX}${track.id}`,
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

function sameMetadata(a: LibraryTrack, b: LibraryTrack): boolean {
  return (
    a.title === b.title &&
    a.artist === b.artist &&
    a.album === b.album &&
    a.duration === b.duration &&
    a.cover === b.cover
  );
}

export function registerCatalogueTracks(
  tracks: readonly CatalogueTrack[],
): void {
  if (tracks.length === 0) return;
  const next = new Map(registryIndex());
  let changed = false;
  for (const track of tracks) {
    if (!track.id) continue;
    const entry = libraryTrackOf(track);
    const current = next.get(entry.id);
    const mergedEntry =
      current && entry.duration <= 0 && current.duration > 0
        ? {
            ...entry,
            duration: current.duration,
            cover: entry.cover ?? current.cover,
          }
        : entry;
    if (!current || !sameMetadata(current, mergedEntry)) changed = true;
    next.set(entry.id, mergedEntry);
  }
  if (!changed) {
    const prevIds = _tracks.map((track) => track.id);
    const nextIds = [...next.keys()];
    if (nextIds.length !== prevIds.length) {
      changed = true;
    } else {
      for (let i = 0; i < nextIds.length; i += 1) {
        if (prevIds[i] !== nextIds[i]) {
          changed = true;
          break;
        }
      }
    }
  }
  if (!changed) return;

  let values = [...next.values()];
  if (values.length > REGISTRY_LIMIT) {
    values = values.slice(values.length - REGISTRY_LIMIT);
  }
  _tracks = values;
  _revision += 1;
  persistRegistry();
}

export function getCatalogueRevision(): number {
  return _revision;
}

export function getCatalogueTracks(): readonly LibraryTrack[] {
  void _revision;
  return _tracks;
}

export function getCatalogueTrack(id: string): LibraryTrack | null {
  void _revision;
  return registryIndex().get(id) ?? null;
}

export function enrichCatalogueTrack(track: CatalogueTrack): CatalogueTrack {
  const hasDuration =
    Number.isFinite(track.duration) && (track.duration ?? 0) > 0;
  if (hasDuration) return track;
  const known = getCatalogueTrack(track.id);
  if (known && known.duration > 0) {
    return {
      ...track,
      duration: known.duration,
      cover: track.cover ?? known.cover,
    };
  }
  return track;
}

export function enrichLibraryTrack(track: LibraryTrack): LibraryTrack {
  if (track.duration > 0) return track;
  if (!track.path.startsWith(CATALOGUE_PREFIX)) return track;
  const known = getCatalogueTrack(track.path.slice(CATALOGUE_PREFIX.length));
  if (known && known.duration > 0) {
    return {
      ...track,
      duration: known.duration,
      cover: track.cover ?? known.cover,
    };
  }
  return track;
}

let _searchQuery = $state("");
let _results = $state<CatalogueSearchResults | null>(null);
let _searching = $state(false);
let _searchError = $state<string | null>(null);
let _searchGeneration = 0;

export function getCatalogueQuery(): string {
  return _searchQuery;
}

export function getCatalogueResults(): CatalogueSearchResults | null {
  return _results;
}

export function isCatalogueSearching(): boolean {
  return _searching;
}

export function getCatalogueSearchError(): string | null {
  return _searchError;
}

export async function searchCatalogue(query: string): Promise<void> {
  const trimmed = query.trim();
  _searchQuery = trimmed;
  if (!trimmed) {
    _searchGeneration += 1;
    _searching = false;
    _searchError = null;
    _results = null;
    return;
  }

  const generation = ++_searchGeneration;
  _searching = true;
  _searchError = null;
  _results = null;
  try {
    const results = await vynl.catalogueSearch(trimmed);
    if (generation !== _searchGeneration) return;
    _results = results;
    registerCatalogueTracks(results.songs);
  } catch (error) {
    if (generation !== _searchGeneration) return;
    _searchError = error instanceof Error ? error.message : String(error);
    console.warn("catalogue search failed:", error);
  } finally {
    if (generation === _searchGeneration) _searching = false;
  }
}

const TASTE_CACHE = "vynl.taste.mix.v1";
const DISMISSED_KEY = "vynl.taste.dismissed.v1";

function readDismissedIds(): Set<string> {
  return readStringSet(DISMISSED_KEY);
}

interface TasteCache {
  profileKey: string;
  seedsKey: string;
  tracks: CatalogueTrack[];
}

function readTasteCache(): TasteCache | null {
  if (typeof localStorage === "undefined") return null;
  try {
    const raw = localStorage.getItem(TASTE_CACHE);
    if (!raw) return null;
    const parsed = JSON.parse(raw) as Partial<TasteCache> | null;
    if (
      !parsed ||
      typeof parsed.profileKey !== "string" ||
      typeof parsed.seedsKey !== "string" ||
      !Array.isArray(parsed.tracks)
    ) {
      return null;
    }
    const tracks = parsed.tracks.filter(
      (t): t is CatalogueTrack =>
        typeof t === "object" &&
        t !== null &&
        typeof (t as CatalogueTrack).id === "string" &&
        typeof (t as CatalogueTrack).title === "string",
    );
    return { profileKey: parsed.profileKey, seedsKey: parsed.seedsKey, tracks };
  } catch {
    return null;
  }
}

function writeTasteCache(cache: TasteCache): void {
  if (typeof localStorage === "undefined") return;
  try {
    localStorage.setItem(TASTE_CACHE, JSON.stringify(cache));
  } catch {}
}

const cachedTaste = readTasteCache();
const cachedDismissed = readDismissedIds();
const cachedTracks = (cachedTaste?.tracks ?? []).filter(
  (t) => !cachedDismissed.has(t.id),
);
if (cachedTracks.length > 0) {
  registerCatalogueTracks(cachedTracks);
}

let _taste = $state<CatalogueTrack[]>(cachedTracks);
let _tasteKey = $state(cachedTaste?.seedsKey ?? "");
let _tasteProfileKey: string | null = cachedTaste?.profileKey ?? null;
let _tasteLoading = $state(false);
let _tasteError = $state<string | null>(null);
let _tasteGeneration = 0;

export function getCatalogueTaste(): readonly CatalogueTrack[] {
  return _taste;
}

export function removeFromCatalogueTaste(id: string): void {
  if (!id) return;
  const next = _taste.filter((t) => t.id !== id);
  if (next.length === _taste.length) return;
  _taste = next;
  if (_tasteProfileKey) {
    writeTasteCache({
      profileKey: _tasteProfileKey,
      seedsKey: _tasteKey,
      tracks: _taste,
    });
  }
}

export function getCatalogueTasteProfileKey(): string | null {
  return _tasteProfileKey;
}

export function isCatalogueTasteLoading(): boolean {
  return _tasteLoading;
}

export function getCatalogueTasteError(): string | null {
  return _tasteError;
}

export function resetCatalogueTaste(): void {
  _tasteGeneration += 1;
  _taste = [];
  _tasteKey = "";
  _tasteLoading = false;
  _tasteError = null;
  _tasteProfileKey = null;
  if (typeof localStorage !== "undefined") {
    try {
      localStorage.removeItem(TASTE_CACHE);
    } catch {}
  }
}

const TASTE_KEEP = 30;
const TASTE_TOTAL = 60;

export async function loadCatalogueTaste(
  seeds: readonly string[],
  force = false,
  profileKey = "",
): Promise<void> {
  const key = seeds.join("|");
  if (!key) {
    _tasteGeneration += 1;
    _taste = [];
    _tasteKey = "";
    _tasteLoading = false;
    _tasteError = null;
    return;
  }
  if (!force && key === _tasteKey && (_taste.length > 0 || _tasteLoading)) {
    if (profileKey && _tasteProfileKey !== profileKey) {
      _tasteProfileKey = profileKey;
      writeTasteCache({ profileKey, seedsKey: key, tracks: _taste });
    }
    return;
  }

  const previous = force ? _taste : [];
  const generation = ++_tasteGeneration;
  _tasteKey = key;
  _tasteLoading = true;
  _tasteError = null;
  try {
    const tracks = await vynl.catalogueTaste([...seeds]);
    if (generation !== _tasteGeneration) return;
    const dismissed = readDismissedIds();
    const filtered = tracks.filter((t) => !dismissed.has(t.id));
    if (previous.length > 0) {
      const kept = shuffle(previous.filter((t) => !dismissed.has(t.id))).slice(
        0,
        TASTE_KEEP,
      );
      const keptIds = new Set(kept.map((track) => track.id));
      const fresh = filtered
        .filter((track) => !keptIds.has(track.id))
        .slice(0, TASTE_TOTAL - kept.length);
      _taste = shuffle([...kept, ...fresh]);
    } else {
      _taste = filtered;
    }
    if (profileKey) {
      _tasteProfileKey = profileKey;
      writeTasteCache({ profileKey, seedsKey: key, tracks: _taste });
    }
    registerCatalogueTracks(_taste);
  } catch (error) {
    if (generation !== _tasteGeneration) return;
    _tasteKey = "";
    _tasteError = error instanceof Error ? error.message : String(error);
    console.warn("catalogue taste failed:", error);
  } finally {
    if (generation === _tasteGeneration) _tasteLoading = false;
  }
}

const DETAIL_CACHE_CAP = 48;

function cappedSet<K, V>(map: Map<K, V>, key: K, value: V, cap: number): void {
  if (map.has(key)) map.delete(key);
  map.set(key, value);
  evictOldest(map, cap);
}

const albumCache = new Map<string, CatalogueAlbumDetail>();
const artistCache = new Map<string, CatalogueArtist>();
const playlistCache = new Map<string, CataloguePlaylistDetail>();

export async function loadCatalogueAlbum(
  id: string,
  force = false,
): Promise<CatalogueAlbumDetail> {
  if (!force) {
    const cached = albumCache.get(id);
    if (cached) return cached;
  }
  const detail = await vynl.catalogueAlbum(id);
  cappedSet(albumCache, id, detail, DETAIL_CACHE_CAP);
  registerCatalogueTracks(detail.tracks);
  return detail;
}

export async function loadCatalogueArtist(
  id: string,
  force = false,
): Promise<CatalogueArtist> {
  if (!force) {
    const cached = artistCache.get(id);
    if (cached) return cached;
  }
  const artist = await vynl.catalogueArtist(id);
  cappedSet(artistCache, id, artist, DETAIL_CACHE_CAP);
  registerCatalogueTracks([...artist.topTracks]);
  return artist;
}

export async function loadCataloguePlaylist(
  id: string,
  force = false,
): Promise<CataloguePlaylistDetail> {
  if (!force) {
    const cached = playlistCache.get(id);
    if (cached) return cached;
  }
  const detail = await vynl.cataloguePlaylist(id);
  mergePlaylistDetail(detail);
  return detail;
}

function mergePlaylistDetail(detail: CataloguePlaylistDetail): void {
  cappedSet(playlistCache, detail.playlist.id, detail, DETAIL_CACHE_CAP);
  registerCatalogueTracks(detail.tracks);
}

export function appendCataloguePlaylistPage(
  playlistId: string,
  page: CataloguePlaylistPage,
): CataloguePlaylistDetail | undefined {
  const current = playlistCache.get(playlistId);
  if (!current) return undefined;
  const seen = new Set(current.tracks.map((track) => track.id));
  const merged: CataloguePlaylistDetail = {
    ...current,
    tracks: [
      ...current.tracks,
      ...page.tracks.filter((track) => !seen.has(track.id)),
    ],
    continuation: page.continuation,
  };
  mergePlaylistDetail(merged);
  return merged;
}

export function saveCatalogueTrack(
  track: CatalogueTrack,
  playlistId?: string | null,
  playlistName?: string | null,
): Promise<CatalogueAddResult> {
  return vynl.catalogueAddToPlaylist(track, playlistId, playlistName);
}

export async function refreshAfterCatalogueSave(): Promise<void> {
  await Promise.allSettled([refreshLibrary(), refreshPlaylists()]);
}

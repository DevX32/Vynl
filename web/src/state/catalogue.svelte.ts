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
import { refreshLibrary } from "./library.svelte";
import { refreshPlaylists } from "./playlists.svelte";

const REGISTRY_KEY = "vynl.catalogue.registry.v1";
const REGISTRY_LIMIT = 800;
const CATALOGUE_PREFIX = "ytm:";

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
      duration: track.duration,
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

function registryIndex(): Map<string, LibraryTrack> {
  if (_indexRevision !== _revision) {
    _index = new Map(_tracks.map((track) => [track.id, track]));
    _indexRevision = _revision;
  }
  return _index;
}

function persistRegistry(): void {
  if (typeof localStorage === "undefined") return;
  try {
    const stored: StoredTrack[] = _tracks.slice(-REGISTRY_LIMIT).map((track) => ({
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

function libraryTrackOf(track: CatalogueTrack): LibraryTrack {
  return {
    id: track.id,
    path: `${CATALOGUE_PREFIX}${track.id}`,
    title: track.title,
    artist: track.artist,
    album: track.album?.name ?? "",
    year: null,
    duration: track.duration ?? 0,
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

export function registerCatalogueTracks(tracks: readonly CatalogueTrack[]): void {
  if (tracks.length === 0) return;
  const next = new Map(registryIndex());
  let changed = false;
  for (const track of tracks) {
    if (!track.id) continue;
    const entry = libraryTrackOf(track);
    const current = next.get(entry.id);
    if (current && sameMetadata(current, entry)) continue;
    next.set(entry.id, entry);
    changed = true;
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

export function cataloguePath(id: string): string {
  return `${CATALOGUE_PREFIX}${id}`;
}

export function isCataloguePath(path: string | null | undefined): boolean {
  return !!path && path.startsWith(CATALOGUE_PREFIX);
}

export function catalogueIdFromPath(path: string | null | undefined): string | null {
  if (!path || !path.startsWith(CATALOGUE_PREFIX)) return null;
  const id = path.slice(CATALOGUE_PREFIX.length);
  return id.length > 0 ? id : null;
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

export function clearCatalogueSearch(): void {
  _searchGeneration += 1;
  _searchQuery = "";
  _results = null;
  _searching = false;
  _searchError = null;
}

let _taste = $state<CatalogueTrack[]>([]);
let _tasteKey = $state("");
let _tasteLoading = $state(false);
let _tasteError = $state<string | null>(null);
let _tasteGeneration = 0;

export function getCatalogueTaste(): readonly CatalogueTrack[] {
  return _taste;
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
}

export async function loadCatalogueTaste(
  seeds: readonly string[],
  force = false,
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
    return;
  }

  const generation = ++_tasteGeneration;
  _tasteKey = key;
  _tasteLoading = true;
  _tasteError = null;
  try {
    const tracks = await vynl.catalogueTaste([...seeds]);
    if (generation !== _tasteGeneration) return;
    _taste = tracks;
    registerCatalogueTracks(tracks);
  } catch (error) {
    if (generation !== _tasteGeneration) return;
    _tasteError = error instanceof Error ? error.message : String(error);
    console.warn("catalogue taste failed:", error);
  } finally {
    if (generation === _tasteGeneration) _tasteLoading = false;
  }
}

const albumCache = new Map<string, CatalogueAlbumDetail>();
const artistCache = new Map<string, CatalogueArtist>();
const playlistCache = new Map<string, CataloguePlaylistDetail>();

export function getCachedAlbum(id: string): CatalogueAlbumDetail | undefined {
  return albumCache.get(id);
}

export function getCachedArtist(id: string): CatalogueArtist | undefined {
  return artistCache.get(id);
}

export function getCachedPlaylist(id: string): CataloguePlaylistDetail | undefined {
  return playlistCache.get(id);
}

export async function loadCatalogueAlbum(
  id: string,
  force = false,
): Promise<CatalogueAlbumDetail> {
  if (!force) {
    const cached = albumCache.get(id);
    if (cached) return cached;
  }
  const detail = await vynl.catalogueAlbum(id);
  albumCache.set(id, detail);
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
  artistCache.set(id, artist);
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
  playlistCache.set(detail.playlist.id, detail);
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

export function clearCatalogueCache(): void {
  albumCache.clear();
  artistCache.clear();
  playlistCache.clear();
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

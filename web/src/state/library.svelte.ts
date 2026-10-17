import type { LibraryTrack } from "@lib/types";
import { vynl } from "@lib/vynl";
import { emitAppEvent } from "@lib/plugins/events";
import { invalidateArtistCache } from "@lib/artist-cache";

let _library = $state<LibraryTrack[]>([]);
let _libraryError = $state<string | null>(null);
let _listenerReady: Promise<void> | null = null;
let _libraryRevision = 0;
let _refreshPromise: Promise<void> | null = null;
let _libraryLoaded = false;
let _libraryLoadedPromise: Promise<void> | null = null;
let _resolveLibraryLoaded: (() => void) | null = null;

function markLibraryLoaded(): void {
  if (_libraryLoaded) return;
  _libraryLoaded = true;
  _resolveLibraryLoaded?.();
  _resolveLibraryLoaded = null;
}

export function whenLibraryLoaded(): Promise<void> {
  if (_libraryLoaded) return Promise.resolve();
  if (!_libraryLoadedPromise) {
    _libraryLoadedPromise = new Promise<void>((resolve) => {
      _resolveLibraryLoaded = resolve;
    });
    void refreshLibrary();
  }
  return _libraryLoadedPromise;
}

function sameTrack(a: LibraryTrack, b: LibraryTrack): boolean {
  return (
    a.id === b.id &&
    a.path === b.path &&
    a.title === b.title &&
    a.artist === b.artist &&
    a.album === b.album &&
    a.year === b.year &&
    a.duration === b.duration &&
    a.cover === b.cover &&
    a.trackNumber === b.trackNumber &&
    a.lyrics === b.lyrics &&
    a.ext === b.ext &&
    a.mtime === b.mtime &&
    a.addedAt === b.addedAt
  );
}

function libraryChanged(next: LibraryTrack[]): boolean {
  if (next.length !== _library.length) return true;
  return next.some((track, index) => {
    const current = _library[index];
    return !current || !sameTrack(track, current);
  });
}

function staleArtists(prev: LibraryTrack[], next: LibraryTrack[]): string[] {
  const prevById = new Map(prev.map((t) => [t.id, t.artist]));
  const out: string[] = [];
  for (const track of next) {
    const before = prevById.get(track.id);
    if (before === undefined || before !== track.artist) {
      const name = track.artist.trim();
      if (name && !out.includes(name)) out.push(track.artist);
    }
  }
  return out;
}

type LibraryListener = (library: LibraryTrack[]) => void;

let _listeners: LibraryListener[] = [];

function notifyLibraryChanged(next: LibraryTrack[]): void {
  if (_listeners.length === 0) return;
  for (const fn of [..._listeners]) {
    try {
      fn(next);
    } catch (e) {
      console.warn("library listener failed:", e);
    }
  }
}

export function onLibraryChanged(fn: LibraryListener): () => void {
  _listeners.push(fn);
  return () => {
    _listeners = _listeners.filter((f) => f !== fn);
  };
}

function applyLibrary(next: LibraryTrack[]): void {
  _library = next;
  _libraryRevision += 1;
  notifyLibraryChanged(next);
}

function startListening(): Promise<void> {
  if (_listenerReady) return _listenerReady;
  _listenerReady = new Promise((resolve) => {
    vynl.onLibraryUpdated((tracks) => {
      markLibraryLoaded();
      if (!libraryChanged(tracks)) return;
      const stale = staleArtists(_library, tracks);
      applyLibrary(tracks);
      if (stale.length > 0) invalidateArtistCache(stale);
      emitAppEvent("library-updated", { count: tracks.length });
    }, resolve);
  });
  return _listenerReady;
}

export function refreshLibrary(): Promise<void> {
  const listenerReady = startListening();
  if (_refreshPromise) return _refreshPromise;

  const requestRevision = _libraryRevision;
  const promise = (async () => {
    try {
      await listenerReady;
      const tracks = await vynl.getLibrary();
      if (_libraryRevision === requestRevision) {
        _libraryError = null;
        if (libraryChanged(tracks)) applyLibrary(tracks);
      }
    } catch (e) {
      console.warn("refreshLibrary failed:", e);
      if (_libraryRevision === requestRevision) {
        _libraryError = e instanceof Error ? e.message : String(e);
      }
    } finally {
      _refreshPromise = null;
      markLibraryLoaded();
    }
  })();
  _refreshPromise = promise;
  return promise;
}

export function getLibrary(): LibraryTrack[] {
  return _library;
}

export function getLibraryError(): string | null {
  return _libraryError;
}

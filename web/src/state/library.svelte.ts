import type { LibraryTrack } from "@lib/types";
import { vynl } from "@lib/vynl";
import { emitAppEvent } from "@lib/plugins/events";

let _library = $state<LibraryTrack[]>([]);
let _listenerReady: Promise<void> | null = null;
let _libraryRevision = 0;
let _refreshPromise: Promise<void> | null = null;

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

function startListening(): Promise<void> {
  if (_listenerReady) return _listenerReady;
  _listenerReady = new Promise((resolve) => {
    vynl.onLibraryUpdated((tracks) => {
      const changed =
        tracks.length !== _library.length ||
        tracks.some((track, index) => {
          const current = _library[index];
          return !current || !sameTrack(track, current);
        });
      _library = tracks;
      _libraryRevision += 1;
      if (changed) emitAppEvent("library-updated", { count: tracks.length });
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
        _library = tracks;
      }
    } catch (e) {
      console.warn("refreshLibrary failed:", e);
    } finally {
      _refreshPromise = null;
    }
  })();
  _refreshPromise = promise;
  return promise;
}

export function getLibrary(): LibraryTrack[] {
  return _library;
}

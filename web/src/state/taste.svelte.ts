import type { CatalogueTrack, LibraryTrack, TasteEntry } from "@lib/types";
import { vynl } from "@lib/vynl";
import { libraryTrackOf } from "./catalogue.svelte";

const TASTE_KEY = "vynl.taste.v1";
const MAX_ENTRIES = 160;

function isArtistRef(value: unknown): boolean {
  if (!value || typeof value !== "object") return false;
  const ref = value as Record<string, unknown>;
  return (
    typeof ref.name === "string" &&
    (ref.id === null || typeof ref.id === "string")
  );
}

function isTrack(value: unknown): value is CatalogueTrack {
  if (!value || typeof value !== "object") return false;
  const track = value as Record<string, unknown>;
  const album = track.album as Record<string, unknown> | null;
  return (
    typeof track.id === "string" &&
    track.id.length > 0 &&
    typeof track.title === "string" &&
    typeof track.artist === "string" &&
    Array.isArray(track.artists) &&
    track.artists.every(isArtistRef) &&
    (album === null ||
      (typeof album === "object" &&
        typeof album.name === "string" &&
        (album.id === null || typeof album.id === "string"))) &&
    (track.duration === null || typeof track.duration === "number") &&
    (track.cover === null || typeof track.cover === "string") &&
    typeof track.explicit === "boolean"
  );
}

function isEntry(value: unknown): value is TasteEntry {
  if (!value || typeof value !== "object") return false;
  const entry = value as Record<string, unknown>;
  return (
    isTrack(entry.track) &&
    typeof entry.plays === "number" &&
    Number.isFinite(entry.plays) &&
    entry.plays >= 0 &&
    typeof entry.lastPlayed === "number" &&
    Number.isFinite(entry.lastPlayed) &&
    typeof entry.liked === "boolean" &&
    (entry.likedAt === null || typeof entry.likedAt === "number")
  );
}

function loadTaste(): TasteEntry[] {
  if (typeof localStorage === "undefined") return [];
  try {
    const raw = localStorage.getItem(TASTE_KEY);
    if (!raw) return [];
    const parsed: unknown = JSON.parse(raw);
    if (!Array.isArray(parsed)) return [];
    return parsed.filter(isEntry);
  } catch (error) {
    console.warn("taste profile load failed:", error);
    return [];
  }
}

let _entries = $state<TasteEntry[]>(loadTaste());

function persist(): void {
  if (typeof localStorage === "undefined") return;
  try {
    localStorage.setItem(TASTE_KEY, JSON.stringify(_entries));
  } catch (error) {
    console.warn("taste profile save failed:", error);
  }
  void vynl.tasteSave(_entries).catch((error) => {
    console.warn("taste profile sync failed:", error);
  });
}

function commit(next: TasteEntry[]): void {
  _entries = next;
  persist();
}

export async function hydrateTaste(): Promise<void> {
  try {
    const stored = await vynl.tasteLoad();
    if (stored.length > 0) {
      _entries = stored;
      persist();
      return;
    }
    if (_entries.length > 0) {
      await vynl.tasteSave(_entries);
    }
  } catch (error) {
    console.warn("taste profile hydrate failed:", error);
  }
}

function mergeTrack(current: CatalogueTrack, patch: CatalogueTrack): CatalogueTrack {
  const artists =
    patch.artists.length > 0
      ? patch.artists.map((artist) => {
          if (artist.id) return artist;
          const match = current.artists.find(
            (known) => known.name === artist.name && known.id,
          );
          return match ?? artist;
        })
      : current.artists;
  const album = patch.album
    ? {
        name: patch.album.name || current.album?.name || "",
        id: patch.album.id ?? current.album?.id ?? null,
      }
    : current.album;
  return {
    ...current,
    ...patch,
    artists,
    album,
    cover: patch.cover ?? current.cover,
  };
}

function trim(entries: TasteEntry[]): TasteEntry[] {
  if (entries.length <= MAX_ENTRIES) return entries;
  const kept = entries
    .slice()
    .sort((a, b) => {
      if (a.liked !== b.liked) return a.liked ? -1 : 1;
      return b.lastPlayed - a.lastPlayed;
    })
    .slice(0, MAX_ENTRIES);
  return kept;
}

export function recordTastePlay(track: CatalogueTrack): void {
  if (!track.id) return;
  const now = Date.now();
  const existing = _entries.find((entry) => entry.track.id === track.id);
  if (existing) {
    commit(
      _entries.map((entry) =>
        entry.track.id === track.id
          ? {
              ...entry,
              track: mergeTrack(entry.track, track),
              plays: entry.plays + 1,
              lastPlayed: now,
            }
          : entry,
      ),
    );
    return;
  }

  commit(
    trim([
      ..._entries,
      {
        track,
        plays: 1,
        lastPlayed: now,
        liked: false,
        likedAt: null,
      },
    ]),
  );
}

export function getTasteLikedIds(): ReadonlySet<string> {
  return new Set(
    _entries.filter((entry) => entry.liked).map((entry) => entry.track.id),
  );
}

export function toggleTasteLike(track: CatalogueTrack): boolean {
  const now = Date.now();
  const existing = _entries.find((entry) => entry.track.id === track.id);
  if (!existing) {
    commit([
      ..._entries,
      {
        track,
        plays: 0,
        lastPlayed: 0,
        liked: true,
        likedAt: now,
      },
    ]);
    return true;
  }

  const liked = !existing.liked;
  commit(
    _entries
      .map((entry) =>
        entry.track.id === track.id
          ? {
              ...entry,
              track: mergeTrack(entry.track, track),
              liked,
              likedAt: liked ? now : null,
            }
          : entry,
      )
      .filter((entry) => entry.liked || entry.plays > 0),
  );
  return liked;
}

export function getLikedTracks(): CatalogueTrack[] {
  return _entries
    .filter((entry) => entry.liked)
    .sort((a, b) => (b.likedAt ?? 0) - (a.likedAt ?? 0))
    .map((entry) => entry.track);
}

export function getRecentlyPlayedTracks(limit = 12): LibraryTrack[] {
  return _entries
    .filter((entry) => entry.plays > 0)
    .sort((a, b) => b.lastPlayed - a.lastPlayed)
    .slice(0, limit)
    .map((entry) => ({
      ...libraryTrackOf(entry.track),
      addedAt: entry.lastPlayed,
    }));
}

export function getTasteSeeds(limit = 6): string[] {
  const now = Date.now();
  return _entries
    .filter((entry) => entry.liked || entry.plays > 0)
    .map((entry) => {
      const daysSincePlay = entry.lastPlayed
        ? (now - entry.lastPlayed) / 86_400_000
        : 60;
      const recency = Math.max(0, 8 - daysSincePlay) / 4;
      const score =
        (entry.liked ? 6 : 0) + Math.min(entry.plays, 8) + recency;
      return { id: entry.track.id, score, lastPlayed: entry.lastPlayed };
    })
    .sort((a, b) => b.score - a.score || b.lastPlayed - a.lastPlayed)
    .slice(0, limit)
    .map((entry) => entry.id);
}

export function getTasteProfileKey(): string {
  const liked = _entries
    .filter((entry) => entry.liked)
    .map((entry) => entry.track.id)
    .sort();
  const played = _entries.some((entry) => entry.plays > 0) ? "p" : "-";
  return `${liked.join(",")}|${played}`;
}

export function getTasteSummary(): {
  liked: number;
  played: number;
  entries: number;
} {
  return {
    liked: _entries.filter((entry) => entry.liked).length,
    played: _entries.filter((entry) => entry.plays > 0).length,
    entries: _entries.length,
  };
}

export function resetTaste(): void {
  commit([]);
}

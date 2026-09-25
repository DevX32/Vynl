import type { LibraryTrack, NowPlaying } from "@lib/types";
import { RPC_THROTTLE } from "@lib/constants";
import { moveItem } from "@lib/reorder";
import { vynl } from "@lib/vynl";
import { getLibrary } from "./library.svelte";
import type { LoopMode } from "./player.svelte";

let _state = $state<NowPlaying | null>(null);
let _userQueue = $state<string[]>([]);
let _contextQueue = $state<string[]>([]);
let _contextIndex = $state(0);
let _currentTime = $state(0);
let _shuffleUpcoming = $state<string[]>([]);
let _shuffleHistory = $state<string[]>([]);
let _shuffleRedo = $state<string[]>([]);

let saveTimer: ReturnType<typeof setTimeout> | null = null;
const SAVE_DEBOUNCE = 3000;

function persistNowPlaying(): void {
  if (saveTimer) clearTimeout(saveTimer);
  saveTimer = setTimeout(() => {
    if (!_state) {
      void vynl.clearNowPlaying();
      return;
    }
    void vynl.saveNowPlaying({
      id: _state.id,
      path: _state.path,
      time: _currentTime,
      queue: _contextQueue,
      userQueue: _userQueue,
      contextQueue: _contextQueue,
      contextIndex: _contextIndex,
      timestamp: Date.now(),
    });
  }, SAVE_DEBOUNCE);
}

export function checkpointNowPlaying(): void {
  persistNowPlaying();
}

const currentId = $derived(_state?.id ?? null);
const isTrackPlaying = $derived(_state?.playing ?? false);
const currentTime = $derived(_currentTime);

export function getCurrentTrack(): NowPlaying | null {
  return _state;
}
export function getCurrentId(): string | null {
  return currentId;
}
export function getIsTrackPlaying(): boolean {
  return isTrackPlaying;
}
export function getCurrentTime(): number {
  return currentTime;
}

function tracksForPaths(
  paths: readonly string[],
  library: LibraryTrack[],
  seen: Set<string>,
): LibraryTrack[] {
  const result: LibraryTrack[] = [];
  for (const p of paths) {
    const t = library.find((x) => x.path === p);
    if (t && !seen.has(t.id)) {
      seen.add(t.id);
      result.push(t);
    }
  }
  return result;
}

const SHUFFLE_HISTORY_LIMIT = 500;

function shuffled(paths: readonly string[]): string[] {
  const result = [...paths];
  for (let i = result.length - 1; i > 0; i--) {
    const j = Math.floor(Math.random() * (i + 1));
    [result[i], result[j]] = [result[j], result[i]];
  }
  return result;
}

function startShuffleSession(
  contextQueue: readonly string[],
  startIndex: number,
  currentPath: string | null = contextQueue[startIndex] ?? null,
): void {
  const library = getLibrary();
  const queued = new Set(_userQueue);
  const seen = new Set<string>();
  const upcoming = contextQueue.slice(startIndex + 1).filter((path) => {
    if (path === currentPath || queued.has(path) || seen.has(path)) return false;
    if (!library.some((track) => track.path === path)) return false;
    seen.add(path);
    return true;
  });

  _shuffleUpcoming = shuffled(upcoming);
  _shuffleHistory = currentPath ? [currentPath] : [];
  _shuffleRedo = [];
}

function refillShuffleSession(currentPath: string | null): void {
  const library = getLibrary();
  const queued = new Set(_userQueue);
  const seen = new Set<string>();
  const candidates = _contextQueue.filter((path) => {
    if (path === currentPath || queued.has(path) || seen.has(path)) return false;
    if (!library.some((track) => track.path === path)) return false;
    seen.add(path);
    return true;
  });

  _shuffleUpcoming = shuffled(candidates);
  if (
    _shuffleUpcoming.length === 0 &&
    currentPath &&
    !queued.has(currentPath) &&
    library.some((track) => track.path === currentPath)
  ) {
    _shuffleUpcoming = [currentPath];
  }
}

function takeShufflePath(): string | null {
  const context = new Set(_contextQueue);
  const library = getLibrary();
  _shuffleUpcoming = _shuffleUpcoming.filter(
    (path) => context.has(path) && library.some((track) => track.path === path),
  );

  const path = _shuffleUpcoming[0] ?? null;
  if (path) _shuffleUpcoming = _shuffleUpcoming.slice(1);
  return path;
}

export function recordShuffleTrack(path: string): void {
  _shuffleUpcoming = _shuffleUpcoming.filter((queued) => queued !== path);

  if (_shuffleRedo[0] === path) {
    _shuffleRedo = _shuffleRedo.slice(1);
    return;
  }

  _shuffleRedo = [];
  if (_shuffleHistory[_shuffleHistory.length - 1] !== path) {
    _shuffleHistory = [..._shuffleHistory, path].slice(-SHUFFLE_HISTORY_LIMIT);
  }
}

export function resetShuffleForMode(): void {
  startShuffleSession(_contextQueue, _contextIndex, _state?.path ?? null);
}

export function getUserQueue(): LibraryTrack[] {
  const seen = new Set<string>();
  return tracksForPaths(_userQueue, getLibrary(), seen);
}

export function getContextUpcoming(shuffle = false): LibraryTrack[] {
  const library = getLibrary();
  const seen = new Set<string>();
  if (currentId) seen.add(currentId);
  if (shuffle) {
    tracksForPaths(_userQueue, library, seen);
    return tracksForPaths(_shuffleUpcoming, library, seen);
  }
  tracksForPaths(_userQueue, library, seen);
  return tracksForPaths(_contextQueue.slice(_contextIndex + 1), library, seen);
}

let seekHandler: ((time: number) => void) | null = null;
let playHandler: ((id: string, queue?: string[]) => void) | null = null;

export function setContextQueue(
  paths: string[],
  startIndex: number,
  currentPath?: string,
): void {
  _contextQueue = [...paths];
  _contextIndex = startIndex;
  startShuffleSession(_contextQueue, startIndex, currentPath);
}

let lastRpc = 0;

function syncRpc(force = false): void {
  const now = Date.now();
  if (!force && now - lastRpc < RPC_THROTTLE) return;
  lastRpc = now;
  if (!_state || !_state.playing) {
    void vynl.rpcUpdate(null);
    return;
  }
  void vynl.rpcUpdate({
    title: _state.title,
    artist: _state.artist,
    album: _state.album,
    duration: _state.duration,
    time: _currentTime,
    playing: _state.playing,
    cover: _state.cover,
  });
}

export function nowPlayingFromTrack(track: LibraryTrack, time = 0): NowPlaying {
  return {
    id: track.id,
    title: track.title,
    artist: track.artist,
    album: track.album,
    year: track.year,
    duration: track.duration,
    path: track.path,
    trackNumber: track.trackNumber,
    time,
    playing: false,
    lyrics: track.lyrics ?? null,
    cover: track.cover ?? null,
  };
}

export function setNowPlaying(t: NowPlaying): void {
  _state = t;
  _currentTime = t.time;
  syncRpc(true);
  persistNowPlaying();
}

let _seekDragging = false;

export function setSeekDragging(v: boolean): void {
  _seekDragging = v;
}

export function isSeekDragging(): boolean {
  return _seekDragging;
}

export function getUserQueuePaths(): string[] {
  return _userQueue;
}

export function getContextQueuePaths(): string[] {
  return _contextQueue;
}

export function getContextIndex(): number {
  return _contextIndex;
}

export function setDragTime(time: number): void {
  if (!_state) return;
  _currentTime = time;
}

export function updateTime(time: number, forcePersist = false): void {
  if (!_state) return;
  const seeked = forcePersist || Math.abs(time - _currentTime) > 2;
  _currentTime = time;
  syncRpc(seeked);
  if (seeked) persistNowPlaying();
}

export function updatePlaying(playing: boolean): void {
  if (!_state) return;
  _state = { ..._state, playing };
  if (!playing && typeof navigator !== "undefined" && "mediaSession" in navigator) {
    navigator.mediaSession.playbackState = "paused";
  }
  syncRpc(true);
  persistNowPlaying();
}

export function patchNowPlaying(
  patch: Partial<Pick<NowPlaying, "cover" | "lyrics" | "title" | "artist" | "album" | "duration">>,
): void {
  if (!_state) return;
  _state = { ..._state, ...patch };
  syncRpc(false);
}

export function setSeekHandler(fn: ((time: number) => void) | null): void {
  seekHandler = fn;
}

export function seekTo(time: number): void {
  seekHandler?.(time);
}

export function setPlayHandler(
  fn: ((id: string, queue?: string[]) => void) | null,
): void {
  playHandler = fn;
}

export function requestPlay(id: string, queue?: string[]): void {
  playHandler?.(id, queue);
}

export function addToQueue(path: string): void {
  if (!_userQueue.includes(path)) {
    _userQueue = [..._userQueue, path];
  }
}

export function addToQueueFront(path: string): void {
  if (!_userQueue.includes(path)) {
    _userQueue = [path, ..._userQueue];
  }
}

export function removeFromUserQueue(path: string): void {
  const idx = _userQueue.indexOf(path);
  if (idx < 0) return;
  const next = [..._userQueue];
  next.splice(idx, 1);
  _userQueue = next;
  persistNowPlaying();
}

export function removeFromContextQueue(path: string): void {
  const idx = _contextQueue.findIndex((p, i) => i > _contextIndex && p === path);
  if (idx < 0) return;
  const next = [..._contextQueue];
  next.splice(idx, 1);
  _contextQueue = next;
  _shuffleUpcoming = _shuffleUpcoming.filter((queued) => queued !== path);
  persistNowPlaying();
}

export function reorderUserQueue(fromPath: string, toPath: string): void {
  const from = _userQueue.indexOf(fromPath);
  const to = _userQueue.indexOf(toPath);
  if (from === to || from < 0 || to < 0) return;
  _userQueue = moveItem(_userQueue, from, to);
  persistNowPlaying();
}

export function reorderContextQueue(fromPath: string, toPath: string): void {
  const shuffleFrom = _shuffleUpcoming.indexOf(fromPath);
  const shuffleTo = _shuffleUpcoming.indexOf(toPath);
  if (shuffleFrom >= 0 && shuffleTo >= 0 && shuffleFrom !== shuffleTo) {
    _shuffleUpcoming = moveItem(_shuffleUpcoming, shuffleFrom, shuffleTo);
  }

  const from = _contextQueue.findIndex(
    (p, i) => i > _contextIndex && p === fromPath,
  );
  const to = _contextQueue.findIndex(
    (p, i) => i > _contextIndex && p === toPath,
  );
  if (from === to || from < 0 || to < 0) return;
  _contextQueue = moveItem(_contextQueue, from, to);
  persistNowPlaying();
}

export function pickNextId(
  userQueue: string[],
  contextQueue: string[],
  contextIndex: number,
  currentId: string | null,
  shuffle: boolean,
  loop: LoopMode,
): string | null {
  const library = getLibrary();

  if (loop === "one" && currentId) {
    return currentId;
  }

  if (shuffle) {
    const queuedPath = userQueue.find((path) =>
      library.some((track) => track.path === path),
    );
    if (queuedPath) {
      return library.find((track) => track.path === queuedPath)?.id ?? null;
    }

    _shuffleRedo = _shuffleRedo.filter((path) =>
      library.some((track) => track.path === path),
    );
    const redoPath = _shuffleRedo[0];
    if (redoPath) {
      return library.find((track) => track.path === redoPath)?.id ?? null;
    }

    let path = takeShufflePath();
    if (!path && loop === "all") {
      const currentPath =
        library.find((track) => track.id === currentId)?.path ?? null;
      refillShuffleSession(currentPath);
      path = takeShufflePath();
    }
    if (!path) return null;
    return library.find((track) => track.path === path)?.id ?? null;
  }

  if (userQueue.length > 0) {
    const t = library.find((x) => x.path === userQueue[0]);
    return t?.id ?? null;
  }
  if (contextIndex + 1 < contextQueue.length) {
    const t = library.find((x) => x.path === contextQueue[contextIndex + 1]);
    return t?.id ?? null;
  }
  if (loop === "all" && contextQueue.length > 0) {
    const t = library.find((x) => x.path === contextQueue[0]);
    return t?.id ?? null;
  }
  return null;
}

export function pickPrevId(
  currentId: string | null,
  contextQueue: string[],
  contextIndex: number,
  shuffle: boolean,
  loop: LoopMode,
): string | null {
  const library = getLibrary();
  if (shuffle) {
    const currentPath = library.find((track) => track.id === currentId)?.path;
    if (!currentPath) return null;
    _shuffleHistory = _shuffleHistory.filter((path) =>
      library.some((track) => track.path === path),
    );
    if (_shuffleHistory[_shuffleHistory.length - 1] !== currentPath) {
      _shuffleHistory = [..._shuffleHistory, currentPath].slice(
        -SHUFFLE_HISTORY_LIMIT,
      );
    }
    if (_shuffleHistory.length <= 1) return null;

    _shuffleHistory = _shuffleHistory.slice(0, -1);
    const previousPath = _shuffleHistory[_shuffleHistory.length - 1];
    _shuffleRedo = [currentPath, ..._shuffleRedo].slice(0, SHUFFLE_HISTORY_LIMIT);
    return library.find((track) => track.path === previousPath)?.id ?? null;
  }
  if (contextIndex > 0) {
    const t = library.find((x) => x.path === contextQueue[contextIndex - 1]);
    return t?.id ?? null;
  }
  if (loop === "all" && contextQueue.length > 0) {
    const t = library.find((x) => x.path === contextQueue[contextQueue.length - 1]);
    return t?.id ?? null;
  }
  return null;
}

export function advanceContextIndex(newTrackId: string): void {
  const library = getLibrary();
  const track = library.find((t) => t.id === newTrackId);
  if (!track) return;
  const idx = _contextQueue.indexOf(track.path);
  if (idx >= 0) {
    _contextIndex = idx;
  }
}

export function clearUserQueue(): void {
  _userQueue = [];
}

export async function restoreNowPlaying(): Promise<{
  id: string;
  queue: string[];
} | null> {
  const saved = await vynl.loadNowPlaying();
  if (!saved) return null;
  const library = getLibrary();
  const track = library.find((t) => t.id === saved.id);
  if (!track) {
    void vynl.clearNowPlaying();
    return null;
  }
  setSeekDragging(false);
  setNowPlaying(nowPlayingFromTrack(track, saved.time ?? 0));
  if (saved.userQueue && saved.userQueue.length > 0) {
    _userQueue = saved.userQueue;
  }
  if (saved.contextQueue && saved.contextQueue.length > 0) {
    _contextQueue = saved.contextQueue;
    _contextIndex = saved.contextIndex ?? 0;
  } else if (saved.queue && saved.queue.length > 0) {
    _contextQueue = saved.queue;
    _contextIndex = 0;
  }
  startShuffleSession(_contextQueue, _contextIndex, saved.path);
  return { id: saved.id, queue: saved.queue };
}

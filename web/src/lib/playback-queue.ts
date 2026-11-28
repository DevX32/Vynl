import { getCurrentTrack, getCurrentTime, removeFromUserQueue, removeFromContextQueue, reorderContextQueue, reorderUserQueue, setNowPlaying, advanceContextIndex, pickNextId, pickPrevId, getUserQueuePaths, getContextQueuePaths, getContextIndex, recordShuffleTrack, nowPlayingFromTrack, findTrackById, peekNextId, isSeekDragging } from "@state/now-playing.svelte";
import { getCrossfade, getCurrentLoop, getCurrentShuffle, setPlayError } from "@state/player.svelte";
import type { LoopMode } from "@state/player.svelte";
import { getSleepMode } from "@state/sleep-timer.svelte";
import { vynl } from "@lib/vynl";
import { catalogueIdFromPath as catalogueIdOf } from "@lib/catalogue";
import type { LibraryTrack } from "@lib/types";
import { seekState, seekToTime, cancelSeek } from "./playback-seek";
import { updateMediaSession } from "./playback-media";
import { prebufferState, schedulePrebuffers } from "./playback-prebuffer";
import { playbackPhase, resetCrossfade, CROSSFADE_TAIL } from "./playback-reconcile";
import { clearRetry } from "./playback-retry";
import { ps } from "./playback-state";
type QueueDeps = { loadAndPlayNative: (path: string, gen: number, seekTo?: number) => void };
let deps: QueueDeps = { loadAndPlayNative: () => {} };
export function initQueue(d: QueueDeps): void { deps = d; }
function advanceNext(automatic: boolean): void {
  const nextTrack = resolveNextTrack(automatic);
  if (!nextTrack) return;
  startTrack(nextTrack);
}

function resolveNextTrack(
  automatic: boolean,
  peek = false,
): LibraryTrack | null {
  const np = getCurrentTrack();
  const selected = getCurrentLoop();
  const loop: LoopMode = !automatic && selected === "one" ? "off" : selected;
  const args = [
    getUserQueuePaths(),
    getContextQueuePaths(),
    getContextIndex(),
    np?.id ?? null,
    getCurrentShuffle(),
    loop,
  ] as const;
  const nextId = peek ? peekNextId(...args) : pickNextId(...args);
  if (!nextId) return null;
  return findTrackById(nextId);
}

function consumeNextTrack(nextTrack: LibraryTrack): void {
  if (getUserQueuePaths().includes(nextTrack.path)) {
    removeFromUserQueue(nextTrack.path);
  }
  advanceContextIndex(nextTrack.id);
  if (getCurrentShuffle()) recordShuffleTrack(nextTrack.path);
}

function startTrack(nextTrack: LibraryTrack): void {
  consumeNextTrack(nextTrack);
  clearRetry();
  setPlayError(null);
  playbackPhase.pendingStreamError = null;
  resetCrossfade();
  cancelSeek(false);
  const gen = ++ps.playGeneration;
  setNowPlaying({ ...nowPlayingFromTrack(nextTrack), playing: true });
  deps.loadAndPlayNative(nextTrack.path, gen);
}

export function maybeStartCrossfade(position: number): boolean {
  const fade = getCrossfade();
  if (fade <= 0 || playbackPhase.crossfadeTried) return false;

  const np = getCurrentTrack();
  if (
    !np?.playing ||
    !np.path ||
    !Number.isFinite(np.duration) ||
    np.duration <= 0
  )
    return false;
  if (isSeekDragging() || seekState.pending) return false;
  if (getCurrentLoop() === "one") return false;
  if (getSleepMode() === "end") return false;

  const remaining = np.duration - position;
  if (!Number.isFinite(remaining)) return false;
  if (remaining <= 0 || remaining > fade + CROSSFADE_TAIL) return false;

  const nextTrack = resolveNextTrack(true, true);
  if (!nextTrack || nextTrack.path === np.path) return false;
  if (nextTrack.duration > 0 && nextTrack.duration < fade) return false;

  const nextVideoId = catalogueIdOf(nextTrack.path);
  if (nextVideoId) {
    if (prebufferState.get(nextVideoId)?.status !== "ready") return false;
  }

  playbackPhase.crossfadeTried = true;

  const generation = ++ps.playGeneration;
  const pauseToken = ps.pauseIntent;
  const crossfade = nextVideoId
    ? vynl.catalogueCrossfadeStream(
        nextVideoId,
        nextTrack.duration,
        generation,
        fade,
      )
    : vynl.playerCrossfade(nextTrack.path, generation, fade);

  void crossfade
    .then(() => {
      if (nextVideoId) prebufferState.delete(nextVideoId);
      if (generation !== ps.playGeneration) return;
      consumeNextTrack(nextTrack);
      setNowPlaying({
        ...nowPlayingFromTrack(nextTrack),
        playing: pauseToken === ps.pauseIntent,
      });
      updateMediaSession();
      resetCrossfade();
      schedulePrebuffers();
    })
    .catch((error) => {
      console.warn("[vynl] crossfade failed:", error);
      if (ps.playGeneration === generation && generation > 0) {
        ps.playGeneration = generation - 1;
        playbackPhase.crossfadeTried = false;
        schedulePrebuffers(true);
      }
      if (nextVideoId) prebufferState.delete(nextVideoId);
    });

  return true;
}

export function next(): void {
  advanceNext(false);
}

export function nextAutomatic(): void {
  advanceNext(true);
}

export function prev(): void {
  const curTime = getCurrentTime();
  if (curTime > 3) {
    seekToTime(0);
    return;
  }

  const np = getCurrentTrack();
  const shuffle = getCurrentShuffle();
  const prevId = pickPrevId(
    np?.id ?? null,
    getContextQueuePaths(),
    getContextIndex(),
    shuffle,
    getCurrentLoop(),
  );
  if (!prevId) return;

  const prevTrack = findTrackById(prevId);
  if (!prevTrack) return;

  if (getUserQueuePaths().includes(prevTrack.path)) {
    removeFromUserQueue(prevTrack.path);
  }
  advanceContextIndex(prevId);

  clearRetry();
  setPlayError(null);
  playbackPhase.pendingStreamError = null;
  resetCrossfade();
  cancelSeek(false);
  const gen = ++ps.playGeneration;
  setNowPlaying({ ...nowPlayingFromTrack(prevTrack), playing: true });
  deps.loadAndPlayNative(prevTrack.path, gen);
}

export function clearPlayError(): void {
  setPlayError(null);
  playbackPhase.pendingStreamError = null;
}

export function removeQueueTrack(path: string): void {
  if (getUserQueuePaths().includes(path)) {
    removeFromUserQueue(path);
  } else {
    removeFromContextQueue(path);
  }
}

export function reorderQueueTrack(fromPath: string, toPath: string): void {
  if (getUserQueuePaths().includes(fromPath)) {
    reorderUserQueue(fromPath, toPath);
  } else {
    reorderContextQueue(fromPath, toPath);
  }
}


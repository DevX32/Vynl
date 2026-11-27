import {
  checkpointNowPlaying,
  getCurrentTrack,
  getCurrentTime,
  getIsTrackPlaying,
  patchNowPlaying,
  removeFromUserQueue,
  removeFromContextQueue,
  reorderContextQueue,
  reorderUserQueue,
  setNowPlaying,
  setContextQueue,
  setSeekHandler,
  updatePlaying,
  updateTime,
  advanceContextIndex,
  clearUserQueue,
  pickNextId,
  pickPrevId,
  getUserQueuePaths,
  getContextQueuePaths,
  getContextIndex,
  recordShuffleTrack,
  nowPlayingFromTrack,
  findTrackById,
  peekNextId,
  isSeekDragging,
} from "@state/now-playing.svelte";
import {
  getCrossfade,
  getCurrentVolume,
  getCurrentMuted,
  getCurrentLoop,
  getCurrentShuffle,
  getPlaybackSpeed,
  getReplayGain,
  setPlayError,
  setVolume,
  toggleMute,
  onMuteChange,
  onSpeedChange,
  onReplayGainChange,
} from "@state/player.svelte";
import type { LoopMode } from "@state/player.svelte";
import {
  consumeEndOfTrack,
  getSleepMode,
  setSleepPauseHandler,
} from "@state/sleep-timer.svelte";
import { vynl } from "@lib/vynl";
import { untrack } from "svelte";

import { clamp01 } from "@lib/pointer";
import { friendlyError } from "@lib/errors";
import { catalogueIdFromPath as catalogueIdOf } from "@lib/catalogue";
import { t } from "@lib/i18n";
import type {
  CatalogueStreamInfo,
  LibraryTrack,
  PlaybackTick,
  StreamErrorEvent,
} from "@lib/types";


import { isSeekUnsupported, extractVideoId } from "./playback-utils";
import { seekState, initSeek, seekRatio, seekFlush, seekToTime, seekOffset, cancelSeek } from "./playback-seek";
export { seekRatio, seekFlush, seekToTime, seekOffset, cancelSeek };
import { initMedia, clearMediaCoverCache, syncMedia, syncMediaNow, getLastMediaSentAt, handleMediaControl, handleMediaPlay, handleMediaPause, handleMediaStop, updateMediaSession } from "./playback-media";
export { clearMediaCoverCache };




import { prebufferState, schedulePrebuffers } from "./playback-prebuffer";


onMuteChange(applyVolume);
onSpeedChange(applySpeed);
onReplayGainChange(applyReplayGain);

setSeekHandler(seekToTime);

setSleepPauseHandler(pausePlayback);

if ("mediaSession" in navigator && !IS_MINI) {
  navigator.mediaSession.setActionHandler("play", handleMediaPlay);
  navigator.mediaSession.setActionHandler("pause", handleMediaPause);
  navigator.mediaSession.setActionHandler("nexttrack", next);
  navigator.mediaSession.setActionHandler("previoustrack", prev);
  navigator.mediaSession.setActionHandler("stop", handleMediaStop);
  navigator.mediaSession.setActionHandler("seekto", (details) => {
    if (typeof details.seekTime === "number") seekToTime(details.seekTime);
  });
  navigator.mediaSession.setActionHandler("seekbackward", (details) => {
    seekOffset(-(details.seekOffset ?? 10));
  });
  navigator.mediaSession.setActionHandler("seekforward", (details) => {
    seekOffset(details.seekOffset ?? 10);
  });
}

import { playbackPhase, beginPlayReconcile, finishPlayReconcile, reconcileIsStale, resetCrossfade, CROSSFADE_TAIL } from "./playback-reconcile";
import { retryState, MAX_CONSECUTIVE_PLAY_FAILURES, clearRetry, scheduleRetry, handlePlayReject, initRetry } from "./playback-retry";
import { IS_MINI, ps, PREBUFFER_CHECK_MS } from "./playback-state";

function handleStreamError(event: StreamErrorEvent): void {
  if (event.generation !== ps.playGeneration) return;
  const np = getCurrentTrack();
  if (!np) return;
  if (catalogueIdOf(np.path) !== event.videoId) return;
  if (!np.playing) {
    playbackPhase.pendingStreamError = event;
    return;
  }
  updatePlaying(false);
  updateMediaSession();
  retryState.failures += 1;
  setPlayError(friendlyError(event.message, t("error.couldNotPlay")));
  if (retryState.failures >= MAX_CONSECUTIVE_PLAY_FAILURES) {
    return;
  }
  scheduleRetry(np.id);
}

function handleTick(tick: PlaybackTick): void {
  if (tick.generation !== ps.playGeneration) return;
  const np = getCurrentTrack();
  if (!np) return;

  if (tick.finished) {
    const streamError = playbackPhase.pendingStreamError;
    playbackPhase.pendingStreamError = null;
    if (np.playing) {
      updatePlaying(false);
      updateMediaSession();
      if (streamError && catalogueIdOf(np.path) === streamError.videoId) {
        retryState.failures += 1;
        setPlayError(
          friendlyError(streamError.message, t("error.couldNotPlay")),
        );
        if (retryState.failures >= MAX_CONSECUTIVE_PLAY_FAILURES) {
          return;
        }
        scheduleRetry(np.id);
        return;
      }
      if (consumeEndOfTrack()) {
        checkpointNowPlaying();
        return;
      }
      nextAutomatic();
    }
    return;
  }

  if (tick.playing) {
    if (retryState.failures > 0) {
      retryState.failures = 0;
    }

    if (playbackPhase.pendingPlayGeneration === ps.playGeneration) {
      finishPlayReconcile(ps.playGeneration);
    }
    if (ps.pauseIntent === 0 && !np.playing) {
      updatePlaying(true);
      updateMediaSession();
    }
  }

  if (isSeekDragging() || seekState.pending) return;

  if (Number.isFinite(tick.position)) {
    updateTime(tick.position);
  }

  if (tick.playing) {
    maybeStartCrossfade(tick.position);
  }

  if (
    tick.duration != null &&
    Number.isFinite(tick.duration) &&
    tick.duration > 0 &&
    !(np.duration > 0)
  ) {
    patchNowPlaying({ duration: tick.duration });
  }

  if (tick.playing && Date.now() - ps.lastPositionCheckpoint >= 5_000) {
    ps.lastPositionCheckpoint = Date.now();
    checkpointNowPlaying();
  }

  if (
    tick.playing &&
    !seekState.pending &&
    !isSeekDragging() &&
    Date.now() - ps.lastPrebufferCheck >= PREBUFFER_CHECK_MS
  ) {
    ps.lastPrebufferCheck = Date.now();
    schedulePrebuffers();
  }
}


if (typeof window !== "undefined" && !IS_MINI) {
  (ps.playerUnlisten as (() => void) | null)?.();
  const offTick = vynl.onPlaybackTick(handleTick);
  const offStreamError = vynl.onStreamError(handleStreamError);
  ps.playerUnlisten = () => {
    offTick();
    offStreamError();
  };
}

async function reconcilePlaybackState(): Promise<void> {
  const np = getCurrentTrack();
  if (!np || ps.reconcilePending || isSeekDragging() || seekState.pending) return;

  ps.reconcilePending = true;
  const generation = ps.playGeneration;
  const trackId = np.id;
  const isCurrent = (): boolean =>
    generation === ps.playGeneration && getCurrentTrack()?.id === trackId;

  try {
    if (!isCurrent()) return;
    let playing: boolean | null = null;
    try {
      playing = await vynl.playerIsPlaying(generation);
    } catch {
      playing = null;
    }
    if (playing === null || !isCurrent()) return;
    if (playing !== getCurrentTrack()?.playing) {
      updatePlaying(playing);
      updateMediaSession();
    }
  } finally {
    ps.reconcilePending = false;
  }
}

function onWindowActive(): void {
  void reconcilePlaybackState();
}

if (typeof document !== "undefined") {
  const onVisibilityChange = () => {
    if (document.visibilityState === "visible") onWindowActive();
  };
  document.addEventListener("visibilitychange", onVisibilityChange);
  window.addEventListener("focus", onWindowActive);
  import.meta.hot?.dispose(() => {
    document.removeEventListener("visibilitychange", onVisibilityChange);
    window.removeEventListener("focus", onWindowActive);
    ps.playerUnlisten?.();
    ps.playerUnlisten = null;
  });
}

export function applyVolume(): void {
  const raw = getCurrentMuted() ? 0 : getCurrentVolume();
  void vynl.playerSetVolume(Math.round(raw * 100)).catch(() => {});
}

export function applySpeed(): void {
  void vynl.playerSetSpeed(getPlaybackSpeed()).catch(() => {});
}

export function applyReplayGain(): void {
  void vynl.playerSetReplayGain(getReplayGain()).catch(() => {});
}

export function setVolumeAt(ratio: number): void {
  setVolume(clamp01(ratio));
  applyVolume();
}

async function resolveMissingStreamVideoId(
  path: string,
): Promise<string | null> {
  try {
    const source = await vynl.syncResolveMissingSource(path);
    const fromSource = source ? extractVideoId(source) : null;
    if (fromSource) return fromSource;
  } catch (e) {
    console.warn("[player] missing source lookup failed", e);
  }

  const np = getCurrentTrack();
  if (!np) return null;
  const query = [np.artist, np.title].filter(Boolean).join(" ").trim();
  if (!query) return null;
  try {
    const results = await vynl.searchTracks(query, 5);
    const top = results[0];
    if (top?.id) return top.id;
  } catch (e) {
    console.warn("[player] missing track search failed", e);
  }
  return null;
}

async function playNative(
  path: string,
  seekTo: number | undefined,
  generation: number,
): Promise<CatalogueStreamInfo | null> {
  if (seekTo !== undefined && (!Number.isFinite(seekTo) || seekTo < 0)) {
    throw new Error("invalid seek time");
  }
  const normalizedSeek =
    seekTo !== undefined && seekTo > 0 ? seekTo : undefined;
  const videoId = catalogueIdOf(path);
  if (videoId) {
    const np = getCurrentTrack();
    const info = await vynl.catalogueStream(
      videoId,
      normalizedSeek,
      np?.duration,
      generation,
    );
    if (normalizedSeek === undefined) prebufferState.delete(videoId);
    return info;
  }
  try {
    await vynl.playerPlay(path, normalizedSeek, generation);
  } catch (error) {
    if (generation !== ps.playGeneration) throw error;
    const missingId = await resolveMissingStreamVideoId(path);
    if (missingId) {
      const np = getCurrentTrack();
      const info = await vynl.catalogueStream(
        missingId,
        normalizedSeek,
        np?.duration,
        generation,
      );
      if (normalizedSeek === undefined) prebufferState.delete(missingId);
      return info;
    }
    if (normalizedSeek === undefined || !isSeekUnsupported(error)) {
      throw error;
    }
    await vynl.playerPlay(path, undefined, generation);
  }
  return null;
}

function loadAndPlayNative(path: string, gen: number, seekTo?: number): void {
  beginPlayReconcile(gen);
  schedulePrebuffers();
  void playNative(path, seekTo, gen)
    .then((info) => {
      finishPlayReconcile(gen);
      if (gen !== ps.playGeneration) return;
      if (info?.duration && info.duration > 0) {
        const np = getCurrentTrack();
        if (np && !(np.duration > 0)) {
          patchNowPlaying({ duration: info.duration });
        }
      }
      if (playbackPhase.pendingStreamError) return;
      updatePlaying(true);
      updateMediaSession();
      schedulePrebuffers();
    })
    .catch((err) => {
      finishPlayReconcile(gen);
      if (gen !== ps.playGeneration) return;
      handlePlayReject(err);
    });
}

export function playTrack(
  id: string,
  contextPaths?: string[],
  seekTo?: number,
): void {
  const track = findTrackById(id);
  if (!track) {
    setPlayError(t("error.couldNotPlay"));
    return;
  }
  clearRetry();

  if (contextPaths && contextPaths.length > 0) {
    clearUserQueue();
    const startIdx = contextPaths.indexOf(track.path);
    setContextQueue(contextPaths, startIdx >= 0 ? startIdx : 0, track.path);
  } else {
    if (getUserQueuePaths().includes(track.path)) {
      removeFromUserQueue(track.path);
    }
    advanceContextIndex(track.id);
    if (getCurrentShuffle()) recordShuffleTrack(track.path);
  }

  setPlayError(null);
  playbackPhase.pendingStreamError = null;
  resetCrossfade();
  retryState.failures = 0;
  cancelSeek(false);
  const gen = ++ps.playGeneration;
  setNowPlaying({ ...nowPlayingFromTrack(track, seekTo ?? 0), playing: true });
  loadAndPlayNative(track.path, gen, seekTo);
}

async function resumeOrPlay(
  path: string,
  seekTo?: number,
  generation = ps.playGeneration,
): Promise<void> {
  if (generation === ps.playGeneration) {
    clearRetry();
    playbackPhase.pendingStreamError = null;
  }
  const token = ps.resumeToken;
  beginPlayReconcile(generation);
  try {
    let resumed = false;
    try {
      resumed = await vynl.playerResume(generation);
    } catch {}
    if (generation !== ps.playGeneration || token !== ps.resumeToken) return;
    if (!resumed) {
      const duration = getCurrentTrack()?.duration ?? 0;
      const atEnd =
        duration > 0 &&
        seekTo !== undefined &&
        seekTo >= Math.max(0, duration - 0.5);
      await playNative(path, atEnd ? undefined : seekTo, generation);
    }
    if (generation !== ps.playGeneration || token !== ps.resumeToken) return;
    updatePlaying(true);
    updateMediaSession();
  } finally {
    finishPlayReconcile(generation);
  }
}

function pausePlayback(): void {
  const generation = ps.playGeneration;
  ps.resumeToken += 1;
  const token = ps.resumeToken;
  ps.pauseIntent += 1;
  const finish = (): void => {
    ps.pauseIntent -= 1;
    if (generation !== ps.playGeneration) return;
    if (token !== ps.resumeToken) return;
    updatePlaying(false);
    updateMediaSession();
  };
  void vynl
    .playerPause(generation)
    .then(finish)
    .catch((error) => {
      console.warn("[vynl] pause failed:", error);
      finish();
    });
}

export function toggle(): void {
  if (reconcileIsStale()) {
    playbackPhase.pendingPlayGeneration = null;
    playbackPhase.playReconcileStartedAt = 0;
  }
  const np = getCurrentTrack();
  if (!np) return;
  const effectivelyPlaying =
    np.playing !== ps.pauseIntent > 0 || ps.resumePending > 0;
  if (effectivelyPlaying) {
    pausePlayback();
  } else {
    const seekTo = getCurrentTime() || 0;
    if (np.path) {
      ps.resumeToken += 1;
      ps.resumePending += 1;
      const resumeGeneration = ps.playGeneration;
      void resumeOrPlay(
        np.path,
        seekTo > 0 ? seekTo : undefined,
        resumeGeneration,
      )
        .catch((error) => {
          if (resumeGeneration !== ps.playGeneration) return;
          handlePlayReject(error);
        })
        .finally(() => {
          ps.resumePending -= 1;
        });
    }
  }
}

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
  loadAndPlayNative(nextTrack.path, gen);
}

function maybeStartCrossfade(position: number): boolean {
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
      }
      if (nextVideoId) prebufferState.delete(nextVideoId);
      playbackPhase.crossfadeTried = true;
    });

  return true;
}

export function next(): void {
  advanceNext(false);
}

function nextAutomatic(): void {
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
  setNowPlaying(nowPlayingFromTrack(prevTrack));
  loadAndPlayNative(prevTrack.path, gen);
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

initSeek({ getPlayGeneration: () => ps.playGeneration, syncMediaNow });
initMedia({ toggle, next, prev, pausePlayback, seekToTime, seekOffset, setVolumeAt, getCurrentVolume: getCurrentVolume, toggleMute });
initRetry({ getPauseIntent: () => ps.pauseIntent, next, updateMediaSession });

if (typeof window !== "undefined" && !IS_MINI) {
  const disposeMediaEffect = $effect.root(() => {
    $effect(() => {
      const np = getCurrentTrack();
      const playing = getIsTrackPlaying();
      syncMedia(np, playing, untrack(() => getCurrentTime()));
    });
  });
  const mediaTimer = window.setInterval(() => {
    const np = getCurrentTrack();
    if (!np) return;
    if (Date.now() - getLastMediaSentAt() < 1_000) return;
    syncMedia(np, getIsTrackPlaying(), getCurrentTime());
  }, 1_000);
  const unlistenMediaControl = vynl.onMediaControl(handleMediaControl, () => {
    void vynl.mediaControlsReady().catch(() => {});
  });
  import.meta.hot?.dispose(() => {
    disposeMediaEffect();
    window.clearInterval(mediaTimer);
    unlistenMediaControl();
  });
}

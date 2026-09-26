import { getLibrary } from "@state/library.svelte";
import {
  checkpointNowPlaying,
  getCurrentTrack,
  getCurrentTime,
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
  isSeekDragging,
  setSeekDragging,
  setDragTime,
} from "@state/now-playing.svelte";
import {
  getCurrentVolume,
  getCurrentMuted,
  getCurrentLoop,
  getCurrentShuffle,
  setPlayError,
  setVolume,
  onMuteChange,
} from "@state/player.svelte";
import { vynl } from "@lib/vynl";
import { clamp01 } from "@lib/pointer";
import { t } from "@lib/i18n";
import type { PluginSharedState } from "@lib/plugins/types";

export function isSharedPlayback(): boolean {
  return (getCurrentTrack()?.id ?? "").startsWith("shared:");
}

function normalizeDuration(value: number | undefined): number {
  return typeof value === "number" && Number.isFinite(value) && value > 0
    ? value
    : 0;
}

let playGeneration = Date.now();
let pendingRetry: ReturnType<typeof setTimeout> | null = null;
let syncRaf = 0;
let syncFrame = 0;
let positionRequestPending = false;
let finishedCheckPending = false;
let seekPending = false;
let seekRequestId = 0;
let pendingPlayGeneration: number | null = null;
let playReconcileUntil = 0;
let lastPositionCheckpoint = Date.now();
let lastSyncTickAt = Date.now();
let reconcilePending = false;
let syncHeartbeatTimer: ReturnType<typeof setInterval> | null = null;

const SYNC_HEARTBEAT_MS = 1_000;
const STALE_SYNC_MS = 2_000;

onMuteChange(applyVolume);

setSeekHandler(seekToTime);

if ("mediaSession" in navigator) {
  navigator.mediaSession.setActionHandler("play", () => {
    if (!getCurrentTrack()?.playing) toggle();
  });
  navigator.mediaSession.setActionHandler("pause", () => {
    if (getCurrentTrack()?.playing) toggle();
  });
  navigator.mediaSession.setActionHandler("nexttrack", () => next());
  navigator.mediaSession.setActionHandler("previoustrack", () => prev());
}

function beginPlayReconcile(generation: number): void {
  pendingPlayGeneration = generation;
  playReconcileUntil = Date.now() + 5_000;
  startSync();
}

function finishPlayReconcile(generation: number): void {
  if (pendingPlayGeneration === generation) {
    pendingPlayGeneration = null;
  }
}

function isHidden(): boolean {
  return (
    typeof document !== "undefined" && document.visibilityState === "hidden"
  );
}

function stopSync(): void {
  if (syncRaf !== 0) {
    cancelAnimationFrame(syncRaf);
    syncRaf = 0;
  }
}

function syncStep(): boolean {
  const np = getCurrentTrack();
  if (!np) {
    stopSync();
    return false;
  }

  const generation = playGeneration;
  const trackId = np.id;
  const isCurrent = (): boolean =>
    generation === playGeneration && getCurrentTrack()?.id === trackId;
  const checkEveryTick = isHidden();

  if (np.playing) {
    if (!isSeekDragging() && !seekPending && !positionRequestPending) {
      positionRequestPending = true;
      void vynl
        .playerGetPosition(generation)
        .then((pos) => {
          if (Number.isFinite(pos) && !isSeekDragging() && !seekPending && isCurrent()) {
            updateTime(pos);
          }
        })
        .catch(() => {})
        .finally(() => {
          positionRequestPending = false;
        });
    }
    if (
      !isSeekDragging() &&
      !seekPending &&
      (checkEveryTick || syncFrame % 10 === 0) &&
      !finishedCheckPending
    ) {
      finishedCheckPending = true;
      void vynl
        .playerCheckFinished(generation)
        .then((finished) => {
          if (finished && isCurrent() && getCurrentTrack()?.playing) {
            updatePlaying(false);
            if (!isSharedPlayback()) nextAutomatic();
          }
        })
        .catch(() => {})
        .finally(() => {
          finishedCheckPending = false;
        });
    }
    if (
      isCurrent() &&
      !isSeekDragging() &&
      !seekPending &&
      Date.now() - lastPositionCheckpoint >= 5_000
    ) {
      lastPositionCheckpoint = Date.now();
      checkpointNowPlaying();
    }
  } else {
    if (
      pendingPlayGeneration !== generation ||
      Date.now() > playReconcileUntil
    ) {
      finishPlayReconcile(generation);
      stopSync();
      return false;
    }
    if (syncFrame % 5 === 0) {
      void vynl
        .playerIsPlaying(generation)
        .then((playing) => {
          if (playing && isCurrent() && !getCurrentTrack()?.playing) {
            finishPlayReconcile(generation);
            updatePlaying(true);
            updateMediaSession();
          }
        })
        .catch(() => {});
    }
  }

  syncFrame += 1;
  return true;
}

function syncTick(): void {
  syncRaf = 0;
  lastSyncTickAt = Date.now();
  if (syncStep()) {
    startSync();
  }
}

function startSync(): void {
  if (syncRaf === 0) {
    syncRaf = requestAnimationFrame(syncTick);
  }
}

function syncHeartbeat(): void {
  const np = getCurrentTrack();
  if (!np) return;
  if (!np.playing && syncRaf === 0 && pendingPlayGeneration === null) return;

  const stale = Date.now() - lastSyncTickAt > STALE_SYNC_MS;
  if (syncRaf !== 0 && !stale) return;

  if (syncRaf !== 0) stopSync();
  if (syncStep()) startSync();
}

async function reconcilePlaybackState(): Promise<void> {
  const np = getCurrentTrack();
  if (!np || reconcilePending || isSeekDragging() || seekPending) return;

  reconcilePending = true;
  const generation = playGeneration;
  const trackId = np.id;
  const isCurrent = (): boolean =>
    generation === playGeneration && getCurrentTrack()?.id === trackId;

  try {
    let finished: boolean | null = null;
    if (!finishedCheckPending) {
      finishedCheckPending = true;
      try {
        finished = await vynl.playerCheckFinished(generation);
      } catch {
        finished = null;
      } finally {
        finishedCheckPending = false;
      }
    }
    if (finished === true) {
      if (isCurrent() && getCurrentTrack()?.playing) {
        updatePlaying(false);
        if (!isSharedPlayback()) nextAutomatic();
      }
      return;
    }
    if (!isCurrent()) return;

    let playing: boolean | null = null;
    try {
      playing = await vynl.playerIsPlaying(generation);
    } catch {
      playing = null;
    }
    if (playing === null || !isCurrent()) return;

    const cur = getCurrentTrack();
    if (!cur) return;
    if (playing !== cur.playing) updatePlaying(playing);
    if (getCurrentTrack()?.playing) {
      updateMediaSession();
      startSync();
    }
  } finally {
    reconcilePending = false;
  }
}

function onWindowActive(): void {
  startSync();
  void reconcilePlaybackState();
}

if (typeof document !== "undefined") {
  document.addEventListener("visibilitychange", () => {
    if (document.visibilityState === "visible") onWindowActive();
  });
  window.addEventListener("focus", onWindowActive);
  if (syncHeartbeatTimer === null) {
    syncHeartbeatTimer = window.setInterval(syncHeartbeat, SYNC_HEARTBEAT_MS);
  }
}

export function applyVolume(): void {
  const raw = getCurrentMuted() ? 0 : getCurrentVolume();
  void vynl.playerSetVolume(Math.round(raw * 100));
}

export function setVolumeAt(ratio: number): void {
  setVolume(clamp01(ratio));
  applyVolume();
}

function updateMediaSession(): void {
  if (!("mediaSession" in navigator)) return;
  const np = getCurrentTrack();
  if (!np) return;
  const meta: MediaMetadataInit = {
    title: np.title,
    artist: np.artist,
    album: np.album || undefined,
  };
  if (np.cover) {
    meta.artwork = [{ src: np.cover }];
  }
  navigator.mediaSession.metadata = new MediaMetadata(meta);
  navigator.mediaSession.playbackState = "playing";
}

function isPlayInterrupt(err: unknown): boolean {
  return (
    err instanceof DOMException &&
    (err.name === "AbortError" || err.name === "NotAllowedError")
  );
}

function clearRetry(): void {
  if (pendingRetry !== null) {
    clearTimeout(pendingRetry);
    pendingRetry = null;
  }
}

function scheduleRetry(failedId: string | null): void {
  clearRetry();
  pendingRetry = window.setTimeout(() => {
    pendingRetry = null;
    if ((getCurrentTrack()?.id ?? null) !== failedId) return;
    setPlayError(null);
    if (!isSharedPlayback()) next();
  }, 1600);
}

function handlePlayReject(err: unknown): void {
  if (isPlayInterrupt(err)) return;
  console.error("[vynl] native play failed:", err);
  updatePlaying(false);
  setPlayError(t("error.couldNotPlay"));
  const failedId = getCurrentTrack()?.id ?? null;
  scheduleRetry(failedId);
}

function isSeekUnsupported(error: unknown): boolean {
  const message = String(error).toLowerCase();
  return message.includes("seek:") && message.includes("support");
}

async function playNative(
  path: string,
  seekTo: number | undefined,
  generation: number,
): Promise<void> {
  if (seekTo !== undefined && (!Number.isFinite(seekTo) || seekTo < 0)) {
    throw new Error("invalid seek time");
  }
  const normalizedSeek = seekTo !== undefined && seekTo > 0 ? seekTo : undefined;
  try {
    await vynl.playerPlay(path, normalizedSeek, generation);
  } catch (error) {
    if (
      normalizedSeek === undefined ||
      generation !== playGeneration ||
      !isSeekUnsupported(error)
    ) {
      throw error;
    }
    await vynl.playerPlay(path, undefined, generation);
  }
}

function loadAndPlayNative(
  path: string,
  gen: number,
  seekTo?: number,
): void {
  beginPlayReconcile(gen);
  void playNative(path, seekTo, gen).then(() => {
    finishPlayReconcile(gen);
    if (gen !== playGeneration) return;
    updatePlaying(true);
    updateMediaSession();
    startSync();
  }).catch((err) => {
    finishPlayReconcile(gen);
    if (gen !== playGeneration) return;
    handlePlayReject(err);
  });
}

export function playTrack(
  id: string,
  contextPaths?: string[],
  seekTo?: number,
): void {
  invalidateSharedPlayback();
  const library = getLibrary();
  const track = library.find((x) => x.id === id);
  if (!track) return;
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
  cancelSeek(false);
  const gen = ++playGeneration;
  setNowPlaying(nowPlayingFromTrack(track, seekTo ?? 0));
  loadAndPlayNative(track.path, gen, seekTo);
}

function playSharedFile(opts: {
  path: string;
  title: string;
  artist: string;
  album?: string;
  duration?: number;
  cover?: string | null;
  lyrics?: string | null;
  seekTo?: number;
  playing?: boolean;
}): void {
  clearRetry();
  setPlayError(null);
  const duration = normalizeDuration(opts.duration);
  const initialTime = normalizeDuration(opts.seekTo);
  cancelSeek(false);
  const gen = ++playGeneration;
  setNowPlaying({
    id: `shared:${opts.path}`,
    title: opts.title,
    artist: opts.artist,
    album: opts.album ?? "",
    year: null,
    duration,
    path: opts.path,
    trackNumber: null,
    time: initialTime,
    playing: false,
    lyrics: opts.lyrics ?? null,
    cover: opts.cover ?? null,
  });
  if (opts.playing === false) {
    beginPlayReconcile(gen);
    void playNative(
      opts.path,
      initialTime > 0 ? initialTime : undefined,
      gen,
    ).then(() => {
      if (gen !== playGeneration) return;
      return vynl.playerPause(gen);
    }).then(() => {
      if (gen !== playGeneration) return;
      finishPlayReconcile(gen);
      return vynl.playerGetPosition(gen);
    }).then((position) => {
      if (gen !== playGeneration || position === undefined) return;
      updatePlaying(false);
      updateTime(Number.isFinite(position) ? position : initialTime, true);
    }).catch((err) => {
      finishPlayReconcile(gen);
      if (gen !== playGeneration) return;
      handlePlayReject(err);
    });
    return;
  }
  loadAndPlayNative(
    opts.path,
    gen,
    initialTime > 0 ? initialTime : undefined,
  );
}


let _sharedKey: string | null = null;
let _sharedGen = 0;
let _lastSharedSeekAt = 0;

function invalidateSharedPlayback(): void {
  _sharedGen += 1;
  _sharedKey = null;
  _lastSharedSeekAt = 0;
}

export async function playShared(s: PluginSharedState): Promise<void> {
  const gen = ++_sharedGen;
  const np = getCurrentTrack();
  const following = (np?.id ?? "").startsWith("shared:");
  const key = s.cacheKey ?? s.sourceUrl ?? null;

  if (!following || (key !== null && key !== _sharedKey)) {
    let path: string | null = null;
    if (s.sourceUrl) {
      try {
        path = await vynl.cacheRemoteAudio(s.sourceUrl, key ?? s.sourceUrl);
      } catch (e) {
        console.warn("[player] shared audio download failed:", e);
        return;
      }
      if (gen !== _sharedGen) return;
    } else if (following && np?.path) {
      path = np.path; 
    }
    if (!path) return;
    _sharedKey = key ?? _sharedKey;
    playSharedFile({
      path,
      title: s.title,
      artist: s.artist,
      album: s.album,
      duration: s.duration,
      cover: s.coverUrl ?? null,
      lyrics: s.lyrics ?? null,
      seekTo: s.time,
      playing: s.playing,
    });
    return;
  }

  const cur = np;
  if (!cur) return;
  const patch: Parameters<typeof patchNowPlaying>[0] = {};
  if (cur.title !== s.title) patch.title = s.title;
  if (cur.artist !== s.artist) patch.artist = s.artist;
  if (s.album !== undefined && cur.album !== s.album) patch.album = s.album;
  let duration = cur.duration;
  if (s.duration !== undefined) {
    duration = normalizeDuration(s.duration);
    if (cur.duration !== duration) patch.duration = duration;
  }
  if ((cur.cover ?? null) !== (s.coverUrl ?? null)) {
    patch.cover = s.coverUrl ?? null;
  }
  if ((cur.lyrics ?? null) !== (s.lyrics ?? null)) {
    patch.lyrics = s.lyrics ?? null;
  }
  if (Object.keys(patch).length > 0) patchNowPlaying(patch);

  const sharedTime = Number.isFinite(s.time)
    ? Math.max(0, s.time)
    : getCurrentTime();
  const boundedTime = duration > 0 ? Math.min(sharedTime, duration) : sharedTime;
  const drift = Math.abs(getCurrentTime() - boundedTime);
  if (drift > 0.2 && Date.now() - _lastSharedSeekAt > 100) {
    _lastSharedSeekAt = Date.now();
    seekToTime(boundedTime);
  } else {
    updateTime(boundedTime);
  }
  if (s.playing !== cur.playing) {
    if (s.playing && cur.path) {
      void resumeOrPlay(cur.path, boundedTime, playGeneration, gen).catch(handlePlayReject);
    } else {
      pausePlayback(gen);
    }
  }
}

export function stopShared(): void {
  const wasShared = isSharedPlayback();
  invalidateSharedPlayback();
  if (!wasShared) return;
  cancelSeek(false);
  const stopGeneration = ++playGeneration;
  void vynl.playerStop(stopGeneration).catch(() => {});
  updatePlaying(false);
}

export async function resumeOrPlay(
  path: string,
  seekTo?: number,
  generation = playGeneration,
  operationId?: number,
): Promise<void> {
  beginPlayReconcile(generation);
  try {
    let resumed = false;
    try {
      resumed = await vynl.playerResume(generation);
    } catch {}
    if (generation !== playGeneration) return;
    if (operationId !== undefined && operationId !== _sharedGen) return;
    if (!resumed) {
      const duration = getCurrentTrack()?.duration ?? 0;
      const atEnd =
        duration > 0 &&
        seekTo !== undefined &&
        seekTo >= Math.max(0, duration - 0.5);
      await playNative(path, atEnd ? undefined : seekTo, generation);
    }
    if (generation !== playGeneration) return;
    if (operationId !== undefined && operationId !== _sharedGen) return;
    updatePlaying(true);
    updateMediaSession();
    startSync();
  } finally {
    finishPlayReconcile(generation);
  }
}

export function pausePlayback(operationId?: number): void {
  const generation = playGeneration;
  const finish = (): void => {
    if (generation !== playGeneration) return;
    if (operationId !== undefined && operationId !== _sharedGen) return;
    updatePlaying(false);
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
  const np = getCurrentTrack();
  if (!np) return;
  if (np.playing) {
    pausePlayback();
  } else {
    const seekTo = getCurrentTime() || 0;
    if (np.path) {
      void resumeOrPlay(
        np.path,
        seekTo > 0 ? seekTo : undefined,
        playGeneration,
      ).catch(handlePlayReject);
    }
  }
}

function advanceNext(automatic: boolean): void {
  const np = getCurrentTrack();
  const selectedLoop = getCurrentLoop();
  const loop = automatic || selectedLoop !== "one" ? selectedLoop : "off";
  const shuffle = getCurrentShuffle();
  const nextId = pickNextId(
    getUserQueuePaths(),
    getContextQueuePaths(),
    getContextIndex(),
    np?.id ?? null,
    shuffle,
    loop,
  );
  if (!nextId) return;
  invalidateSharedPlayback();

  const library = getLibrary();
  const nextTrack = library.find((t) => t.id === nextId);
  if (nextTrack) {
    const uq = getUserQueuePaths();
    const uqIdx = uq.indexOf(nextTrack.path);
    if (uqIdx >= 0) {
      removeFromUserQueue(nextTrack.path);
    }
    advanceContextIndex(nextId);
    if (shuffle) recordShuffleTrack(nextTrack.path);
  }

  cancelSeek(false);
  const gen = ++playGeneration;
  setNowPlaying(
    nextTrack
      ? nowPlayingFromTrack(nextTrack)
      : { ...np!, id: nextId, time: 0, playing: false, duration: 0, lyrics: null, cover: null },
  );
  if (nextTrack) {
    loadAndPlayNative(nextTrack.path, gen);
  }
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
  invalidateSharedPlayback();

  const library = getLibrary();
  const prevTrack = library.find((t) => t.id === prevId);
  if (prevTrack) {
    advanceContextIndex(prevId);
  }

  cancelSeek(false);
  const gen = ++playGeneration;
  setNowPlaying(
    prevTrack
      ? nowPlayingFromTrack(prevTrack)
      : { ...np!, id: prevId, time: 0, playing: false, duration: 0, lyrics: null, cover: null },
  );
  if (prevTrack) {
    loadAndPlayNative(prevTrack.path, gen);
  }
}

let _seekTarget: number | null = null;

function clampSeekTarget(time: number, total?: number): number | null {
  if (!Number.isFinite(time)) return null;
  const duration = total ?? getCurrentTrack()?.duration ?? 0;
  const lower = Math.max(0, time);
  return Number.isFinite(duration) && duration > 0
    ? Math.min(lower, duration)
    : lower;
}

function requestSeek(time: number): void {
  const target = clampSeekTarget(time);
  if (target === null) return;

  _seekTarget = target;
  setDragTime(target);
  seekPending = true;
  const requestId = ++seekRequestId;
  const generation = playGeneration;

  void vynl
    .playerSeek(target, generation)
    .then(() => {
      if (requestId !== seekRequestId || generation !== playGeneration) return;
      seekPending = false;
      _seekTarget = null;
      updateTime(target, true);
    })
    .catch((error) => {
      if (requestId !== seekRequestId || generation !== playGeneration) return;
      seekPending = false;
      _seekTarget = null;
      console.warn("[vynl] seek failed:", error);
      void vynl
        .playerGetPosition(generation)
        .then((position) => {
          if (
            Number.isFinite(position) &&
            requestId === seekRequestId &&
            generation === playGeneration
          ) {
            updateTime(position);
          }
        })
        .catch(() => {});
    });
}

export function seekRatio(ratio: number, total?: number): void {
  if (!getCurrentTrack() || !Number.isFinite(ratio)) return;
  const duration = normalizeDuration(total);
  const target = clampSeekTarget(ratio * duration, duration);
  if (target === null) return;
  seekRequestId += 1;
  seekPending = false;
  setDragTime(target);
  _seekTarget = target;
}

export function seekFlush(): void {
  if (_seekTarget !== null) requestSeek(_seekTarget);
}

export function seekToTime(time: number): void {
  requestSeek(time);
}

export function seekOffset(delta: number): void {
  if (!Number.isFinite(delta)) return;
  const base = _seekTarget ?? getCurrentTime();
  requestSeek(base + delta);
}

export function cancelSeek(restorePosition = true): void {
  seekRequestId += 1;
  seekPending = false;
  _seekTarget = null;
  setSeekDragging(false);
  if (restorePosition) {
    const generation = playGeneration;
    void vynl
      .playerGetPosition(generation)
      .then((position) => {
        if (generation === playGeneration && Number.isFinite(position)) {
          updateTime(position);
        }
      })
      .catch(() => {});
  }
}

export function clearPlayError(): void {
  setPlayError(null);
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

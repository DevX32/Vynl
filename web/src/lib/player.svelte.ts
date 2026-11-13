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
  peekPrevId,
  isSeekDragging,
  setSeekDragging,
  setDragTime,
} from "@state/now-playing.svelte";
import {
  getCrossfade,
  getCurrentVolume,
  getCurrentMuted,
  getCurrentLoop,
  getCurrentShuffle,
  setPlayError,
  setVolume,
  onMuteChange,
} from "@state/player.svelte";
import type { LoopMode } from "@state/player.svelte";
import { vynl } from "@lib/vynl";
import { clamp01 } from "@lib/pointer";
import { friendlyError } from "@lib/errors";
import { t } from "@lib/i18n";
import type {
  CatalogueStreamInfo,
  LibraryTrack,
  MediaControlEvent,
  MediaUpdate,
  PlaybackTick,
} from "@lib/types";
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
let seekPending = false;
let seekRequestId = 0;
let pendingPlayGeneration: number | null = null;
let lastPositionCheckpoint = Date.now();
let reconcilePending = false;

let pauseIntent = 0;

const CATALOGUE_PREFIX = "ytm:";

function catalogueIdOf(path: string | null | undefined): string | null {
  if (!path || !path.startsWith(CATALOGUE_PREFIX)) return null;
  const id = path.slice(CATALOGUE_PREFIX.length);
  return id.length > 0 ? id : null;
}

interface PrebufferEntry {
  status: "pending" | "ready" | "failed";
  at: number;
}

interface PrebufferTarget {
  videoId: string;
  duration?: number;
}

const prebufferState = new Map<string, PrebufferEntry>();
const PREBUFFER_REFRESH_MS = 45_000;
const PREBUFFER_CHECK_MS = 15_000;
let lastPrebufferCheck = 0;
let prebufferCandidates: string[] = [];

function requestPrebuffer(target: PrebufferTarget, force: boolean): void {
  const now = Date.now();
  const entry = prebufferState.get(target.videoId);
  if (!force && entry && now - entry.at < PREBUFFER_REFRESH_MS) return;
  if (entry?.status === "pending") return;

  prebufferState.set(target.videoId, { status: "pending", at: now });
  void vynl
    .cataloguePrebuffer(target.videoId, target.duration)
    .then(() => {
      if (!prebufferCandidates.includes(target.videoId)) return;
      prebufferState.set(target.videoId, { status: "ready", at: Date.now() });
    })
    .catch((error) => {
      if (!prebufferCandidates.includes(target.videoId)) return;
      prebufferState.set(target.videoId, { status: "failed", at: Date.now() });
      console.warn("[vynl] prebuffer failed:", error);
    });
}

function schedulePrebuffers(force = false): void {
  if (typeof window === "undefined") return;
  const currentId = getCurrentTrack()?.id ?? null;

  const nextTrack = findTrackById(
    peekNextId(
      getUserQueuePaths(),
      getContextQueuePaths(),
      getContextIndex(),
      currentId,
      getCurrentShuffle(),
      getCurrentLoop(),
    ),
  );
  const prevTrack = findTrackById(
    peekPrevId(
      currentId,
      getContextQueuePaths(),
      getContextIndex(),
      getCurrentShuffle(),
      getCurrentLoop(),
    ),
  );

  const targets: PrebufferTarget[] = [];
  for (const track of [nextTrack, prevTrack]) {
    const videoId = catalogueIdOf(track?.path);
    if (!videoId) continue;
    if (targets.some((target) => target.videoId === videoId)) continue;
    targets.push({ videoId, duration: track?.duration });
  }

  for (const stale of prebufferCandidates) {
    if (targets.some((target) => target.videoId === stale)) continue;
    prebufferState.delete(stale);
    void vynl.catalogueDiscardPrebuffer(stale).catch(() => {});
  }
  prebufferCandidates = targets.map((target) => target.videoId);

  for (const target of targets) requestPrebuffer(target, force);
}

onMuteChange(applyVolume);

setSeekHandler(seekToTime);

if ("mediaSession" in navigator) {
  navigator.mediaSession.setActionHandler("play", handleMediaPlay);
  navigator.mediaSession.setActionHandler("pause", handleMediaPause);
  navigator.mediaSession.setActionHandler("nexttrack", next);
  navigator.mediaSession.setActionHandler("previoustrack", prev);
}

function beginPlayReconcile(generation: number): void {
  pendingPlayGeneration = generation;
}

function finishPlayReconcile(generation: number): void {
  if (pendingPlayGeneration === generation) {
    pendingPlayGeneration = null;
  }
}

const CROSSFADE_TAIL = 0.75;
let crossfadeTried = false;

function resetCrossfade(): void {
  crossfadeTried = false;
}

function handleTick(tick: PlaybackTick): void {
  if (tick.generation !== playGeneration) return;
  const np = getCurrentTrack();
  if (!np) return;

  if (tick.finished) {
    if (np.playing) {
      updatePlaying(false);
      updateMediaSession();
      if (!isSharedPlayback()) nextAutomatic();
    }
    return;
  }

  if (tick.playing && maybeStartCrossfade(tick.position)) {
    return;
  }

  if (tick.playing && pauseIntent > 0) return;

  if (tick.playing) {
    if (pendingPlayGeneration === playGeneration) {
      finishPlayReconcile(playGeneration);
    }
    if (!np.playing) {
      updatePlaying(true);
      updateMediaSession();
    }
  }

  if (isSeekDragging() || seekPending) return;

  if (Number.isFinite(tick.position)) {
    updateTime(tick.position);
  }

  if (
    tick.duration != null &&
    Number.isFinite(tick.duration) &&
    tick.duration > 0 &&
    !(np.duration > 0)
  ) {
    patchNowPlaying({ duration: tick.duration });
  }

  if (tick.playing && Date.now() - lastPositionCheckpoint >= 5_000) {
    lastPositionCheckpoint = Date.now();
    checkpointNowPlaying();
  }

  if (
    tick.playing &&
    !seekPending &&
    !isSeekDragging() &&
    Date.now() - lastPrebufferCheck >= PREBUFFER_CHECK_MS
  ) {
    lastPrebufferCheck = Date.now();
    schedulePrebuffers();
  }
}

vynl.onPlaybackTick(handleTick);

async function reconcilePlaybackState(): Promise<void> {
  const np = getCurrentTrack();
  if (!np || reconcilePending || isSeekDragging() || seekPending) return;

  reconcilePending = true;
  const generation = playGeneration;
  const trackId = np.id;
  const isCurrent = (): boolean =>
    generation === playGeneration && getCurrentTrack()?.id === trackId;

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
    reconcilePending = false;
  }
}

function onWindowActive(): void {
  void reconcilePlaybackState();
}

if (typeof document !== "undefined") {
  document.addEventListener("visibilitychange", () => {
    if (document.visibilityState === "visible") onWindowActive();
  });
  window.addEventListener("focus", onWindowActive);
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
  navigator.mediaSession.playbackState = getIsTrackPlaying()
    ? "playing"
    : "paused";
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
    setPlayError(null);
    if ((getCurrentTrack()?.id ?? null) !== failedId) return;
    if (!isSharedPlayback()) next();
  }, 1600);
}

function handlePlayReject(err: unknown): void {
  if (isPlayInterrupt(err)) return;
  console.error("[vynl] native play failed:", err);
  updatePlaying(false);
  updateMediaSession();
  setPlayError(friendlyError(err, t("error.couldNotPlay")));
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
): Promise<CatalogueStreamInfo | null> {
  if (seekTo !== undefined && (!Number.isFinite(seekTo) || seekTo < 0)) {
    throw new Error("invalid seek time");
  }
  const normalizedSeek = seekTo !== undefined && seekTo > 0 ? seekTo : undefined;
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
    if (
      normalizedSeek === undefined ||
      generation !== playGeneration ||
      !isSeekUnsupported(error)
    ) {
      throw error;
    }
    await vynl.playerPlay(path, undefined, generation);
  }
  return null;
}

function loadAndPlayNative(
  path: string,
  gen: number,
  seekTo?: number,
): void {
  beginPlayReconcile(gen);
  void playNative(path, seekTo, gen).then((info) => {
    finishPlayReconcile(gen);
    if (gen !== playGeneration) return;
    if (info?.duration && info.duration > 0) {
      const np = getCurrentTrack();
      if (np && !(np.duration > 0)) {
        patchNowPlaying({ duration: info.duration });
      }
    }
    updatePlaying(true);
    updateMediaSession();
    schedulePrebuffers();
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
  const track = findTrackById(id);
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
  resetCrossfade();
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
  resetCrossfade();
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
    pauseIntent += 1;
    void playNative(
      opts.path,
      initialTime > 0 ? initialTime : undefined,
      gen,
    ).then(() => {
      if (gen !== playGeneration) return;
      return vynl.playerPause(gen);
    }).finally(() => {
      pauseIntent -= 1;
    }).then(() => {
      if (gen !== playGeneration) return;
      finishPlayReconcile(gen);
      return vynl.playerGetPosition(gen);
    }).then((position) => {
      if (gen !== playGeneration || position === undefined) return;
      updatePlaying(false);
      updateMediaSession();
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
  updateMediaSession();
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
  } finally {
    finishPlayReconcile(generation);
  }
}

export function pausePlayback(operationId?: number): void {
  const generation = playGeneration;
  pauseIntent += 1;
  const finish = (): void => {
    pauseIntent -= 1;
    if (generation !== playGeneration) return;
    if (operationId !== undefined && operationId !== _sharedGen) return;
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
  invalidateSharedPlayback();
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
  resetCrossfade();
  cancelSeek(false);
  const gen = ++playGeneration;
  setNowPlaying(nowPlayingFromTrack(nextTrack));
  loadAndPlayNative(nextTrack.path, gen);
}

function maybeStartCrossfade(position: number): boolean {
  const fade = getCrossfade();
  if (fade <= 0 || crossfadeTried) return false;

  const np = getCurrentTrack();
  if (!np?.playing || !np.path || np.duration <= 0) return false;
  if (isSeekDragging() || seekPending) return false;
  if (isSharedPlayback() || getCurrentLoop() === "one") return false;

  const remaining = np.duration - position;
  if (remaining <= 0 || remaining > fade + CROSSFADE_TAIL) return false;

  const nextTrack = resolveNextTrack(true, true);
  if (!nextTrack || nextTrack.path === np.path) return false;
  if (nextTrack.duration > 0 && nextTrack.duration < fade) return false;

  const nextVideoId = catalogueIdOf(nextTrack.path);
  if (nextVideoId) {
    if (prebufferState.get(nextVideoId)?.status !== "ready") return false;
  }

  crossfadeTried = true;

  const generation = playGeneration + 1;
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
      if (generation !== playGeneration + 1) return;
      playGeneration = generation;
      if (nextVideoId) prebufferState.delete(nextVideoId);
      consumeNextTrack(nextTrack);
      setNowPlaying({
        ...nowPlayingFromTrack(nextTrack),
        playing: true,
      });
      updateMediaSession();
      resetCrossfade();
      schedulePrebuffers();
    })
    .catch((error) => {
      console.warn("[vynl] crossfade failed:", error);
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

  invalidateSharedPlayback();
  if (getUserQueuePaths().includes(prevTrack.path)) {
    removeFromUserQueue(prevTrack.path);
  }
  advanceContextIndex(prevId);

  clearRetry();
  setPlayError(null);
  resetCrossfade();
  cancelSeek(false);
  const gen = ++playGeneration;
  setNowPlaying(nowPlayingFromTrack(prevTrack));
  loadAndPlayNative(prevTrack.path, gen);
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
  const np = getCurrentTrack();
  const videoId = catalogueIdOf(np?.path);

  const seek = videoId
    ? vynl.catalogueSeekStream(
        videoId,
        target,
        np?.duration,
        generation,
        getIsTrackPlaying(),
        requestId,
      )
    : vynl.playerSeek(target, generation);

  void seek
    .then(() => {
      if (requestId !== seekRequestId || generation !== playGeneration) return;
      seekPending = false;
      _seekTarget = null;
      updateTime(target, true);
      syncMediaNow();
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
  if (duration <= 0) {
    cancelSeek();
    return;
  }
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

type CurrentMedia = ReturnType<typeof getCurrentTrack>;

let lastMediaSignature: string | null = null;
let lastMediaSentAt = 0;

function mediaSignature(np: CurrentMedia, playing: boolean): string {
  if (!np) return "";
  return [
    np.id,
    np.title,
    np.artist,
    np.album,
    np.duration,
    np.cover ?? "",
    playing ? "1" : "0",
  ].join("\u001f");
}

function sendMediaUpdate(
  np: CurrentMedia,
  playing: boolean,
  position: number,
): void {
  const payload: MediaUpdate | null = np
    ? {
        title: np.title,
        artist: np.artist,
        album: np.album,
        duration: np.duration,
        cover: np.cover,
        playing,
        position,
      }
    : null;
  void vynl.mediaUpdate(payload).catch((error) => {
    console.warn("[vynl] media update failed:", error);
  });
}

function syncMedia(
  np: CurrentMedia,
  playing: boolean,
  position: number,
  force = false,
): void {
  const signature = mediaSignature(np, playing);
  if (!force && signature === lastMediaSignature) return;
  lastMediaSignature = signature;
  lastMediaSentAt = Date.now();
  sendMediaUpdate(np, playing, position);
}

function syncMediaNow(): void {
  syncMedia(getCurrentTrack(), getIsTrackPlaying(), getCurrentTime(), true);
}

function handleMediaPlay(): void {
  if (!getIsTrackPlaying()) toggle();
}

function handleMediaPause(): void {
  if (getIsTrackPlaying()) toggle();
}

function handleMediaStop(): void {
  if (getIsTrackPlaying()) pausePlayback();
  seekToTime(0);
}

function handleMediaControl(event: MediaControlEvent): void {
  switch (event.action) {
    case "play":
      handleMediaPlay();
      break;
    case "pause":
      handleMediaPause();
      break;
    case "toggle":
      toggle();
      break;
    case "next":
      next();
      break;
    case "previous":
      prev();
      break;
    case "stop":
      handleMediaStop();
      break;
    case "seek":
      seekToTime(event.position);
      break;
    case "seekBy":
      seekOffset(event.offset);
      break;
    case "setVolume":
      setVolumeAt(clamp01(event.volume));
      break;
  }
}

if (typeof window !== "undefined") {
  $effect.root(() => {
    $effect(() => {
      const np = getCurrentTrack();
      const playing = getIsTrackPlaying();
      const position = getCurrentTime();
      const due = np !== null && Date.now() - lastMediaSentAt >= 2_000;
      syncMedia(np, playing, position, due);
    });
  });
  vynl.onMediaControl(handleMediaControl);
}

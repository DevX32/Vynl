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
import { prefetchLyrics } from "@state/lyrics.svelte";
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
import { convertFileSrc } from "@tauri-apps/api/core";
import { clamp01 } from "@lib/pointer";
import { friendlyError } from "@lib/errors";
import { evictOldest } from "@lib/cache";
import { catalogueIdFromPath as catalogueIdOf } from "@lib/catalogue";
import { t } from "@lib/i18n";
import type {
  CatalogueStreamInfo,
  LibraryTrack,
  MediaControlEvent,
  MediaUpdate,
  PlaybackTick,
  StreamErrorEvent,
} from "@lib/types";

const IS_MINI =
  typeof window !== "undefined" && window.location.hash.startsWith("#mini");

function normalizeDuration(value: number | undefined): number {
  return typeof value === "number" && Number.isFinite(value) && value > 0
    ? value
    : 0;
}

let playGeneration = Date.now();
let pendingRetry: number | null = null;
let consecutivePlayFailures = 0;
const MAX_CONSECUTIVE_PLAY_FAILURES = 3;
let seekPending = false;
let seekRequestId = 0;
let pendingPlayGeneration: number | null = null;
let lastPositionCheckpoint = Date.now();
let reconcilePending = false;

let pauseIntent = 0;
let resumeToken = 0;
let resumePending = 0;

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
const PREBUFFER_CHECK_MS = 5_000;
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
  if (getCurrentLoop() === "one") return;
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

  if (nextTrack) prefetchLyrics(nextTrack);

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

let playReconcileStartedAt = 0;
const RECONCILE_STALE_MS = 10_000;

function beginPlayReconcile(generation: number): void {
  pendingPlayGeneration = generation;
  playReconcileStartedAt = Date.now();
}

function finishPlayReconcile(generation: number): void {
  if (pendingPlayGeneration === generation) {
    pendingPlayGeneration = null;
    playReconcileStartedAt = 0;
  }
}

function reconcileIsStale(): boolean {
  return (
    pendingPlayGeneration !== null &&
    Date.now() - playReconcileStartedAt >= RECONCILE_STALE_MS
  );
}

const CROSSFADE_TAIL = 0.75;
let crossfadeTried = false;
let pendingStreamError: StreamErrorEvent | null = null;

function resetCrossfade(): void {
  crossfadeTried = false;
}

function handleStreamError(event: StreamErrorEvent): void {
  if (event.generation !== playGeneration) return;
  const np = getCurrentTrack();
  if (!np) return;
  if (catalogueIdOf(np.path) !== event.videoId) return;
  if (!np.playing) {
    pendingStreamError = event;
    return;
  }
  updatePlaying(false);
  updateMediaSession();
  consecutivePlayFailures += 1;
  setPlayError(friendlyError(event.message, t("error.couldNotPlay")));
  if (consecutivePlayFailures >= MAX_CONSECUTIVE_PLAY_FAILURES) {
    return;
  }
  scheduleRetry(np.id);
}

function handleTick(tick: PlaybackTick): void {
  if (tick.generation !== playGeneration) return;
  const np = getCurrentTrack();
  if (!np) return;

  if (tick.finished) {
    const streamError = pendingStreamError;
    pendingStreamError = null;
    if (np.playing) {
      updatePlaying(false);
      updateMediaSession();
      if (streamError && catalogueIdOf(np.path) === streamError.videoId) {
        consecutivePlayFailures += 1;
        setPlayError(
          friendlyError(streamError.message, t("error.couldNotPlay")),
        );
        if (consecutivePlayFailures >= MAX_CONSECUTIVE_PLAY_FAILURES) {
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
    if (consecutivePlayFailures > 0) {
      consecutivePlayFailures = 0;
    }

    if (pendingPlayGeneration === playGeneration) {
      finishPlayReconcile(playGeneration);
    }
    if (pauseIntent === 0 && !np.playing) {
      updatePlaying(true);
      updateMediaSession();
    }
  }

  if (isSeekDragging() || seekPending) return;

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

let _playerUnlisten: (() => void) | null = null;

if (typeof window !== "undefined" && !IS_MINI) {
  (_playerUnlisten as (() => void) | null)?.();
  const offTick = vynl.onPlaybackTick(handleTick);
  const offStreamError = vynl.onStreamError(handleStreamError);
  _playerUnlisten = () => {
    offTick();
    offStreamError();
  };
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
  const onVisibilityChange = () => {
    if (document.visibilityState === "visible") onWindowActive();
  };
  document.addEventListener("visibilitychange", onVisibilityChange);
  window.addEventListener("focus", onWindowActive);
  import.meta.hot?.dispose(() => {
    document.removeEventListener("visibilitychange", onVisibilityChange);
    window.removeEventListener("focus", onWindowActive);
    _playerUnlisten?.();
    _playerUnlisten = null;
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

function updateMediaSession(): void {
  if (!("mediaSession" in navigator) || IS_MINI) return;
  const np = getCurrentTrack();
  if (!np) {
    navigator.mediaSession.metadata = null;
    navigator.mediaSession.playbackState = "none";
    return;
  }
  const meta: MediaMetadataInit = {
    title: np.title,
    artist: np.artist,
    album: np.album || undefined,
  };
  if (np.cover) {
    meta.artwork = [
      {
        src: /^https?:\/\//i.test(np.cover)
          ? np.cover
          : convertFileSrc(np.cover),
      },
    ];
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
    if (pauseIntent > 0) return;
    if ((getCurrentTrack()?.id ?? null) !== failedId) return;
    if (consecutivePlayFailures >= MAX_CONSECUTIVE_PLAY_FAILURES) {
      updatePlaying(false);
      updateMediaSession();
      return;
    }
    setPlayError(null);
    next();
  }, 1600);
}

function handlePlayReject(err: unknown): void {
  if (isPlayInterrupt(err)) return;
  console.error("[vynl] native play failed:", err);
  updatePlaying(false);
  updateMediaSession();
  consecutivePlayFailures += 1;
  if (consecutivePlayFailures >= MAX_CONSECUTIVE_PLAY_FAILURES) {
    setPlayError(t("error.couldNotPlay"));
    return;
  }
  setPlayError(friendlyError(err, t("error.couldNotPlay")));
  const failedId = getCurrentTrack()?.id ?? null;
  scheduleRetry(failedId);
}

function isSeekUnsupported(error: unknown): boolean {
  const message = String(error).toLowerCase();
  return message.includes("seek:") && message.includes("support");
}

function extractVideoId(value: string): string | null {
  const raw = value.trim();
  if (!raw) return null;
  if (/^[a-zA-Z0-9_-]{11}$/.test(raw)) return raw;
  try {
    const url = new URL(raw);
    const host = url.hostname.replace(/^www\./, "").toLowerCase();
    if (host === "youtu.be") {
      const id = url.pathname.split("/").filter(Boolean)[0];
      return id && /^[a-zA-Z0-9_-]{11}$/.test(id) ? id : null;
    }
    if (host.endsWith("youtube.com")) {
      const v = url.searchParams.get("v");
      if (v && /^[a-zA-Z0-9_-]{11}$/.test(v)) return v;
      const parts = url.pathname.split("/").filter(Boolean);
      const idx = parts.findIndex((p) => p === "shorts" || p === "embed");
      const id = idx >= 0 ? parts[idx + 1] : null;
      return id && /^[a-zA-Z0-9_-]{11}$/.test(id) ? id : null;
    }
  } catch {}
  return null;
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
    if (generation !== playGeneration) throw error;
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
      if (gen !== playGeneration) return;
      if (info?.duration && info.duration > 0) {
        const np = getCurrentTrack();
        if (np && !(np.duration > 0)) {
          patchNowPlaying({ duration: info.duration });
        }
      }
      if (pendingStreamError) return;
      updatePlaying(true);
      updateMediaSession();
      schedulePrebuffers();
    })
    .catch((err) => {
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
  pendingStreamError = null;
  resetCrossfade();
  consecutivePlayFailures = 0;
  cancelSeek(false);
  const gen = ++playGeneration;
  setNowPlaying({ ...nowPlayingFromTrack(track, seekTo ?? 0), playing: true });
  loadAndPlayNative(track.path, gen, seekTo);
}

async function resumeOrPlay(
  path: string,
  seekTo?: number,
  generation = playGeneration,
): Promise<void> {
  if (generation === playGeneration) {
    clearRetry();
    pendingStreamError = null;
  }
  const token = resumeToken;
  beginPlayReconcile(generation);
  try {
    let resumed = false;
    try {
      resumed = await vynl.playerResume(generation);
    } catch {}
    if (generation !== playGeneration || token !== resumeToken) return;
    if (!resumed) {
      const duration = getCurrentTrack()?.duration ?? 0;
      const atEnd =
        duration > 0 &&
        seekTo !== undefined &&
        seekTo >= Math.max(0, duration - 0.5);
      await playNative(path, atEnd ? undefined : seekTo, generation);
    }
    if (generation !== playGeneration || token !== resumeToken) return;
    updatePlaying(true);
    updateMediaSession();
  } finally {
    finishPlayReconcile(generation);
  }
}

function pausePlayback(): void {
  const generation = playGeneration;
  resumeToken += 1;
  const token = resumeToken;
  pauseIntent += 1;
  const finish = (): void => {
    pauseIntent -= 1;
    if (generation !== playGeneration) return;
    if (token !== resumeToken) return;
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
    pendingPlayGeneration = null;
    playReconcileStartedAt = 0;
  }
  const np = getCurrentTrack();
  if (!np) return;
  const effectivelyPlaying =
    np.playing !== pauseIntent > 0 || resumePending > 0;
  if (effectivelyPlaying) {
    pausePlayback();
  } else {
    const seekTo = getCurrentTime() || 0;
    if (np.path) {
      resumeToken += 1;
      resumePending += 1;
      const resumeGeneration = playGeneration;
      void resumeOrPlay(
        np.path,
        seekTo > 0 ? seekTo : undefined,
        resumeGeneration,
      )
        .catch((error) => {
          if (resumeGeneration !== playGeneration) return;
          handlePlayReject(error);
        })
        .finally(() => {
          resumePending -= 1;
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
  pendingStreamError = null;
  resetCrossfade();
  cancelSeek(false);
  const gen = ++playGeneration;
  setNowPlaying({ ...nowPlayingFromTrack(nextTrack), playing: true });
  loadAndPlayNative(nextTrack.path, gen);
}

function maybeStartCrossfade(position: number): boolean {
  const fade = getCrossfade();
  if (fade <= 0 || crossfadeTried) return false;

  const np = getCurrentTrack();
  if (
    !np?.playing ||
    !np.path ||
    !Number.isFinite(np.duration) ||
    np.duration <= 0
  )
    return false;
  if (isSeekDragging() || seekPending) return false;
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

  crossfadeTried = true;

  const generation = ++playGeneration;
  const pauseToken = pauseIntent;
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
      if (generation !== playGeneration) return;
      consumeNextTrack(nextTrack);
      setNowPlaying({
        ...nowPlayingFromTrack(nextTrack),
        playing: pauseToken === pauseIntent,
      });
      updateMediaSession();
      resetCrossfade();
      schedulePrebuffers();
    })
    .catch((error) => {
      console.warn("[vynl] crossfade failed:", error);
      if (playGeneration === generation && generation > 0) {
        playGeneration = generation - 1;
      }
      if (nextVideoId) prebufferState.delete(nextVideoId);
      crossfadeTried = true;
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
  pendingStreamError = null;
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
  pendingStreamError = null;
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

let lastMediaSentAt = 0;

const mediaCoverPaths = new Map<string, string>();

export function clearMediaCoverCache(): void {
  mediaCoverPaths.clear();
}

async function resolveMediaCover(cover: string | null): Promise<string | null> {
  if (!cover) return null;
  if (!/^https?:\/\//i.test(cover)) return cover;
  const cached = mediaCoverPaths.get(cover);
  if (cached) return cached;
  try {
    const local = await vynl.cacheCover(cover);
    mediaCoverPaths.set(cover, local);
    evictOldest(mediaCoverPaths, 200);
    return local;
  } catch {
    return cover;
  }
}

let mediaUpdateChain: Promise<void> = Promise.resolve();
let lastMediaUpdateWarningAt = 0;

function warnMediaUpdateFailed(error: unknown): void {
  const now = Date.now();
  if (now - lastMediaUpdateWarningAt < 60_000) return;
  lastMediaUpdateWarningAt = now;
  console.warn("[vynl] media update failed:", error);
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
  mediaUpdateChain = mediaUpdateChain
    .then(async () => {
      if (payload) payload.cover = await resolveMediaCover(payload.cover);
      await vynl.mediaUpdate(payload);
    })
    .catch(warnMediaUpdateFailed);
}

function syncMedia(np: CurrentMedia, playing: boolean, position: number): void {
  lastMediaSentAt = Date.now();
  sendMediaUpdate(np, playing, position);
}

function syncMediaNow(): void {
  syncMedia(getCurrentTrack(), getIsTrackPlaying(), getCurrentTime());
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

let lastMediaAction: { action: string; at: number } | null = null;
function handleMediaControl(event: MediaControlEvent): void {
  const now = Date.now();
  if (
    lastMediaAction?.action === event.action &&
    now - lastMediaAction.at < 150
  ) {
    return;
  }
  lastMediaAction = { action: event.action, at: now };
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
    case "volumeUp":
      setVolumeAt(getCurrentVolume() + 0.05);
      break;
    case "volumeDown":
      setVolumeAt(getCurrentVolume() - 0.05);
      break;
    case "mute":
      toggleMute();
      break;
  }
}

if (typeof window !== "undefined" && !IS_MINI) {
  const disposeMediaEffect = $effect.root(() => {
    $effect(() => {
      const np = getCurrentTrack();
      const playing = getIsTrackPlaying();
      syncMedia(
        np,
        playing,
        untrack(() => getCurrentTime()),
      );
    });
  });
  const mediaTimer = window.setInterval(() => {
    const np = getCurrentTrack();
    if (!np) return;
    if (Date.now() - lastMediaSentAt < 1_000) return;
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

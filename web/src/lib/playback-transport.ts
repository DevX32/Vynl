import { getCurrentTrack, getCurrentTime, patchNowPlaying, setNowPlaying, updatePlaying, clearUserQueue, removeFromUserQueue, advanceContextIndex, getUserQueuePaths, setContextQueue, recordShuffleTrack, nowPlayingFromTrack, findTrackById } from "@state/now-playing.svelte";
import { getCurrentVolume, getCurrentMuted, getCurrentShuffle, getPlaybackSpeed, getReplayGain, setPlayError, setVolume } from "@state/player.svelte";
import { vynl } from "@lib/vynl";
import { clamp01 } from "@lib/pointer";
import { catalogueIdFromPath as catalogueIdOf } from "@lib/catalogue";
import { t } from "@lib/i18n";
import type { CatalogueStreamInfo } from "@lib/types";
import { isSeekUnsupported, extractVideoId } from "./playback-utils";
import { cancelSeek } from "./playback-seek";
import { updateMediaSession } from "./playback-media";
import { prebufferState, schedulePrebuffers } from "./playback-prebuffer";
import { playbackPhase, beginPlayReconcile, finishPlayReconcile, reconcileIsStale, resetCrossfade } from "./playback-reconcile";
import { retryState, clearRetry, handlePlayReject } from "./playback-retry";
import { ps } from "./playback-state";
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

export function loadAndPlayNative(path: string, gen: number, seekTo?: number): void {
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

export function pausePlayback(): void {
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


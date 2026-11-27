import { checkpointNowPlaying, getCurrentTrack, patchNowPlaying, updatePlaying, updateTime, isSeekDragging } from "@state/now-playing.svelte";
import { setPlayError } from "@state/player.svelte";
import { consumeEndOfTrack } from "@state/sleep-timer.svelte";
import { vynl } from "@lib/vynl";
import { friendlyError } from "@lib/errors";
import { catalogueIdFromPath as catalogueIdOf } from "@lib/catalogue";
import { t } from "@lib/i18n";
import type { PlaybackTick, StreamErrorEvent } from "@lib/types";
import { updateMediaSession } from "./playback-media";
import { seekState } from "./playback-seek";
import { schedulePrebuffers } from "./playback-prebuffer";
import { playbackPhase, finishPlayReconcile } from "./playback-reconcile";
import { retryState, MAX_CONSECUTIVE_PLAY_FAILURES, scheduleRetry } from "./playback-retry";
import { IS_MINI, ps, PREBUFFER_CHECK_MS } from "./playback-state";
type TickDeps = { nextAutomatic: () => void; maybeStartCrossfade: (position: number) => boolean };
let deps: TickDeps = { nextAutomatic: () => {}, maybeStartCrossfade: () => false };
export function initTick(d: TickDeps): void { deps = d; }
export function handleStreamError(event: StreamErrorEvent): void {
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

export function handleTick(tick: PlaybackTick): void {
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
      deps.nextAutomatic();
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
    deps.maybeStartCrossfade(tick.position);
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

export async function reconcilePlaybackState(): Promise<void> {
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

export function onWindowActive(): void {
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

